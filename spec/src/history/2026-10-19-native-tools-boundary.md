# Native tool snapshot and diagnostic boundary

Revision: enclosing Jujutsu change `nokqurst`, based on `e3bfffde`.

Milestone review: TOOLS-01B source prerequisites advance; the Notes native host
and mounted Codex acceptance remain separate. M0–M5 and publication retain their
original gates. Local transcript files remain the approved profile; JJ
persistence is deferred.

## Contract review

LAI-TOOLS-001/002 expose `Config::validate` for native hosts before transport
resource allocation. `serve` uses that same validator. Snapshot bounds, names,
duplicate refusal, schemas, wire framing and execution authority are unchanged.
The public regression exercises valid, duplicate and over-budget snapshots.

LAI-PROTO-006 retains exact MCP environment serialization while `EnvVar::Debug`
redacts values, including nested `McpServer` diagnostics. Names remain visible.
Applications still own credentials. No secret is persisted or transformed by
this change. The initial exact test demonstrates the derived Debug leak; the
fixed test also proves exact JSON delivery remains unchanged.

## Commands

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-acp-protocol --test mcp_environment_debug --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-app-tools -p lapis-acp-protocol --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- clippy -p lapis-ai-app-tools -p lapis-acp-protocol --all-targets --locked -- -D warnings`
- `pnpm dev:cargo -- fmt -p lapis-ai-app-tools -p lapis-acp-protocol`
- `pnpm exec prettier --write spec/src/current/contracts/acp/protocol.md spec/src/current/contracts/app-tools.md`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- package -p lapis-ai-app-tools --allow-dirty --locked`
- `pnpm spec:check` (including exact reruns after the recorded failures).

## Results

The exact Debug regression passes after one purposeful before failure. All
76 affected ACP protocol cases, 16 MCP engine cases and one protocol doctest
pass. Strict all-target Clippy passes for both crates. Original cancellation,
deadline, blocked-output and partial-frame regressions remain unchanged.
The independent Cargo archive verifies with 17 files, 60.2 KiB source and
16.9 KiB compressed; SHA-256
`c0897c5755b1afb7e73e285f116115b16ae5ef48312cfb466b822ec39e71ecdf`.
Logs: `.local/native-tools-env-debug-{before,after}.log`,
`native-tools-source-{tests,clippy,package}.log`.

## Failures

The new Debug test initially fails once because derived Debug exposes the
sentinel value. Manual field-level redaction fixes that exact target without
changing serialization or suppressing diagnostics/tests. Native consumer
compilation and acceptance are tracked in Notes, not inferred here. The first
spec-check run identifies the missing new history SUMMARY entry; add its exact
navigation row and rerun that gate.
The next run passes validation, formatting, boundaries, governance and mdBook
but requires the existing mapped runtime chapter for protected crate edits.
Record the unchanged secret-safe diagnostic contract there and rerun spec-first.
The next governance run hits the 30 GiB reserve at 29.8 GiB; no assertion is
weakened. jj-store's managed cleanup previews and removes eligible idle build
outputs, restoring 32.1 GiB without touching vaults or running applications.
The complete original spec gate then passes all ten lanes, including all
45 governance cases, formatting, architecture, boundaries, progress, upstream
integrity, mdBook and spec-first. Final log:
`.local/native-tools-source-spec-accepted.log`.

## Gaps

Authenticated native IPC, host policy/lifetime, registry/approval wiring, mounted
Codex reads and guarded writes remain Notes TOOLS-01B/C. The source changes do
not enable runtime tools. Broad unrelated provider/platform/distribution and
release qualification remain user-deferred. Full workspace integration is a
later gate; this record reports the exact affected Rust surface.

Next action: qualify and consume this source boundary in the native Notes
bridge, then verify actual Codex tools with local transcript persistence.

Gate result: passed (snapshot validation and redacted environment diagnostics).
