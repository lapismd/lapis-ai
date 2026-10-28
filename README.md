# Lapis AI

[![CI](https://github.com/lapismd/lapis-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/lapismd/lapis-ai/actions/workflows/ci.yml)
[![Specification](https://github.com/lapismd/lapis-ai/actions/workflows/publish-spec-pages.yml/badge.svg)](https://lapismd.github.io/lapis-ai/)

Shared Rust AI infrastructure for Lapis applications: ACP transport, a maintained
Buzz-derived agent, workspace tools, local/paired compute, TOML configuration and
supervised Rust clients. Applications enable services independently and own
permissions, workspace admission, context, credentials and conversation history.

This is the shared backend foundation for Rust applications, including future
Lapis Notes Tauri integration. Production helpers use Rust and native processes;
Node and Python are development tools. The [Tauri ACP transport and typed client](spec/src/current/contracts/tauri.md)
are qualified independently; application context, history, tools and optional
compute retain their consuming application gates.

## Packages and delivery

All fourteen Cargo packages share one version. The private npm package is a development
and Changesets coordinator. The first registry publication is pending.

| Package              | Responsibility                                             | Delivery                  |
| -------------------- | ---------------------------------------------------------- | ------------------------- |
| `lapis-acp`          | CLI discovery, prepared tasks and persistent Control V1–V3 | Cargo + native executable |
| `lapis-acp-protocol` | Bounded ACP JSON-RPC framing and payloads                  | Cargo library             |
| `lapis-acp-agent`    | Trusted catalogs and launch environments                   | Cargo library             |
| `lapis-acp-runner`   | Agent subprocess and session lifecycle                     | Cargo library             |
| `lapis-acp-control`  | Application control protocol                               | Cargo library             |
| `lapis-acp-testkit`  | Deterministic model-free test fixtures                     | Cargo library             |
| `lapis-agent`        | Provider adapters, models, permissions and MCP tools       | Cargo + native executable |
| `lapis-agent-tools`  | Permissioned workspace tools over MCP stdio                | Cargo + native executable |
| `lapis-ai-config`    | Inert TOML and secret-reference resolution                 | Cargo library             |
| `lapis-ai-app-tools` | Injectable bounded MCP stdio transport for app-owned tools | Cargo library             |
| `lapis-ai-capture`   | Read-only bounded external-session source discovery        | Cargo library             |
| `lapis-ai-runtime`   | Supervised helper clients and resource verification        | Cargo library             |
| `lapis-compute`      | Authenticated local inference and admitted paired compute  | Native executable only    |

Compute retains the pinned Mesh SDK revision; its `0.76.0-rc9` dependencies are
not available on crates.io. `publish = false` is deliberate. Native bundles
contain all four production helpers, runtime inventory, licenses and checksums,
with no model weights. The admitted native profiles are macOS ARM64 Metal and
Linux ARM64 CPU with glibc 2.38 or newer. Other native profiles, signing and
installers require separate qualification.

## Build and run

Install Rust **1.99.0**, Node **22+**, pnpm **10.34.5**, Python 3 and mdBook
**0.5.4**. Linux source builds need `pkg-config` and OpenSSL development headers.
Cargo commands use the disk-budget wrapper, which requires 30 GiB free.

```sh
pnpm install --frozen-lockfile --ignore-pnpmfile
pnpm dev:cargo -- fetch --locked
pnpm build
pnpm check
pnpm test
pnpm spec:check

target/debug/lapis-acp --config examples/lapis-ai.toml --check-config
target/debug/lapis-acp agents
target/debug/lapis-agent --config examples/lapis-ai.toml --models
```

`pnpm test` includes the explicit `lapis-agent/test-support` feature so fixture
executables are available. A normal build or Cargo install does not install
those fixtures. Run `pnpm dev:cargo -- <arguments>` for focused Cargo work.

When a registry version is available, executable installation uses:

```sh
cargo install lapis-acp --version 0.1.1 --locked
cargo install lapis-agent --version 0.1.1 --locked
cargo install lapis-agent-tools --version 0.1.1 --locked
```

Native distributions are published through [GitHub releases](https://github.com/lapismd/lapis-ai/releases).
Verify `SHA256SUMS` before admitting a bundle. Native preparation and model
selection/download are explicit operations. Source and release builds never
require an account with a model provider.

## Use from a Rust host

Use `lapis-ai-config` and `lapis-ai-runtime` at the same released version, or
workspace paths while preparing the initial release. A host loads `Config`,
resolves approved references and starts only the clients it needs: `AcpClient`,
`AgentClient` or `ComputeClient`. Each instance owns its process and private data
root; there is no machine-wide singleton or shared AI controller.

```toml
version = 1

[acp]
executable = "/absolute/path/to/lapis-acp"
runner = "local"
catalog = "runners.toml"
```

See the complete [example configuration](examples/lapis-ai.toml),
[runners](examples/runners.toml), [tools](examples/tools.toml), and
[configuration and Rust usage](docs/usage.md). Paths resolve relative to the
selected TOML file. Configuration loading is inert; unknown fields fail closed.
Credentials use environment or host-secret references, never literal TOML values.

```sh
pnpm dev:cargo -- run -p lapis-ai-runtime --example native-session -- examples/lapis-ai.toml
```

The example denies tools by default. The trusted application owns workspace
admission and must choose the exact permission option for each approved request.
Timeouts and broken pipes are unknown outcomes; prompts are never replayed.
Third-party ACP adapters are separate installations and may have their own runtime
requirements. See [qualification and packaging](docs/qualification.md) for real
inference, client and bundle acceptance commands.

## Changesets and releases

Run `pnpm changeset` and select **@lapismd/lapis-ai** for a release-worthy change.
`pnpm release:status` shows the coordinated plan. GitHub maintains a **Version
Packages** PR that updates all Cargo versions, internal ranges, lockfiles and
changelog. The PR requires human review; automation never merges it.

After a reviewed version PR is merged, exact-source CI qualifies the version,
creates an immutable tag and starts protected Cargo/native publication. Registry
retries verify existing package checksums and publish only missing packages.
The npm coordinator stays private. Initial crates.io namespace bootstrap and
subsequent trusted-publisher setup are documented in [releasing](docs/releasing.md).
This setup intentionally leaves the first version PR open and publishes no crates.

## Specification and provenance

Read the [published specification](https://lapismd.github.io/lapis-ai/),
[canonical Markdown](spec/src/current/index.md) and
[milestones](spec/src/current/progress/milestones.md). `pnpm spec:build` builds
mdBook locally. PRs build the book; main deploys it to GitHub Pages. Generated
HTML is non-normative.

New production files stay within 300 counted lines, tests within 500 and spec
Markdown within 400 physical lines / 32768 bytes. Imported exceptions never grow.
The [source pins](docs/provenance.toml), [import manifest](docs/imports/source-manifest.json)
and [agent attribution](docs/agent-upstream.md) preserve upstream ownership.
Apache-2.0; see [LICENSE](LICENSE.md) and [notices](docs/licenses/NOTICE.md).

## Typed renderer client

The private `@lapismd/ai` migration package exposes a typed client from built
`dist` and an optional `@lapismd/ai/tauri` transport. `pnpm build:client`,
`pnpm check:client` and `pnpm test:client` qualify this boundary; normal root
commands include them. It shares the coordinated version and has no npm release
lane. See [the client guide](packages/ai/README.md) for consumption and recovery.

Native source discovery is independently reusable through `lapis-ai-capture`.
See [the capture guide](docs/capture.md). Hosts retain activity admission,
redaction and durable publication; registering a profile grants none of these.

For host-injected MCP tools, see [the application-tool guide](docs/app-tools.md).
The engine supplies no native execution or application authorization.
