# Controlling application boundary

`lapis-acp` has no message bus, no relay, no subscription and no author gate. Work
arrives from a controlling application that spawned the process, over stdin, and
results leave over stdout. There is no listener, no port, no token and no
authentication, because there is no peer: the parent process is the trust
boundary and it already decided to run this executable with this environment.

This chapter replaces the upstream relay intake entirely. The upstream author
gate, membership discovery, owner resolution and workflow-provenance checking
have no analogue here and are not partially reimplemented.

## Requirements

| ID           | Requirement                                                                                                                                                                                                                                                |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-CTRL-001 | The controlling application MUST drive `lapis-acp` over a versioned newline-delimited JSON control protocol with no authentication and no network listener.                                                                                                |
| LAI-CTRL-002 | `lapis-acp` MUST accept one prepared task document of at most 1 MiB from a file or stdin and MUST reject unknown fields, duplicate fields, wrong types, trailing documents and malformed JSON.                                                             |
| LAI-CTRL-003 | Every attempted invocation MUST emit exactly one terminal record on stdout and MUST NOT include the prompt, credentials, agent final text or raw adapter errors.                                                                                           |
| LAI-CTRL-004 | Diagnostics and child stderr MUST stay on stderr so that stdout remains unconditionally parseable by the controlling application.                                                                                                                          |
| LAI-CTRL-005 | The task document MUST NOT change the working directory, executable, tools or credentials, which come only from trusted launch configuration.                                                                                                              |
| LAI-CTRL-006 | A control frame MUST carry an explicit integer version, and an unsupported version MUST be rejected before any agent subprocess is started.                                                                                                                |
| LAI-CTRL-007 | Reading a task document from a file or stdin MUST have its own bounded interval, separate from the execution deadline.                                                                                                                                     |
| LAI-CTRL-008 | Interactive control version 2 MUST retain one caller-configured ACP session across ordered turns, stream correlated bounded activity and permission requests, and accept only current permission decisions without changing the version-1 record contract. |
| LAI-CTRL-009 | Explicit control version 3 MUST initialize a worker without opening or prompting, serialize bounded caller-scoped sessions under its launch directory, and fence cancellation, activity and permissions by exact session identity.                         |

## Task document, version 1

One UTF-8 JSON object, at most 1 MiB including whitespace, supplied as a path or
as `-` for stdin. Stdin MUST be closed after the document.

```json
{
  "version": 1,
  "taskId": "example-001",
  "agent": "codex",
  "prompt": "Inspect the working copy and report findings.",
  "maxDurationMs": 120000
}
```

| Field           | Contract                                                                                                                 |
| --------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `version`       | Integer `1`. Any other version fails before agent launch.                                                                |
| `taskId`        | Nonblank string, 1 to 256 UTF-8 bytes, no control characters. Correlation only; not a deduplication key and not a claim. |
| `agent`         | Runner identifier that MUST resolve to an enabled catalog entry. This is a mis-execution guard, not an authorship check. |
| `prompt`        | Nonblank prepared task text. Context, evidence and reporting instructions belong here.                                   |
| `maxDurationMs` | Positive integer. The shorter of this value and the trusted absolute cap wins.                                           |

All fields are required. The document is executable task input rather than a
sandboxed message, so only supply tasks trusted with that runner's capabilities.

## Terminal record

One JSON line per attempted invocation. `taskId` is null until the document
validates. `status` is `completed`, `failed`, `invalid`, `timed_out` or
`cancelled`. `stopReason` appears only on normal completion. Failures carry a
stable sanitized `error` code such as `invalid_task_json`, `agent_spawn_failed`,
`runner_mismatch` or `configuration_timeout`.

```json
{
  "version": 1,
  "taskId": "example-001",
  "status": "completed",
  "stopReason": "end_turn"
}
```

A record is not a durable ledger. A killed harness or a broken stdout can leave
no record at all, and launching the same task twice can execute it twice.
Durability belongs to the controlling application.

## Transport pluggability

The control transport is a trait with one implementation in the first delivery:
newline-delimited JSON over the inherited stdin and stdout. A controlling
application that wants a Unix socket, a local HTTP endpoint or an in-process
link adds a transport rather than editing the runner. The trait boundary is the
reason the one-shot task runner and a future persistent service can share the
same session and turn machinery.

## Implemented surface

| Module       | Owns                                                                                  |
| ------------ | ------------------------------------------------------------------------------------- |
| `task`       | Document version 1, strict validation, the stable rejection code vocabulary           |
| `input`      | Bounded reads from a path or stdin, capped while reading rather than after            |
| `record`     | `TerminalRecord`, `Status`, `ExitCode` and the record-level outcome codes             |
| `invocation` | Resolving a runner, launching, executing one turn, stopping, and reducing to a record |
| `transport`  | The `ControlTransport` trait and the stdio implementation                             |

Rejection happens before any agent is spawned. A document that names an unknown
runner, a version this build does not accept, or a runner other than the one
this process was launched as never reaches a launch attempt, and the fixture log
stays empty for every refusal case.

## Reserved namespace

Only the task identity the harness injects is reserved, under the `JJ_ACP_TASK_`
prefix. An earlier design reserved the whole `JJ_ACP_` tree and the fixture peer
caught the consequence: it also stripped the launch options an operator sets on
purpose, such as `JJ_ACP_RUNNER` and `JJ_ACP_MODEL`, breaking configuration in
the name of protecting identity. Reserving narrowly is still sufficient, because
the only thing a runner definition must not be able to forge is which task the
harness says it is running.

## Exit code resolution

The invocation reports the exit code for every outcome except a signal-driven
cancellation, where it sets a flag instead. The CLI owns signal handling and
therefore owns the choice between `130` and `143`, because only it observed
which signal arrived. Keeping that decision out of the control crate is what
lets a controlling application drive the same invocation over a different
transport and apply its own interruption semantics.

## Configuration shortfalls are diagnostics, not record fields

A setting that did not land is reported on stderr with its classified reason
and, for an adapter rejection, the numeric error code. It is never added to the
terminal record. Two reasons: the record schema is fixed so a controlling
application can parse it unconditionally, and an adapter's own rejection text is
untrusted and can quote prompt content, so only the classification and the code
are safe to publish.

The turn outcome still carries the full configuration report, so a caller
embedding the crate can act on each setting individually. The one-shot CLI
reduces it to diagnostics because its contract is one record and one exit code.

## The persistent control protocol

`lapis-acp serve` reads newline-delimited JSON frames on stdin and writes frames on
stdout. Version 1 carries four requests and six responses.

| Request    | Fields                | Answered by                                  |
| ---------- | --------------------- | -------------------------------------------- |
| `submit`   | `requestId`, `task`   | one `record` frame, plus `diagnostic` frames |
| `cancel`   | `requestId`, `target` | `ack`, or `error` with `not_in_flight`       |
| `runners`  | `requestId`           | `runners`                                    |
| `shutdown` | `requestId`           | `ack`, then the loop drains and exits        |

Every frame carries `version`, and the harness opens each session with a `ready`
frame carrying the resolved runner list, so a controlling application learns
what it can ask for before asking. An unsupported version is refused before any
agent is started, which is what makes the version field meaningful rather than
decorative.

A refused task document is delivered as a `record` frame with `status: invalid`
rather than as an `error` frame. One shape for every task outcome means a
controlling application handles success, failure and refusal through the same
code path, and cannot miss a refusal because it was only listening for records.
Frame-level problems — bad JSON, a missing version, an unknown type — are
`error` frames, because there is no task to report against.

Diagnostics travel as `diagnostic` frames rather than stderr. The one-shot path
writes them to stderr because a human is reading it; the persistent path has a
machine on the other end, and a machine that has to watch two streams to know
what happened will eventually miss one. Stdout still carries nothing but frames.

## Concurrency, scopes and the queue

Submissions carry a caller-named `scope`. A scope has at most one turn in
flight; distinct scopes run concurrently up to the worker limit; a bounded queue
absorbs bursts and refuses admission when full rather than growing without
limit.

The scope is opaque. Upstream scoped by channel and thread because a relay has
those concepts. Here the controlling application decides what a scope means and
this harness only guarantees non-overlap within one, which is why the default
scope exists: a peer that never names one gets the serial behaviour it had
before scopes were introduced.

Queue selection is first-fit rather than strictly FIFO, because a submission
whose scope is busy must not block an unrelated scope behind it. Order within a
scope is preserved, which is the guarantee that actually matters.

A submission queued behind its own scope is not refused. Serializing a scope
means the second turn waits, not that it is rejected, and admission is
acknowledged with a `queued` frame so a peer can tell a waiting submission from
a dropped one.

| Request    | Fields                        | Answered by                                   |
| ---------- | ----------------------------- | --------------------------------------------- |
| `submit`   | `requestId`, `scope?`, `task` | `queued`, then one `record`, plus diagnostics |
| `cancel`   | `requestId`, `target`         | `ack`, or `error` with `not_in_flight`        |
| `runners`  | `requestId`                   | `runners`                                     |
| `status`   | `requestId`                   | `status`                                      |
| `shutdown` | `requestId`                   | `ack`, then drain and exit                    |

`status` reports the configured worker count and queue bound alongside the live
queue depth and the scopes with a turn in flight, so a peer can distinguish
"your work is waiting" from "this harness cannot take more work".

## Ordering between a record and the state it implies

A finished submission routes its frames back through the loop rather than
emitting them from the task that produced them. The loop releases the scope
first and writes the record second.

Emitting directly from the task looked correct and was not: a peer could observe
a record and then ask for `status` that still listed that submission as running,
because the loop had not yet processed the completion. That is a contradiction a
controlling application has no way to resolve, and it is invisible in a test
that does not ask the question.

## The transport seam

The loop depends on two traits rather than on stdio: a `FrameSource` it owns and
a `FrameSink` it shares. They are split because a persistent server has one
reader and many writers, and modelling that as one bidirectional object would
force both sides to share a lock only the writers need.

An earlier shape of this trait modelled the one-shot path — read a task
document, emit a record — and nothing used it, because the persistent protocol
exchanges requests and responses rather than documents. It was replaced with the
seam the loop actually needs, and the scheduler is now tested in process over an
in-memory pair as well as through the spawned binary. A seam that no caller
passes through is a claim rather than an abstraction.

Sources return raw lines and sinks write whole lines atomically. A transport
that dropped or rewrote a frame it disliked would hide the very thing a peer
needs to hear about, and an interleaved frame is indistinguishable from a
corrupt one.

## Crashes and slots

A worker slot is held by a permit for the whole turn, so releasing it on drop is
what makes a crashed agent give its slot back. The crash is reported as
`agent_exited` and the submission is never re-dispatched: a turn whose outcome
is unknown may already have produced side effects, and running it again is the
controlling application's decision to make with its own durable state.

Shutdown cancels running submissions and waits for slots to return, bounded by
the same cleanup intervals a single invocation uses. When the bound elapses
first that is reported as a diagnostic rather than concealed, because a peer
told the shutdown was clean has no way to detect an orphan.

## Interactive control version 2

One trusted launch owns one session, runner and working directory. Frames carry
version 2 and requestId. Requests are open (optional loadSessionId), configure
(model, effort, mode), prompt (text, optional context, maxDurationMs), cancel
(target), permission (permissionId, optionId or null), status and close. Unknown
fields and oversized frames are refused. No request accepts a path, executable
or environment.

Responses are ready, queued, opened, configured, activity, permission, record,
status, ack, error and closed. Every activity has an increasing sequence and
request identity; content is separate from terminal records. The parent owns
redaction and durable history. Session and raw adapter capabilities are facts,
not publication authority. Configuration reports expose classified outcomes;
unsupported requested modes prevent prompting until corrected.

Permission options retain the exact adapter identifiers. Decisions are valid
only for that live permission and its advertised options. Timeout, cancellation,
EOF and close cancel pending permissions. Queue and event capacities are
bounded; output backpressure or overflow ends execution with an explicit unknown
outcome. A control connection never retries work. Coverage loss remains latched
after the output queue drains. A racing completed turn cannot report success or
a reusable session after any activity frame was lost.

Canceling an admitted prompt while it is still queued reports cancelled without
discarding a warm session. Its terminal frame explicitly reports sessionUsable;
an individual queued cancellation retains usability, while connection shutdown
does not. The next explicit prompt can reuse the retained session.

## Warm worker control version 3

A trusted launch fixes one runner and a managed directory root. `warm` only
spawns and initializes; it sends neither session/new nor a prompt. `open`
carries an opaque application sessionId and an existing cwd whose canonical path
is inside that root. It optionally loads an advertised adapter session.
Executables, environment and arbitrary paths remain launch policy.

Configure, prompt, cancel and permission require the application sessionId.
Every session response includes that ID; opened additionally includes the
adapterSessionId. Inbound updates and permissions from a different adapter
session fail the worker. Status and close address the whole worker.
close_session releases application participation without stopping other
sessions. There is no provider session-delete claim. At most 64 sessions may be
opened over one worker lifetime, preventing retained provider sessions from
growing without bound. One turn runs at a time; queue capacity and request
identity limits match v2. A valid RPC rejection during open/configure reports
agent_rejected for that request and preserves the warm worker and other scopes.
It does not imply process failure or uncertain execution. Protocol failure,
process exit, timeout or interrupted execution closes the worker and invalidates
all scopes. The application owns generation fencing, durable identities, history
and explicit continuation. No prompt is retried by this protocol.

Version 3 ignores delayed updates from exact known inactive adapter sessions and
automatically cancels their permission requests. Unknown or mismatched active
identities still invalidate the worker; inactive traffic never enters the
current application transcript. Only fixed failure classifications and numeric
agent codes are exposed, never untrusted diagnostic details.

Version-2 and version-3 prompts allow empty authored text only when bounded
prepared context is nonblank, enabling comments-only submissions without dummy
prompt text. Both text and context empty or whitespace remains invalid.
Version-2 and version-3 prompts accept bounded optional context separately from
nonempty authored text. This data cannot alter launch policy. A single
session/prompt sends the context first and authored text last. An advertised
promptCapabilities.embeddedContext selects a text resource with an opaque
context URI and text/plain MIME type; absent or false capability uses a separate
text block. Context is never fetched from the URI. No preliminary turn, retry or
session-configuration side effect is introduced. Existing text-only callers
retain their wire shape and lifecycle.

## Trusted coding-tool declarations

The controlling application may select a bounded JSON file of stdio MCP
declarations through `--tools` or `JJ_ACP_TOOLS`. This is trusted launch
configuration, independent of tasks and v2/v3 control input. Session creation
and supported loading receive the same declarations. Warm initialization does
not start a tool session. The file is bounded to 64 KiB and eight unique stdio
servers; only absolute executable paths are accepted, with bounded
arguments/environment and no reserved harness identity names. Errors disclose
stable codes, not file contents or credentials. ACP continues to own no tool
policy, authentication, network or durable settings.

Cargo publication metadata and coordinated versions follow the
[release contract](../release.md). The ACP wire, catalog, session, task and
permission contracts remain unchanged by release preparation.
