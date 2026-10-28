# CLI and exit codes

Trusted MCP launch files now accept bounded literal-loopback HTTP declarations
alongside stdio. This extends LAI-CLI-004's existing `--tools`/`JJ_ACP_TOOLS`
input, with no new task/control fields. LAI-CLI-002/005 retain stable refusals
and credential-free CLI output. Native endpoint authentication remains host-owned.

The owned-session consumer regression drives the compiled control-v3 binary
through `lapis-ai-runtime`. This adds test-only composition and confirms the
unchanged LAI-CLI-008 boundary; runtime adapters do not enter CLI production code.
Its test-only internal version now inherits the coordinated workspace range.

The `lapis-acp` binary is the only executable in this repository. It composes the
crates and owns nothing else: argument parsing, launch configuration assembly,
signal handling and the process exit code.

The first delivery is the one-shot prepared task runner. A persistent service
mode is specified but not implemented, and the ledger records it as deferred
rather than implying parity.

## Requirements

| ID          | Requirement                                                                                                                                                                                             |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-CLI-001 | The binary MUST expose `run --task <path\|->` executing exactly one prepared task in one fresh agent process and session, then exit.                                                                    |
| LAI-CLI-002 | Exit codes MUST be `0` normal completion, `1` startup, execution or output failure, `2` invalid arguments, launch configuration or task input, `124` deadline exceeded, `130` SIGINT and `143` SIGTERM. |
| LAI-CLI-003 | SIGINT and SIGTERM MUST trigger bounded cleanup before exit and MUST NOT leave an unreaped child when cleanup succeeds.                                                                                 |
| LAI-CLI-004 | Every launch option MUST be settable by a command-line flag and by a matching environment variable, with the flag taking precedence.                                                                    |
| LAI-CLI-005 | The binary MUST NOT write diagnostics, prompts or agent output to stdout on any path, including startup failure and signal handling.                                                                    |
| LAI-CLI-006 | A task run MUST NOT start a subscription, a standing pool, a heartbeat or any other service facility, and inherited service-only options MUST NOT enable one.                                           |
| LAI-CLI-007 | serve --control-version 2 MUST select the interactive protocol explicitly while the default and one-shot commands retain version-1 behavior.                                                            |
| LAI-CLI-008 | serve --control-version 3 MUST explicitly select warm worker control; versions 1 and 2 MUST retain their existing contracts.                                                                            |

## Commands

| Command                     | Role                                                      | Status   |
| --------------------------- | --------------------------------------------------------- | -------- |
| `lapis-acp run --task -`    | Execute one prepared task document read from stdin        | Planned  |
| `lapis-acp run --task PATH` | Execute one prepared task document read from a file       | Planned  |
| `lapis-acp agents`          | List resolved runners with availability and install hints | Planned  |
| `lapis-acp serve`           | Persistent control transport with a standing session pool | Deferred |

Exit code `0` means the turn returned normally. It is not proof that the task
succeeded; the caller MUST inspect `stopReason`. A refusal is a normal
completion with `stopReason` set to `refusal`, which is why the two are separate
signals.

## Working directory and inheritance

The working directory is the launch working directory. A task document cannot
change it, and no option in the document is consulted for it. This keeps the
untrusted input away from the one decision that determines which files an agent
can touch.

The inherited environment is preserved and extended, never replaced. Existing
variables that the harness needs to control are overridden deliberately and the
override is recorded; everything else passes through so a controlling
application can configure provider credentials the way it already does.

## Implemented surface

| Module     | Owns                                                                          |
| ---------- | ----------------------------------------------------------------------------- |
| `args`     | Flag and environment parsing, defaults, usage text, refusals                  |
| `run_task` | Catalog assembly, prompt file read, launch configuration, one invocation      |
| `agents`   | One JSON object per line describing each resolved runner and its availability |
| `signals`  | SIGINT and SIGTERM installation and the cancellation watch                    |
| `lib`      | Dispatch, runtime construction, emitting one record, resolving the exit code  |

Every launch option has a flag and a matching `JJ_ACP_*` variable, and the flag
wins. Defaults for the idle window and the absolute cap are 620 and 7200
seconds, inherited from the upstream harness; the cancellation grace and reap
bounds default to five seconds each.

Installing a signal handler replaces the default terminate-immediately
behaviour, so an interruption now produces bounded cleanup and a record instead
of an orphaned agent and silence. A failure to install is reported rather than
ignored, because running without cancellation handling would silently weaken
that guarantee.

## Stdout contract

Stdout carries terminal records, and for `--help`, `--version` and `agents` the
output those commands exist to produce. Nothing else is ever written there.
Usage for a malformed command line goes to stderr, so a controlling application
that mis-invokes the binary still sees empty stdout rather than prose it cannot
parse.

A record is emitted on every path that reaches an invocation, including a spawn
failure and an agent error. A broken stdout is the one case with no record, and
it exits `1`; the record is not a durable ledger and a killed harness can leave
none.

## Failure classification

The exit code distinguishes causes a controlling application must treat
differently. `1` with `agent_exited` means the agent is gone and a new one can
safely be started. `124` means a bound elapsed and the agent's state is unknown,
so starting another one risks two agents working the same task. `2` means
nothing was started at all, so the same document can be corrected and
resubmitted.

`130` and `143` mean the run was interrupted and bounded cleanup ran. Cleanup
being bounded is not the same as cleanup being complete, and the record says
`cancelled` with `interrupted` rather than claiming the agent stopped.

## Subcommand option handling

`agents` parses the same launch options as `run`, because it inspects a catalog.
Building it from defaults instead reported a smaller runner set than the
operator configured and looked like a correct answer, which is the worst shape
for this class of bug: a catalog file that silently does nothing.

Arguments that no subcommand would read are refused rather than ignored.
`--help --catalog x` is a mistake, and accepting it teaches an operator that
unparsed flags are harmless.

## Session configuration flags

`--model`, `--effort` and `--mode` request session settings, each with a
matching `JJ_ACP_MODEL`, `JJ_ACP_EFFORT` and `JJ_ACP_MODE` variable and the flag
winning. All three are requests, not requirements: a value the adapter does not
offer is reported on stderr and the adapter's own default is kept, and the run
still completes normally with exit `0`.

That asymmetry is intentional. Failing a task because an adapter lacks a
preference would make the flags dangerous to pass, and a controlling application
that targets several adapters could not set a preference at all. The record
schema is unchanged by configuration, so shortfalls travel on stderr where an
operator reads them and never in the record a machine parses.

## The serve command

`lapis-acp serve` parses the same launch options as `run` and refuses `--task`,
because a task arrives per control frame and a launch-time document would be
meaningless. Sharing the option parser is what stops a flag from meaning one
thing in `run` and something else in `serve`; the trusted launch configuration
is built by one function both commands call.

A signal ends the loop rather than cancelling one task. A controlling
application that has stopped reading has nobody to receive a record, so the
honest outcome is a bounded drain followed by the signal's own exit code.
Reaching end of input is a clean exit `0`, because a closed stdin is how a
controlling application says it is finished.

Unusable launch configuration exits `2` with the reason on stderr and nothing on
stdout. There is no loop to serve and no peer to frame a refusal for, so the
exit code is the only machine-readable signal available.

## Pool sizing

`--workers` sets how many submissions `serve` executes at once and
`--queue-capacity` how many may wait, each with a matching `JJ_ACP_WORKERS` and
`JJ_ACP_QUEUE_CAPACITY` variable. Workers default to one, which preserves the
serial behaviour from before scopes existed, so raising concurrency is an
explicit choice rather than a surprise. Both are validated: zero is refused, and
workers are capped at 32.

The one-shot path ignores both, because a process that runs one task and exits
has nothing to size.

## Interactive session integration

The additive --control-version option accepts 1 or 2 and defaults to 1. Version
2 requires an explicit --runner. Signal and owner EOF request bounded
interactive cleanup.

Queued prompt cancellation in explicit control v2 reports sessionUsable while
retaining the warm session. The next explicit follow-up reuses it; connection
shutdown still closes it. Version-1 command and terminal record shapes remain
unchanged. E-M11-003 records the binary regression and source gates.

Warm worker integration explicitly selects --control-version 3 and a fixed
managed cwd root. The flag default remains version 1; version 2 remains a single
interactive session with no request-supplied paths.

A version-3 adapter rejection of open/configure remains request-scoped. Other
opened sessions stay usable on the initialized process. Protocol errors,
timeouts and process exits retain worker-wide invalidation (E-M12-004).

Serve controls v2/v3 accept optional prepared context independently of text.
This adds no launch flag, filesystem selection or credential surface. The CLI
preserves caller session/request fencing and emits one terminal outcome per real
prompt; warming and context preparation do not submit preliminary turns.

Interactive v2/v3 callers may submit a comments-only turn using empty `text` and
nonblank prepared `context`. The CLI retains both fields without inventing user
text; an entirely empty prompt remains invalid (E-M12-003).

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
