# Runner catalog

A runner is an external ACP-speaking agent that `lapis-acp` can spawn. Runners are
data, not code: a catalog entry names an executable, its arguments, an
environment floor, an MCP transport and an install hint. Adding support for a
new agent MUST be a catalog change.

The catalog is tiered so that trust decreases as flexibility increases, matching
the model the upstream harness uses and the model `@lapismd/ai-host` already
exposes to Lapis consumers.

| Tier | Source                                                | Editable | Auto-install |
| ---- | ----------------------------------------------------- | -------- | ------------ |
| 1    | Compiled-in defaults                                  | No       | Never        |
| 2    | Catalog file supplied by launch configuration         | No       | Never        |
| 3    | Caller-supplied definition in a task or control frame | No       | Never        |

No tier may auto-install. Availability is determined by resolving the declared
executable, and an unavailable runner reports its install hint rather than
attempting a download or executing a shell command.

## Requirements

| ID            | Requirement                                                                                                                                  |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-AGENT-001 | A runner MUST be declared as data carrying identifier, label, command, arguments, environment floor, MCP transport and install hint.         |
| LAI-AGENT-002 | Adding a runner MUST NOT require a code change, an install shell command or a network fetch.                                                 |
| LAI-AGENT-003 | Reserved harness environment keys MUST be stripped before a definition or caller override is merged, and MUST NOT be settable by any tier.   |
| LAI-AGENT-004 | A disabled or unknown runner identifier MUST fail resolution and MUST NOT fall back to a default runner.                                     |
| LAI-AGENT-005 | Runner identifiers MUST be validated against a fixed pattern and MUST be unique within a resolved catalog.                                   |
| LAI-AGENT-006 | The launch environment MUST be assembled from the inherited environment, the definition floor and caller overrides in that precedence order. |

## Reserved environment

The harness injects identity and correlation variables into a spawned agent so
that tools it provides can attribute work correctly. Those names are reserved. A
runner definition, a catalog file or a caller-supplied override that sets one is
stripped before merge, and the strip is recorded so the operator can see that a
definition attempted to claim harness identity.

This is the only place `lapis-acp` performs anything resembling an authorization
check, and it is a namespace guard rather than an identity system. There is no
keypair, no signature, no token and no peer credential inside the ACP crates. The controlling application decided to run this runner with this
environment; `lapis-acp` only refuses to let the runner definition contradict that
decision.

## Default catalog

The compiled-in tier names the adapters that are already in use elsewhere in the
workspace, so an operator who has one of them installed needs no configuration.
Identifiers are stable and reserved: a lower tier cannot shadow them.

| Identifier | Adapter                                      | MCP transport |
| ---------- | -------------------------------------------- | ------------- |
| `codex`    | `@agentclientprotocol/codex-acp`             | stdio         |
| `claude`   | `claude-agent-acp`, legacy `claude-code-acp` | stdio         |
| `opencode` | `opencode`                                   | stdio         |
| `goose`    | `goose acp`                                  | stdio         |
| `cursor`   | `cursor-agent acp`                           | http          |

A runner that speaks ACP over stdio and satisfies the protocol requirements in
[ACP wire protocol](protocol.md) works without being listed here. The catalog
exists for discovery and labelling, not as an allowlist.

## Implemented surface

| Module         | Owns                                                                               |
| -------------- | ---------------------------------------------------------------------------------- |
| `definition`   | `RunnerDefinition`, `CommandSpec`, `McpTransport`, identifier and label validation |
| `catalog`      | Tiers, collision rules, resolution and listing                                     |
| `defaults`     | The compiled-in tier and its reserved identifiers                                  |
| `config`       | Catalog document parsing                                                           |
| `environment`  | Reserved-key stripping and three-layer merge                                       |
| `availability` | The `CommandProbe` trait, a `PATH` probe, a fixed probe and availability reports   |
| `error`        | Typed refusal reasons                                                              |

A candidate argv is a `CommandSpec`, and a definition may list several in
preference order. The first whose executable resolves wins, which is how one
definition covers both a current and a legacy adapter command name without
inventing two runners.

Resolution refuses rather than substitutes. An unknown identifier and a disabled
one are distinct errors, and neither falls back to a default, because silently
running a different agent than the one requested would execute untrusted prompt
content with capabilities the caller did not choose.

Collisions are refused for the whole batch rather than skipping the offending
entry. A catalog file that fails to load therefore cannot produce a quietly
smaller runner set than the operator wrote.

`CommandProbe` is a trait so availability is testable against a synthetic search
path. The real probe only tests whether a file exists; it never executes the
candidate to see whether it works, because executing an untrusted binary to
probe it would be worse than not probing at all.

## Environment precedence

The three layers apply in order: the inherited environment, the definition
floor, then caller overrides. A later layer replaces an earlier one for the same
name, and the first appearance fixes its position so the result is
deterministic.

Every layer is stripped of names under the `JJ_ACP_TASK_` prefix before merging,
and the stripped names are returned so an operator can see that a definition
tried to claim harness identity. The harness then adds its own reserved
variables through a separate function that no catalog file and no caller
override can reach. The prefix is matched rather than an enumerated list, so a
future harness variable is covered without a rule change.

## Qualification and TOML

Current qualification is recorded in the Lapis AI verification table. Adapter
presence in a source checkout is not live acceptance here. Trusted TOML catalogs
are described in `docs/usage.md`; JSON remains a compatibility input. Authored
TOML rejects unknown fields and duplicate alias spellings, resolves executable
paths against its file and requires environment references for credentials.

Cargo publication metadata and coordinated versions follow the
[release contract](../release.md). The ACP wire, catalog, session, task and
permission contracts remain unchanged by release preparation.

The GitHub ownership transfer updates package documentation links to
`lapismd/lapis-ai`; the owning ACP requirements and runtime behavior are unchanged.
