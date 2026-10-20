# Configuration and Rust usage

Select configuration explicitly:

```sh
target/debug/lapis-acp --config examples/lapis-ai.toml --check-config
target/debug/lapis-acp serve --config examples/lapis-ai.toml
target/debug/lapis-agent --config examples/lapis-ai.toml --models
target/debug/lapis-compute --config examples/lapis-ai.toml
```

`--check-config` parses and validates without resolving secrets or starting
services. Unknown TOML fields and unsupported versions fail closed. Every path
resolves against its selected file. There is no configuration discovery in the
current directory. Use a private application data root for compute and agent
state; it must be distinct from an admitted source workspace.

Explicit CLI flags override TOML, then retained compatibility environment values,
then defaults. Rust hosts can override configuration fields before preparing a
launch, or use `PreparedLaunch::override_environment` for a prepared process.
Helper stdin/stdout remain JSON protocols. Models, invitations and private state
keep their original formats; TOML is the authored configuration format.

`examples/lapis-ai.toml`, `runners.toml` and `tools.toml` form a complete example.
Select an actual model and supply the named environment references. For local
compute the host obtains the authenticated loopback endpoint from readiness and
passes it to the agent. Compute startup only binds its endpoints; preparation,
model download/import and model start are explicit operations.

## Credentials

Use `NAME = { env = "SOURCE_ENVIRONMENT_NAME" }` or
`NAME = { host = "application-secret-reference" }` under a component's
`environment` table. Compute `token` and `identity` use the same references.
A Rust `Resolver` implements `environment` and optional `host_secret` callbacks.
The helpers' CLI uses environment references; host references require a Rust
client and host resolver. Ordinary values use TOML strings. Secret-like names
cannot use literal values. References and resolved launch environments redact
values in Debug output and errors use fixed classifications.

The helpers retain the `JJ_ACP_*`, `BUZZ_*`, provider, `JJ_STORE_COMPUTE_*`,
`JJ_STORE_CHATGPT_*` and `LAPIS_AGENT_TOKEN_*` compatibility namespaces. The
legacy environment allowlist is in `lapis-ai-config/src/launch.rs`. Hosts should
supply dedicated private HOME/config roots where provider adapters need them.
Private token-source sockets and capabilities stay host-owned. No credential is
passed as a process argument or persisted in TOML.

## Runners and tools

TOML runner catalogs use `[[runners]]`: `id`, `label`, `command`, optional `args`,
`env`, `enabled`, `mcp_transport`, `alternate_commands`, `install_hint` and
`install_instructions_url`. Camel-case compatibility spellings are accepted;
duplicate spellings are refused. Bare runner commands resolve through PATH;
relative executable paths containing a slash resolve against the catalog.
Runner `env` is a table of ordinary strings or `{ env = "REFERENCE" }` values.
It resolves only when the selected catalog is loaded for launch/discovery.
Compiled-in runner identifiers cannot be shadowed.

TOML stdio MCP declarations use `[[tools]]`: `name`, `command`, `args` and `env`.
Commands are file-relative. `env` is an array of `{ name = "NAME", value = "text" }`
or `{ name = "NAME", env = "REFERENCE" }`. Credential-like variables require a
reference. JSON catalogs and JSON MCP declarations remain compatibility inputs.
For host-owned HTTP application tools, use `type = "http"`, `name`, `url` and
`headers` instead. URLs must use a literal loopback IP with an explicit port;
credentials, queries and fragments in URLs are refused. Header entries use
`name` and `env` references, for example
`headers = [{ name = "Authorization", env = "LAPIS_TOOL_AUTHORIZATION" }]`.
The referenced value includes any authentication scheme such as `Bearer`.
Parsing creates no listener and does not probe the endpoint. The controlling
application owns HTTP authentication, approvals and the endpoint lifetime.
Rust callers retain the existing `McpServer` stdio builder and convert it with
`.into()` where session/runner configuration now accepts `McpServerConfig`;
`HttpMcpServer` supplies the additional typed transport. Stdio wire output is
unchanged.
Only the trusted Rust host selects catalogs and tool executables. ACP permissions
are an additional gate; tools are not safe to expose as arbitrary WebView launch
commands.

## Supervised clients

Add `lapis-ai-config` and `lapis-ai-runtime` as Rust dependencies. They share the coordinated workspace version. The first registry release is
pending; use a source checkout until a release is available. `Config::load` is inert. `AcpClient::start`
launches Control V3 and validates readiness; its command sender retains request
IDs, session handles and exact provider permission option IDs.

The independent Rust example demonstrates:

```sh
pnpm dev:cargo -- run -p lapis-ai-runtime --example native-session -- examples/lapis-ai.toml
```

The lifecycle is ready → warm → open admitted workspace → configure execution
mode → prompt with a maximum duration → events/permission → terminal record →
close. The example uses `ask` mode and denies tools by default. Only its explicit
`--approve-tools` flag chooses `allow_once`; production hosts implement their own
approval UI and policy. ACP frames are bounded at 1 MiB; compute commands at
64 KiB including the newline. Generic subprocess output is bounded at 4 MiB and
256 queued frames. Coverage loss fails closed. A timeout or broken pipe is an
unknown outcome and never resubmits the prompt.

`AgentClient` exposes bounded model and metadata calls. `ComputeClient` exposes
configuration/pairing, model prepare/download/import/remove/start, events and
non-streaming inference. Each instance owns its process, environment and data
root; there is no machine-wide singleton. Explicit shutdown closes input, waits,
then terminates and reaps the process group when necessary. Dropping a client
forces teardown as a fallback; applications should await shutdown before ending
their Tokio runtime.

A packaged host calls `ResourceBundle::load(bundle_directory)`, then
`bundle.configure(&mut config)` before starting clients. The loader verifies the
platform, helper inventory, version, hashes and pinned native ABI. The Rust
example also accepts `--bundle PATH`. Hash verification is integrity checking for
an already admitted bundle; signing and distribution trust remain host-owned.

## Hosts with existing orchestration

Use `PreparedLaunch::from_command(&command)` after selecting the executable,
explicit environment and working directory in trusted native code. No implicit
parent environment is copied. A deliberately unset variable remains absent.
`AcpClient::start_prepared_with_options(plan, AcpOptions::default())` retains
host launch flags and fixes Control V3. `readiness()` provides the consumed
ready frame; use `commands().send_frame(&frame)` for an already admitted V3
frame. Existing `AcpSession` startup keeps its fixed argument contract.

`AgentClient::models_prepared` and `metadata_prepared` accept prepared launches,
`ProcessLimits` and a deadline. Input and output bounds are independent, allowing
a bounded transcript with a smaller metadata result. Cancellation drops the
owned process; hosts keep credential leases alive until cleanup.

For live compute controls, start with `ComputeClient::start_prepared`, retain
`commands()` independently and drive `next_event()` from one event owner. A
command submission acknowledges transport only; correlated events establish
completion. Do not run a competing event reader alongside convenience calls.
`connection()` exposes a native-only endpoint and bearer token; `peer_endpoint`
constructs a peer URL after the host admits that identity. These credentials
are neither serializable nor visible in Debug output. The connection does not
perform pairing or grant repository/tool access. Await `shutdown()` on exit.
