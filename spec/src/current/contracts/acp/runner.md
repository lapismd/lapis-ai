# Session and turn lifecycle

Trusted turn configuration accepts mixed typed MCP declarations. The existing
new/load flow preserves HTTP headers and stdio fields without interpreting
either transport. LAI-RUN-001 process ownership, permissions, deadlines and
supervised cleanup are unchanged; the application owns HTTP admission and
endpoint lifetime.

`lapis-acp-runner` executes prepared turns. It does not decide what to run, when to
run it, or what to do with the result; intake and disposition belong to
`lapis-acp-control` and to the controlling application above it.

A session scope is the unit of serialization. One scope has at most one turn in
flight. Distinct scopes may run concurrently when more than one agent subprocess
exists. This mirrors the upstream per-channel guarantee without inheriting its
channel concept: the controlling application names the scope, and `lapis-acp`
treats it as an opaque key.

## Requirements

| ID          | Requirement                                                                                                                                                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-RUN-001 | The runner MUST execute one prepared turn per session scope with at most one turn in flight per scope.                                                                                                                                                                                                     |
| LAI-RUN-002 | Cancellation MUST be cooperative and bounded: request cancellation, wait a bounded grace period, then kill the process group and reap within a bounded interval.                                                                                                                                           |
| LAI-RUN-003 | An idle timeout reset by agent output and an absolute wall-clock turn deadline MUST both apply, and the shorter remaining bound MUST win.                                                                                                                                                                  |
| LAI-RUN-004 | A timeout MUST be reported as an unknown outcome and MUST NOT be recorded as success or as a proven failure.                                                                                                                                                                                               |
| LAI-RUN-005 | A crashed agent MUST be reported as a distinct outcome from a bound that elapsed, and a restart MUST NOT silently re-dispatch a turn that was already sent.                                                                                                                                                |
| LAI-RUN-006 | Session creation MUST apply a configured model, reasoning effort and execution mode where the adapter advertises them, MUST resolve each against the current session rather than a cached one, and MUST retain the adapter default rather than fail the turn when a setting is unsupported or unavailable. |
| LAI-RUN-007 | On Unix, cleanup MUST signal the whole process group before reaping its leader so ordinary descendants cannot outlive the turn, and MUST report whether the stop was confirmed.                                                                                                                            |
| LAI-RUN-008 | Interactive sessions MUST negotiate once, apply configuration before prompting, load only advertised sessions, preserve cancellation and absolute deadlines during permission waits, and close their process ownership within bounded intervals.                                                           |
| LAI-RUN-009 | Warm workers MUST negotiate once, preserve distinct session configuration, invalidate all sessions after uncertain execution, never replay work, and bound both active and lifetime session counts.                                                                                                        |

## Deadline composition

Two independent bounds apply to a turn. The idle timeout resets on any agent
stdout activity, so a long-running but visibly progressing turn is not killed.
The absolute turn duration never resets, so a turn that emits output forever
still ends. A task document may lower the absolute bound; it can never raise it
above the launch configuration cap, because the document is untrusted input and
the cap is trusted policy.

Deadline accounting covers adapter startup, session setup and the turn itself.
There is no separate unbounded startup phase. A launch configuration read has
its own short bound and reports a distinct outcome, because a blocked
configuration read must not be indistinguishable from a blocked agent.

## Cancellation and cleanup

Cancellation is a sequence, not an event. The runner requests cooperative
cancellation through the protocol, waits a bounded grace period for the agent to
return a stop reason, then kills the process group and waits a further bounded
interval to reap it. Each stage reports what actually happened.

Cleanup is best-effort and this is stated rather than hidden. A harness killed
with SIGKILL, or a machine that loses power, cannot run cleanup at all. A
descendant that deliberately escapes its process group is outside the guarantee.
An unreaped child is reported, not assumed stopped.

Descendant containment is a Unix property. On other platforms cleanup addresses
the direct child only, and the reduced guarantee is reported rather than
silently assumed to be equivalent.

## Recovery

LAI-RUN-009 distinguishes a valid RPC rejection of session preparation from a
failed connection. Rejected open/configure reports a classified error without
retiring unrelated warm scopes. Uncertain prompt execution still invalidates the
worker, with no replay or automatic fallback.

A turn that ends without a stop reason is an unknown outcome. The runner records
what it observed and stops. It does not retry, because a retry can execute the
same side effects twice, and it does not fabricate a terminal state to make the
record tidier. Recovery from an unknown outcome is the controlling application's
decision, made with its own durable state.

## Implemented surface

| Module    | Owns                                                                               |
| --------- | ---------------------------------------------------------------------------------- |
| `launch`  | `LaunchSpec`, spawning with piped stdio, inherited stderr and a Unix process group |
| `process` | `ProcessGuard`, bounded grace, process-group kill, bounded reap, `Termination`     |
| `turn`    | `Agent`, `TurnRequest`, and `run_turn` for one negotiated, prompted, stopped turn  |
| `outcome` | `TurnOutcome`, `TurnStatus`, `Failure` and the protocol error classification       |

The child's stderr is inherited rather than piped, so agent diagnostics reach
the operator directly and can never be mistaken for protocol frames on stdout.

`Failure` carries a classified `kind` and an untrusted `detail`. Only the kind
and the agent's numeric error code may be published; the detail is diagnostic
text for stderr and can quote prompt content or a credential the agent was
given. The split is structural, so sanitization cannot be forgotten by a later
caller.

A timeout is classified by which bound elapsed, because the two mean different
things. An idle timeout says the agent went silent. An absolute timeout says the
cap was reached, possibly while the agent was still visibly working. Both are
unknown outcomes, and neither is retried.

## The descendant hazard

Killing the agent's direct child does not stop a child the agent spawned. The
fixture peer's descendant mode proves this rather than asserting it: after the
direct child is killed the recorded descendant pid is still alive. That is why
`ProcessGuard` kills the process group on Unix, and why `Termination` reports
whether the stop was confirmed instead of assuming it.

`Termination::confirmed_stopped` is false when the reap bound elapsed first. A
caller that needs certainty must treat that as an unknown outcome and report it,
which is what the control layer does through its diagnostics rather than through
the terminal record.

## Orderly shutdown before a kill

Termination is the fallback, not the first move. When a turn ends the agent's
stdin is released before the grace window starts, because closing a pipe write
end is what produces the end of input a well-behaved agent waits for. Flushing
is not sufficient: an async shutdown on a child's stdin leaves the descriptor
open, so an agent blocked reading it never learns the exchange is over.

The observable difference is that a completed turn now stops its agent without a
kill. That matters for a real adapter that flushes session state on exit, and it
removes a misleading diagnostic from the normal path: a group kill is reported
because it is worth knowing about, and reporting it after every successful task
would train an operator to ignore it.

Releasing the connection also means the process guard is the only remaining
owner, so the cleanup sequence cannot race against a further protocol write.

## Signal ordering and why it is not interchangeable

A process group id is only unambiguously ours while the group leader is
unreaped, because a pid cannot be recycled while its process is still a zombie.
Both group signals are therefore sent before, or in the absence of, any
successful reap: a cooperative `SIGTERM` to the group first, then a bounded
wait, then `SIGKILL` to the group only when the leader is still outstanding.
Reaping first and signalling after would risk killing an unrelated group whose
leader happens to have taken the recycled pid.

The cooperative group signal is what makes containment hold on the graceful path
too. Closing an agent's stdin lets it exit on its own, but a child it spawned
has no reason to notice, and an agent that exits voluntarily leaves that child
running. Signalling the group before the grace window means a descendant is
stopped whether the agent cooperated or had to be killed.

The residual case is stated rather than hidden: a descendant that ignores
`SIGTERM`, whose leader then exits voluntarily inside the grace window, is not
reached, because at that point the leader is reaped and the group id is no
longer safely ours. Containment is guaranteed for the fatal path and best-effort
for the cooperative one. `Termination` reports which happened through
`group_signalled`, `group_killed` and `escalated`, so a caller can tell them
apart instead of assuming.

A side benefit is speed. Agents that previously sat out the whole grace window
after a completed turn now stop on the cooperative signal, which cut the task
contract suite's wall clock by roughly a factor of six without weakening any
assertion.

## Crash is not a timeout

An agent that exits mid-turn and an agent that stops answering are different
facts with different consequences, and they get different records. A crash is
`failed` with `agent_exited` and exit `1`: the agent is gone, so its state is
knowable and a caller may decide to start a new one. A bound that elapsed is
`timed_out` with exit `124`: the agent may still be running and still producing
side effects, so its state is unknown.

Neither is retried. The distinction exists so a controlling application can make
that decision itself with its own durable state, not so this harness can make it
automatically. Re-dispatching a turn whose outcome is unknown can execute its
side effects twice, which is why the crash tests assert the dispatched prompt
count is exactly one rather than merely asserting the record.

A crash during startup is reported the same way and is distinguishable by what
never happened: no `session/new` and no `session/prompt` reached the agent, so
nothing was dispatched and nothing can have partially executed.

## Session configuration

Three settings are applied after `session/new` and before the turn: model,
reasoning effort and execution mode. Each is resolved against the current
session's advertisement, never against a cached catalogue or a per-adapter
table, because the surface a given adapter exposes varies — including between
two sessions of the same adapter version.

| Module          | Owns                                                                        |
| --------------- | --------------------------------------------------------------------------- |
| `configuration` | `SessionConfiguration`, `SettingOutcome`, `ConfigurationReport` and `apply` |

Order matters. The model is selected first, because an adapter rebuilds its
option catalogue for the new model and may expose a different reasoning-effort
identifier afterwards; the capabilities used for effort and mode resolution are
taken from the switch response when it carries any, and only fall back to the
original session result. Applying effort before the model would resolve it
against a catalogue the switch is about to invalidate.

## Configuration error classification

One distinction governs all three settings, and it is the reason a preference is
safe to request at all.

A **transport-class** failure — I/O, write timeout, request timeout, protocol
violation, agent exit — aborts and propagates, because the stream may be corrupt
and continuing to speak over it would produce nonsense that looks like a result.
An **application-level** rejection, meaning the adapter answered with a JSON-RPC
error, is recorded and the session continues on the adapter's own default.

The distinction is what keeps a preference from being able to fail a task.
Asking for a model an adapter does not have is a question, not a fault, and
treating it as one would punish a caller for asking.

Outcomes are classified rather than collapsed, because they send an operator
looking in different places: `unsupported` means the adapter has no such
control, `unavailable` means it has one that does not offer this value,
`rejected` means it was asked and refused, and `not-this-setting` means the
value exists under a different category so applying it here would have been a
guess.

## Effort has three encodings in the wild

Reasoning effort is not one mechanism. Across two adapters and three observed
sessions it appeared as:

1. A separate configuration option in the `thought_level` category, with an
   adapter-defined identifier that must be discovered.
2. An attribute inside a parameterized model identifier, under any of `effort`,
   `reasoning_effort` or `variant`, where applying it means selecting a
   different model.
3. A bare positional bracket token such as `[medium]`, which is **not** read as
   an effort.

The third is refused deliberately. The token usually is a reasoning level, so a
closed vocabulary would make the inference mostly right — and mostly right is
the worst possible property here, because the failure mode is silently selecting
a model the operator never asked for. Matching only what an adapter explicitly
keyed keeps the behaviour predictable, and the shortfall is reported so the
operator learns the adapter has no settable effort rather than wondering why
nothing changed.

Control V3 has an explicit compatibility profile for the identified
`@agentclientprotocol/codex-acp` adapter. When no keyed/independent effort
surface exists, it may select an exact advertised compact variant such as
`model[medium]`, within the caller's explicitly selected, advertised model
family. Only known effort tokens are interpreted. Generic parsing remains
opaque; other adapters and unknown suffixes retain unsupported outcomes. Both
model and effort are confirmed only when the adapter accepts that exact variant.
This private profile changes no wire or public Rust signatures (LAI-RUN-006/009).

Matching is exact first and normalized second. Exact match is what keeps
adapter-specific levels selectable when they exist in no portable vocabulary;
normalization is what lets a caller asking for `off` reach an adapter that
advertises `none`. The value sent is always the adapter's own spelling.

## Interactive session integration

Interactive open and load retain their agent connection. Follow-up prompts reuse
the same session and serialized process; close, protocol loss and timeout
release process ownership. Configuration and permission handling use bounded
application callbacks rather than the accumulating one-shot recording handler.

Queued prompt cancellation in explicit control v2 reports sessionUsable while
retaining the warm session. The next explicit follow-up reuses it; connection
shutdown still closes it. Version-1 command and terminal record shapes remain
unchanged. E-M11-003 records the binary regression and source gates.

Warm worker initialization is independent of session creation. Serial sessions
share one initialized connection but retain separate negotiated mode state.
Cancellation, absolute deadlines and bounded process-group cleanup apply to
opening and every turn. Unknown outcomes invalidate every session; no implicit
restart or prompt replay occurs. Multi-session fixture qualification is separate
from qualification of each real installed adapter.

Interactive PromptInput retains authored bytes separately from caller-prepared
context. One turn uses ordered context and text blocks, with embedded-resource
selection supplied from the negotiated session. Existing text-only prompt
callers, deadlines, cancellation, quiescence and unusable-session outcomes are
unchanged.

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
