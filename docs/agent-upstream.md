# Bundled local agent

The vendored buzz-agent source and tests are pinned to Buzz revision
ef2aa1ae38fadcc0bc22b8bf6ed96b35933146be (Apache-2.0). Original source:
crates/buzz-agent in https://github.com/block/buzz. Cargo workspace fields are
made standalone. src/lib.rs additionally advertises a single ask mode and
accepts selecting it for an existing session; the existing permission gate
remains mandatory for every tool call. Modified files carry an attribution
notice. The owned entrypoint supplies a stable product executable name and
avoids raw diagnostics. Provider defaults, standing instructions, secret custody
and coding tools belong to jj-store. Upstream LICENSE is retained with the
crate. Updating this pin requires protocol/tool/cancellation qualification.

The maintained fork additionally advertises and implements ACP mode config
options for permission-gated Ask mode, and performs bounded authenticated model
discovery against the configured OpenAI-compatible provider. Paired catalogs
therefore come from the selected inference host; warm initialization still
performs no discovery or prompting.

The pinned model capability manifest and normative test corpus from upstream
`scripts/` are retained under the owned `runtime/local-agent/scripts/`
directory. The subprocess fixture serves model discovery separately from
scripted completion responses, preserving the permission and cancellation
assertions. Catalog discovery is enabled by the owned local-agent entrypoint;
the vendored upstream entrypoint retains its configured-model behavior.

The owned `--models` entrypoint additionally discovers Anthropic (paginated),
OpenAI/compatible, OpenRouter (tool-capable only), and Databricks/v2 catalogs.
It resolves the vendored provider configuration and existing authentication,
never interactive sign-in. The helper has a 20-second deadline and bounded
model/output validation; the parent supervises cancellation and process reaping.
It advertises the existing Ask mode only. Empty catalogs remain empty, including
Databricks v2's otherwise curated fallback. The vendor pin is unchanged.
Provider/menu behavior references Buzz
`desktop/src/features/agents/ui/agentConfigOptions.tsx` and
`desktop/src-tauri/src/commands/agent_models{,_openrouter}.rs` at the same pin.

The owned `--metadata` entrypoint calls the maintained fork's bounded tool-free
summarization API without constructing ACP sessions or MCP tools. Its version-1
JSON input is capped at 128 KiB of transcript and output at 16 KiB, with a
120-second deadline. The prompt treats capture as untrusted. Missing Databricks
credentials refuse instead of opening interactive sign-in. Request durability,
authorization and publication belong to the controlling application.

Metadata summaries select a maintained single-attempt transport policy. Coding
turns retain their existing retries; metadata disables transport/auth/endpoint
replay after an unknown completion. The owned helper rejects malformed results.

The maintained fork adds a distinct `chatgpt-plan` Responses dialect and an
injectable access-token source. The application performs dynamic-client OAuth,
Keychain custody and rotating refresh; the worker receives only account-bound
access tokens through private Unix IPC. The public OpenAI endpoint is fixed and
cannot be replaced by provider environment settings. Subscription completion,
summary and compaction calls consume bounded SSE and require a completed
terminal event before releasing text or tool calls. History retains exact output
and opaque reasoning in memory; canonical capture never serializes this provider
replay state. One confirmed pre-stream 401 permits renewal; uncertain requests
are not retried. The upstream pin, other provider transports, MCP credential
filtering and mandatory tool permissions remain in place.

The protocol follows OpenAI's OSS registration, account/session, model and
inference documentation at
https://developers.openai.com/siwc/token-sharing-open-source. Rust
implementation research also examined Everruns revision
`ef90d32744503cced40fda8e113768121f805657`; this integration introduces no
Everruns package or source dependency. Current Zed/Goose references use Codex
backends and do not establish this direct public subscription flow. Synthetic
OAuth/JWKS and inference fixtures qualify the owned boundaries separately from
live account and signed desktop acceptance.

Local Rust 1.99 qualification also simplifies equivalent hint/catalog guards,
keeps test modules after production items, scopes a synthetic request lock
before awaiting, and checks that the unknown-inference fixture actually reads
request bytes. Provider retries, tool permission gates and the source pin are
unchanged.
