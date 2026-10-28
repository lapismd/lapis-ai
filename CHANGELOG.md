# @lapismd/lapis-ai

## 0.2.0

### Minor Changes

- 5017c03: Add an optional asynchronous session-update hook with synchronous-handler
  compatibility, and bounded control output handoff for bursty ACP responses.
- d087d64: Expose a process-free installed-model inventory and safe compute failure categories.
  Correlate model/runtime download and import progress with explicit operations, and
  retain preparation and imported-model display facts across application restarts.
- a93e795: Expose host-prepared Control V3 and agent clients, independent compute command
  and event ownership, and regular-file resource bundle verification.
- 8280545: Add typed HTTP MCP declarations for native host-owned Cursor tools. Existing
  stdio builders and wire fields are retained; session/runner tool vectors now
  accept McpServerConfig with explicit From conversion. Trusted launch files accept
  bounded literal-loopback HTTP endpoints and redacted headers, with environment
  references for authored authentication values. Parsing performs no network IO.
- d797747: Add the independent read-only session-source capture library with versioned
  product profiles and bounded physical-root discovery.
- 9fed4a7: Extract the reusable ACP host into lapis-ai-service with an owned runtime handle while preserving the Tauri adapter's public commands and admission contract.
- bc12d6a: Add Rust-owned ACP sessions with typed requests, exact permission choices,
  bounded acknowledged event recovery and supervised worker lifetime.
- fcd0be1: Add bounded transient JSONL and read-only OpenCode SQLite session pages for
  host-owned local import browsers. Source metadata and bodies stay native until
  the application applies privacy and explicit import policy.
- f1471bf: Allow native hosts to retain exact session-processing authority during renderer
  suspension while preserving current admission for commands, factories and startup.
- 8ae30a3: Add an optional host-admitted Tauri ACP transport with scoped worker and channel
  handles, cooperative startup revocation and native resume-binding policy.
- 669ebfe: Add a private typed ACP renderer client with a public Tauri transport, bounded
  ordered delivery, request correlation and explicit uncertain outcomes.

### Patch Changes

- 1f5bf52: Retain native ACP teardown ownership across cancelled close/revocation waits and
  wait for retiring workers during application shutdown.

## 0.1.1

### Patch Changes

- f445c3e: Prepare the shared native ACP, maintained agent, TOML configuration and supervised
  Rust clients for their first coordinated Cargo release. Distribute the pinned
  compute helper through native GitHub bundles and publish the canonical spec.

## 0.1.0

Local foundation import: native ACP, maintained agent, compute, TOML configuration
and supervised clients. This source baseline has not been published to crates.io.
