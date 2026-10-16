# lapis-acp

[![CI](https://github.com/stevejuma/lapis-acp/actions/workflows/ci.yml/badge.svg)](https://github.com/stevejuma/lapis-acp/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/lapis-acp.svg)](https://crates.io/crates/lapis-acp)

A pluggable [Agent Client Protocol](https://agentclientprotocol.com/) harness
that runs coding agents for a controlling application over stdio JSON-RPC. The
application owns credentials, authorization, prompt policy and durable history.
lapis-acp owns the runner catalog, subprocesses, sessions and turn lifecycle.

## Install

With Rust **1.98.1 or newer**:

```sh
cargo install lapis-acp --version 0.1.0 --locked
lapis-acp --version
lapis-acp --help
```

[GitHub releases](https://github.com/stevejuma/lapis-acp/releases) provide
`lapis-acp-<version>-<target>.tar.gz` archives and `SHA256SUMS` for:

| Target                      | Platform                               |
| --------------------------- | -------------------------------------- |
| `x86_64-unknown-linux-gnu`  | Linux x86-64, Ubuntu 22.04 or newer    |
| `aarch64-unknown-linux-gnu` | Linux ARM64, Ubuntu 22.04 or newer     |
| `x86_64-apple-darwin`       | macOS Intel, macOS 15 or newer         |
| `aarch64-apple-darwin`      | macOS Apple Silicon, macOS 15 or newer |

Extract your platform's archive, verify its checksum, and place `lapis-acp` on
PATH. These are GNU/Linux builds; macOS downloads have no Developer ID
notarization. Node, pnpm and Python are developer/test prerequisites, not
requirements of the installed harness. An adapter can have its own runtime
requirements.

## Choose and authenticate an adapter

Install an ACP-speaking adapter separately and authenticate it using its own
instructions. lapis-acp never installs adapters or authenticates a provider.

```sh
npm install -g @agentclientprotocol/codex-acp
lapis-acp agents
```

Discovery emits one JSON object per runner with availability and installation
hints. Built-in IDs are `codex`, `claude`, `opencode`, `goose` and `cursor`.
Codex launches `codex-acp`; Cursor launches `cursor-agent acp` and uses the
operator's existing `cursor-agent login` session. Availability only proves
executable discovery; the
[qualification ledger](https://stevejuma.github.io/lapis-acp/milestones.html)
records the adapter/version combinations actually exercised.

## Run one prepared task

Save this as `task.json`:

```json
{
  "version": 1,
  "taskId": "inspect-001",
  "agent": "codex",
  "prompt": "Inspect the working copy and report findings.",
  "maxDurationMs": 120000
}
```

```sh
lapis-acp run --task task.json --runner codex --cwd /absolute/approved/workspace
cat task.json | lapis-acp run --task - --runner codex --cwd /absolute/approved/workspace
```

The task cannot change the executable, working directory, credentials or tools.
Its runner must match the trusted `--runner`. Stdin must close after a task
submitted through `--task -`. Stdout emits one sanitized terminal record:

```json
{
  "version": 1,
  "taskId": "inspect-001",
  "status": "completed",
  "stopReason": "end_turn"
}
```

The one-shot record omits prompts, credentials and final agent text. Diagnostics
and child stderr go to stderr. Use interactive control when the application
needs streamed content and permission decisions.

| Exit code | Meaning                                |
| --------- | -------------------------------------- |
| 0         | Completed                              |
| 1         | Failed                                 |
| 2         | Invalid task or launch configuration   |
| 124       | Timed out; outcome unknown             |
| 130 / 143 | SIGINT / SIGTERM after bounded cleanup |

A timeout does not establish success or failure. The harness never retries an
invocation. The application must decide how to handle an unknown outcome.

## Customize trusted launch configuration

Flags override their corresponding `JJ_ACP_*` variables. Useful flags include
`--model`, `--effort`, `--mode`, `--idle-timeout`, `--max-turn-duration`,
`--cancel-grace`, `--reap-timeout`, `--system-prompt-file`, `--workers` and
`--queue-capacity`; `lapis-acp --help` lists their environment names.
Adapter-specific model, effort and mode identifiers come from negotiation.
Interactive prompting requires a successfully applied execution mode.

A catalog adds runners without changing the harness. Save as `runners.json`:

```json
{
  "runners": [
    {
      "id": "my-agent",
      "label": "My agent",
      "command": "/absolute/path/to/my-acp-agent",
      "args": [],
      "env": {}
    }
  ]
}
```

```sh
lapis-acp agents --catalog runners.json
lapis-acp run --task task.json --catalog runners.json --runner my-agent --cwd /approved/workspace
```

For the custom-runner task, change `task.json`'s `agent` to `my-agent`. Catalog
IDs cannot shadow built-ins. Launch environments merge inherited values, catalog
values, then caller overrides. Reserved `JJ_ACP_TASK_*` identity values are
stripped and supplied by the harness.

Trusted stdio MCP tools can be supplied through `--tools tools.json` or
`JJ_ACP_TOOLS`. A declaration file is a JSON array:

```json
[
  {
    "name": "workspace",
    "command": "/absolute/path/to/tool-server",
    "args": [],
    "env": []
  }
]
```

The file allows at most eight unique servers and 64 KiB; commands must be
absolute. Tool selection belongs to the controlling application and cannot be
changed by a task or control frame. Do not put credentials in argv. The harness
forwards declarations when creating/loading supported ACP sessions; adapter MCP
transport support varies.

## Control a persistent process

The default `serve` interface is control version 1: scoped task submissions,
bounded worker queues, cancellation and terminal records.

```sh
lapis-acp serve --runner codex --cwd /approved/workspace --workers 2 --queue-capacity 8
```

Explicit version 2 retains one interactive session:

```sh
lapis-acp serve --control-version 2 --runner codex --cwd /approved/workspace
```

A controlling application reads `ready`, sends `open`, waits for `opened`, sends
`configure` with an advertised mode, waits for `configured` with `modeReady`,
then sends `prompt`. For example, request frames are newline-delimited JSON:

```json
{"version":2,"type":"open","requestId":"open-1"}
{"version":2,"type":"configure","requestId":"config-1","mode":"<advertised-mode-id>"}
{"version":2,"type":"prompt","requestId":"turn-1","text":"Inspect the workspace.","maxDurationMs":120000}
```

Consume streamed `activity` and `permission` frames until `record`. Permission
replies must use the exact live permission and option IDs. `close` and stdin EOF
request bounded cleanup. The parent owns redaction and durable transcripts.

**Control version 3 is experimental.** It negotiates one warm worker and opens
multiple caller-named sessions under a trusted managed directory root:

```sh
lapis-acp serve --control-version 3 --runner codex --cwd /approved/managed-root
```

Send `warm`, then `open` with an application `sessionId` and an existing `cwd`
under that root. Configure, prompt, cancel and permission frames carry the same
session ID. Warmup sends no prompt. Optional prepared `context` is separate from
authored `text`; nonblank context permits a comments-only turn. Valid session
preparation rejection preserves unrelated scopes; uncertain execution
invalidates the worker and is never replayed. Live-adapter,
permission/cancellation and broader consumer qualification remain open in M12.

See the [control specification](https://stevejuma.github.io/lapis-acp/control.html)
for complete request/response shapes and lifecycle rules.

## Develop and build from source

Install Rust 1.98.1, Node 22 or newer, pnpm 10.34.5, Python 3 and mdBook 0.5.4.

```sh
pnpm install --frozen-lockfile
pnpm spec:check
pnpm check
pnpm test
pnpm build
cargo build -p lapis-acp --release --locked
cargo install --path crates/lapis-acp-cli --locked
```

CI uses deterministic fixtures and requires no model account or credential.
Apache-2.0 attribution and package licenses ship in every distribution. Read
[contribution guidance](https://github.com/stevejuma/lapis-acp/blob/main/CONTRIBUTING.md)
and the repository's agent guide before changing protected behavior.

Releases use Changesets to keep all six Cargo crates on one version. Run
`pnpm changeset` to describe a release-worthy change, targeting the private
`@lapismd/lapis-acp` coordinator. `pnpm release:status` shows pending changes.
GitHub creates a **Version Packages** PR; merging it after passing CI starts
qualification, creates the immutable version tag and runs trusted Cargo/native
publication. For local preparation use `pnpm release:version`, which
synchronizes Cargo manifests, lockfiles, changelog and installation examples.
See
[release guidance](https://github.com/stevejuma/lapis-acp/blob/main/docs/releasing.md).
The
[0.1.0 history](https://github.com/stevejuma/lapis-acp/blob/main/docs/releases/0.1.0.md)
records the already published release. The npm coordinator remains private.

## Specification and provenance

The [published specification](https://stevejuma.github.io/lapis-acp/) mirrors the
canonical `spec/src` Markdown. Build locally with `pnpm spec:build`; generated
HTML is non-normative. The ledger is the single source of implementation status.

This is a derived work of [block/buzz](https://github.com/block/buzz)'s
Apache-2.0 `buzz-acp` crate. The ACP lifecycle and task harness were ported;
Nostr intake, identity, key custody and relay publication were excluded. The
pinned upstream and region map live in
[upstream-lock](https://github.com/stevejuma/lapis-acp/blob/main/docs/upstream-lock.md).
See [LICENSE](https://github.com/stevejuma/lapis-acp/blob/main/LICENSE.md) and
[NOTICE](https://github.com/stevejuma/lapis-acp/blob/main/docs/licenses/NOTICE.md).
