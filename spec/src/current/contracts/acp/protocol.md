# ACP wire protocol

`lapis-acp-protocol` owns the Agent Client Protocol surface that `lapis-acp` speaks to
an agent subprocess over stdin and stdout. It is a hand-rolled JSON-RPC
implementation, matching upstream: there is no ACP crate dependency to vendor,
and no network transport of any kind.

Framing is newline-delimited JSON with an explicit per-frame byte cap. A frame
that exceeds the cap is a protocol error, never a silent truncation and never an
unbounded allocation. Bounded reads are what allow the harness to survive a
malfunctioning agent that emits an endless line.

## Requirements

| ID            | Requirement                                                                                                                                                                                       |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-PROTO-001 | The protocol crate MUST encode and decode ACP JSON-RPC frames over newline-delimited stdio with an explicit per-frame byte cap and MUST fail closed on overflow.                                  |
| LAI-PROTO-002 | The client MUST request protocol version 2 at `initialize` and MUST fall back to version 1 when the agent reports it, recording the negotiated version.                                           |
| LAI-PROTO-003 | The client MUST support `session/new`, `session/prompt`, `session/cancel`, `session/set_model`, `session/set_config_option`, inbound `session/request_permission` and `session/update`.           |
| LAI-PROTO-004 | A turn MUST terminate in exactly one stop reason drawn from `end_turn`, `max_tokens`, `max_turn_requests`, `refusal` and `cancelled`.                                                             |
| LAI-PROTO-005 | An agent error MUST be surfaced as a typed protocol error preserving the agent's numeric code, and MUST substitute the whole error object when no message is present so provider detail survives. |
| LAI-PROTO-006 | The protocol crate MUST remain free of process spawning, scheduling, credential handling and transport policy.                                                                                    |
| LAI-PROTO-007 | Session loading MUST use the advertised loadSession capability and the caller-pinned working directory; permission waits MUST remain bounded by their request and turn lifetimes.                 |
| LAI-PROTO-008 | Inbound session updates and permission requests MUST expose their adapter session identity to caller validation before delivery; default handlers MUST preserve existing single-session behavior. |

## Method direction

| Method                       | Direction    | Role                                                      |
| ---------------------------- | ------------ | --------------------------------------------------------- |
| `initialize`                 | client→agent | Negotiate protocol version and read agent capabilities    |
| `session/new`                | client→agent | Open a session with working directory and MCP server list |
| `session/prompt`             | client→agent | Submit one prepared turn                                  |
| `session/cancel`             | client→agent | Request cooperative cancellation of an in-flight turn     |
| `session/set_model`          | client→agent | Select a model when the agent advertises the capability   |
| `session/set_config_option`  | client→agent | Select a configuration option such as a reasoning effort  |
| `session/update`             | agent→client | Stream turn progress, tool calls and content chunks       |
| `session/request_permission` | agent→client | Ask the controlling side to approve or refuse an action   |

Capability negotiation is advisory in one direction only. When an agent does not
advertise a capability, a request for it is rejected locally with a reason
naming the absent capability rather than being sent and timing out. When an
agent advertises a capability the harness does not model, the harness ignores it
and records that the session is narrower than the agent.

## Port relationship

This chapter governs the ported, decomposed derivative of the upstream ACP
client. Upstream concentrates framing, correlation, session setup, model and
permission handling and legacy first-turn framing in one 215 KB file. Here those
concerns are twelve modules, the largest of which is 228 counted lines, and
`docs/upstream-lock.md` records which upstream region each came from so a
re-port can be compared region by region instead of file by file.

## Implemented surface

| Module          | Owns                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------ |
| `frame`         | Bounded newline-delimited reader and writer, skip counting                                 |
| `envelope`      | Request, notification and response construction, inbound classification, id matching       |
| `error`         | Typed wire failures and agent error extraction                                             |
| `budget`        | Connection limits and the two-bound turn budget                                            |
| `initialize`    | Client identity, advertised client capabilities, version negotiation                       |
| `content`       | Text content blocks and MCP server declarations                                            |
| `session`       | `session/new`, `session/prompt` and `session/cancel` payloads and stop reason parsing      |
| `config_option` | Model and reasoning-effort discovery and selection                                         |
| `update`        | `session/update` classification with raw pass-through                                      |
| `permission`    | `session/request_permission` normalization and response construction                       |
| `handler`       | The inbound-traffic hook and a recording default                                           |
| `client`        | The connection: correlation, budgets and servicing agent traffic while awaiting a response |

`Connection` is generic over a reader and a writer rather than owning a child
process, which is what allows the whole surface to be tested over an in-memory
pipe. Process ownership, respawn and cleanup belong to
[session and turn lifecycle](runner.md).

A turn budget carries an idle bound that resets on every frame and an absolute
bound that never resets; the shorter remaining bound always wins. Both are
clamped to a nonzero floor so a zero value cannot produce a deadline that has
already passed and a confusing immediate timeout.

## Deliberate divergences

Three behaviours differ from upstream on purpose, and each is a boundary this
repository holds rather than an oversight in the port.

Approval is not automatic. Upstream answered every `session/request_permission`
by selecting the option whose kind is `allow_once`, falling back to
`reject_once`, because it ran unattended in an auto mode. Here the default
answer is cancellation and a caller that wants unattended approval must choose
it explicitly, select by semantic kind rather than by identifier, and record
that it did so. An unanswered approval request must never silently become an
approval.

Client capabilities are advertised as absent. This crate implements no
`fs/read_text_file`, `fs/write_text_file` or `terminal/*` method, so it
advertises all three as false. Upstream also advertised terminal-native
authentication and vendor-specific notification extensions; both are omitted
because this harness holds no credentials and publishes no telemetry, and
advertising a facility that does not exist would only produce requests that
fail.

Skipped frames are counted rather than only logged. A blank line is a frame
separator and is not counted; an unparseable line is counted and exposed through
`skipped_frames`, so a caller can distinguish an agent that emits stray
diagnostics from one that emits nothing but noise. Skipping is tolerance, not
acceptance: a stream of only garbage yields no frames and eventually trips the
idle bound.

## Budget-scoped negotiation

`initialize` and `session/new` each have a `_within` variant taking an explicit
turn budget. A caller executing one prepared turn passes the turn's own budget,
so adapter startup and session setup share the idle and absolute bounds with the
turn instead of getting a separate unbounded phase. A hung adapter is then
reported as the bound that actually elapsed: an adapter that emits nothing trips
the idle window, and one that streams without answering trips the absolute cap.

The default variants keep a fixed request limit for callers that have no turn
budget yet. This distinction was not free: until it existed, a hanging adapter
consumed a fixed 60 second request timeout and reported a deadline rather than
silence, which is a different diagnosis and a much slower one.

`send_request` and `await_response` are separate operations so a caller can race
the wait against a cancellation signal without abandoning the request. The agent
still owes an answer for that id, and abandoning it would leave the answer
unread. This is safe because the frame reader is resumable: bytes already
consumed from the underlying reader survive a cancelled read and complete the
same frame on the next one.

## What a real adapter changed

Qualification against `cursor-agent acp` corrected one behaviour and confirmed
two design choices.

The correction: `advertises` tested for presence, and adapters send an explicit
`false` for facilities they do not support. `"audio": false` therefore read as
advertised, which would have sent audio content to an agent that had already
said it cannot take it. A boolean leaf is now honoured as a boolean; a
non-boolean, non-null leaf is an object or marker whose presence is the
advertisement, which is how `loadSession: true` and
`mcpCapabilities: { http: true }` continue to read.

The first confirmation is version negotiation. The client asks for protocol
version 2 and this adapter answers 1. Supporting both is not defensive
over-engineering; it is the only reason the exchange worked at all.

The second is the dual option identifier. The ACP schema says `configId` and
this adapter sends `id`, exactly as upstream recorded for a different adapter.
Accepting both is what makes its 43 model candidates selectable rather than
invisible.

The captured `initialize` and `session/new` payloads are committed as a test
fixture, so these assertions are re-checked against real adapter output on every
run rather than against a synthetic payload that only proves the parser agrees
with itself.

## Two adapters, two shapes

Both captured payloads are committed as fixtures, and they disagree in ways that
matter. Neither is a superset of the other, so parsing has to be driven by what
a session actually advertises rather than by an expected shape.

| Surface                     | `cursor-agent acp`         | `codex-acp` 1.13.1                                                      |
| --------------------------- | -------------------------- | ----------------------------------------------------------------------- |
| Negotiated protocol version | 1                          | 1                                                                       |
| `agentInfo`                 | absent                     | name, title and version                                                 |
| Option identifier key       | `id`                       | `id`                                                                    |
| Model values                | parameterized, bracketed   | bare names                                                              |
| `thought_level` option      | absent                     | present, as `reasoning_effort`                                          |
| Session modes               | `agent`, `plan`, `ask`     | `read-only`, `agent` and others                                         |
| MCP transports              | http and sse               | http only                                                               |
| Extra capabilities          | `sessionCapabilities.list` | `auth.logout`, `providers`, `_meta.authStatus`, resume, fork, subagents |

Three consequences are load-bearing.

Capability flags must be read as booleans, not as presence, because both
adapters send explicit `false` for facilities they do not support and one of
them sends it for an MCP transport. Option identifiers must be discovered per
session, because `reasoning_effort` is a name no parser would guess and Cursor
has no equivalent option at all. Model values must be matched exactly against
the advertised candidates, because a bare name is valid for one adapter and
meaningless for the other.

Both adapters also emitted a notification that is not part of the exchange being
waited for — Codex sends `_auth/status_update` during the handshake. A client
that read only responses would stall here, which is why unmodelled notifications
are routed to the handler and skipped rather than treated as a protocol
violation.

## Normalized matching without a closed vocabulary

The `thinking` module normalizes reasoning effort for _matching_ only. It
defines a portable level set — `off`, `low`, `medium`, `high`, `xhigh` — plus
the attribute keys that carry a level in real adapter output: `effort`,
`reasoning_effort`, `variant`, and `thinking` as a weaker marker that reasoning
happens without naming how much.

It is deliberately not an enum that constrains what can be selected. A real
adapter advertises `max` and `ultra`, which no portable vocabulary names, so a
closed type would make them unreachable. `effort_matches` therefore tries an
exact comparison first and falls back to normalization, and the value sent back
is always the adapter's own spelling.

Parameterized model identifiers are parsed rather than string-matched:
`model_id_base` separates the name from its parameters, `model_id_attrs` reads
the key-value pairs, and `advertised_effort` applies a fixed key precedence so
`model[thinking=true,effort=high]` reports the level regardless of the order the
adapter emitted the attributes. A malformed or keyless bracket yields nothing
rather than a guess.

## Interactive session integration

Interactive loading calls session/load only after loadSession was advertised.
Loading never prompts. Permission callback waits remain inside the overall
request bound, including when no new frame arrives.

Warm worker handlers validate sessionId before inbound session updates or
permission requests are delivered. Single-session handlers default to accepting
the existing traffic. A scope mismatch is a protocol failure, never permission
approval or activity reassignment.

Version 3 ignores delayed updates from exact known inactive adapter sessions and
automatically cancels their permission requests. Unknown or mismatched active
identities still invalidate the worker; inactive traffic never enters the
current application transcript. Only fixed failure classifications and numeric
agent codes are exposed, never untrusted diagnostic details.

Prepared prompt context is an ordered content block independent of authored
text. Embedded resource support is selected from negotiated capabilities by the
caller. The protocol helper emits an inline text resource or separate text
block, then the unchanged authored text. It performs no URI or file access.

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

The GitHub ownership transfer updates package documentation links to
`lapismd/lapis-ai`; the owning ACP requirements and runtime behavior are unchanged.
