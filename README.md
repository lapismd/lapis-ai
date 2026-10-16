# Lapis AI

Shared Rust AI infrastructure for Lapis applications. This standalone repository
contains `lapis-acp`, the maintained Buzz-derived `lapis-agent`,
`lapis-agent-tools`, `lapis-compute`, and reusable configuration/client libraries.
Applications select services independently and own permissions, workspace
admission, conversation history, context and credentials.

The runtime uses Rust and native helper processes. Node and Python are development
tools. Tauri integration is currently a [contract](spec/src/current/contracts/tauri.md);
there is no shared AI controller or consumer migration in this repository.

```sh
pnpm install --frozen-lockfile
pnpm build
pnpm check
pnpm test
pnpm spec:check
```

Rust 1.99.0 and the dependency lock are pinned. Cargo commands go through
`pnpm dev:cargo -- ...`, which checks available disk space. New production files
stay within 300 counted lines, tests within 500; pinned imported exceptions never
grow. Specification Markdown stays within 400 physical lines / 32768 bytes.

| Package                            | Purpose                                                                  |
| ---------------------------------- | ------------------------------------------------------------------------ |
| `lapis-acp` and five ACP libraries | ACP subprocess transport, catalog, lifecycle, Control V1–V3 and fixtures |
| `lapis-agent`                      | Maintained Buzz provider adapters, permission gates, models and metadata |
| `lapis-agent-tools`                | Workspace coding tools over trusted MCP stdio                            |
| `lapis-compute`                    | Authenticated local inference and explicit paired compute                |
| `lapis-ai-config`                  | Inert versioned TOML and secret-reference resolution                     |
| `lapis-ai-runtime`                 | Supervised Rust clients and verified resource bundles                    |

Start with [configuration and Rust usage](docs/usage.md),
[qualification and packaging](docs/qualification.md), the
[canonical specification](spec/src/current/index.md), and
[milestones](spec/src/current/progress/milestones.md).

The imported code is pinned in [provenance](docs/provenance.toml) and the
[source manifest](docs/imports/source-manifest.json). Apache-2.0; see
[notices](docs/licenses/NOTICE.md) and [agent attribution](docs/agent-upstream.md).
This local repository has no release remote. Publishing and application adoption
are separate work.
