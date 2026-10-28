# Trusted HTTP MCP declarations

Revision: enclosing Jujutsu change `qtutmtzk`, based on `d8d6a61d`.

Milestone review: this is the first shared prerequisite of the approved desktop
M10.2–M10.4 implementation plan. M0–M5 retain their original status and gates.
Desktop M10.2/M10.3 remain incomplete; M10.4 remains planned. The desktop's
uncommitted E-AI-096 draft slice overlaps chat and canonical milestone files,
so it is preserved without desktop integration in this source change.

## Contract review

LAI-PROTO-003 extends session configuration with `McpServerConfig` and
`HttpMcpServer`. The existing `McpServer` stdio builder and its exact four wire
fields remain intact; Rust configuration vectors use explicit `From` conversion.
HTTP encoding includes the ACP transport discriminator and redacted headers.
LAI-PROTO-006, LAI-CTRL-001 and LAI-RUN-001 retain process, trust, tool policy,
permission and uncertain-outcome ownership. Encoding and parsing perform no
network operation and create no listener.

The existing trusted `--tools`/`JJ_ACP_TOOLS` file accepts mixed stdio and HTTP
declarations under one eight-server/64-KiB bound. URLs require literal loopback
IPs and explicit nonzero ports; user information, queries, fragments, controls,
ambiguous host aliases and remote endpoints refuse. Headers are bounded and
case-insensitively unique, reject controls and transport-framing overrides, and
retain exact values on the wire. Authored TOML authentication headers require
environment references. Credentials are redacted from nested Debug output.
Task/control input gains no tool endpoint or credential authority.

The coordinated minor Changeset records the Rust interface addition. No
version, tag, package publication or running application changes occur.

## Commands

- Before: `pnpm dev:cargo -- test -p lapis-acp --lib tools::tests --locked`.
- `pnpm dev:cargo -- fmt --all`.
- `pnpm dev:cargo -- test -p lapis-acp --lib tools::tests` (updates the existing
  lock entry with the explicit URL parser dependency; subsequent runs locked).
- `pnpm dev:cargo -- test -p lapis-acp-protocol --test mcp_http --test payload --test mcp_environment_debug --locked`.
- `pnpm dev:cargo -- test -p lapis-acp-protocol -p lapis-acp-agent -p lapis-acp-runner -p lapis-acp-control -p lapis-acp --locked`.
- `pnpm dev:cargo -- test -p lapis-acp --test http_tools --locked`.
- `pnpm test > .local/m10-http-tests.log 2>&1`.
- `pnpm dev:cargo -- clippy -p lapis-acp-protocol -p lapis-acp-runner -p lapis-acp-control -p lapis-acp --all-targets --locked -- -D warnings > .local/m10-http-clippy.log 2>&1`.
- `pnpm check:architecture`, `pnpm check:boundaries`, `pnpm spec:validate`,
  `pnpm spec:first`, `pnpm check:progress`, `pnpm check:upstream`.
- `pnpm exec prettier --write` for changed Markdown only.
- `pnpm dev:cargo -- fmt --all -- --check` and
  `pnpm exec prettier --check` for every changed Markdown file.
- `pnpm spec:check > .local/m10-http-spec.log 2>&1`.

The five-package suite covers the complete shared ACP impact surface. The final
workspace suite is required by the owning guide because this shared public
configuration feeds the native adapter and supervised clients too; focused
passes alone do not establish that integration.

## Results

The baseline's two stdio tests pass; its trusted parser admitted no HTTP
declaration. All six declaration tests now pass, covering mixed transports,
secret diagnostics, URL/port admission, header injection/duplicates and authored
secret references. All 29 selected protocol tests pass. The new real-launcher
integration test passes: both declarations reach the scripted provider with
exact headers while CLI output omits the credential marker.

The complete five-package ACP suite passes. Strict Clippy passes for all changed
Rust packages/targets. Architecture, fourteen-package dependency boundaries,
canonical validation, progress/import provenance and upstream-map self-tests
pass. The complete `pnpm test` passes: 1,061 Rust tests across 101 executable/doc
targets, two inherited ignored tests, 43 renderer-client and 45 governance tests.
Final Rust and changed-document formatting pass. Canonical validation covers
43 chapters/101 requirements, provenance retains 186 source records, and the
upstream self-test retains forty curated digests. Coordinated release metadata
remains at 0.1.1.
The complete ten-lane specification check passes, including client checks,
governance, mdBook, format, boundaries, progress and spec-first. Final evidence
and compact verification-link edits rerun the consuming document checks.

Versions: macOS ARM64, Rust 1.99.0, Node 26.9.0 and pnpm 10.34.5. No actual
Cursor/OpenCode/Lapis model or desktop provider acceptance runs in this slice.

## Failures

The first `spec:first` correctly rejected missing mapped CLI, control, runner
and governance chapter reviews. Those chapters now explicitly document the
addition or unchanged contracts; the gate is rerun with the evidence.
One combined documentation patch used an incorrect runner heading and applied
no changes; it was corrected without affecting implementation.

The workspace build emits the existing macOS compute linker warning about a
large `__eh_frame` section. This source change does not modify compute or waive
its native qualification.

## Gaps

This proves declaration parsing and delivery, not a Cursor tool server. Native
HTTP authentication/listener lifetime, AppToolHost/approval wiring, real Cursor
and other provider acceptance, Keychain integration and compute remain desktop
implementation gates. The generic MCP stdio engine is unchanged. No model,
provider login, credentials, user cache or vault is used by the new tests.

Next action: integrate the approved native runtime/configuration and HTTP tool
host after the overlapping desktop draft change is safely settled. Native
AgentStore/ThreadStore chat remains deferred; publication remains unauthorized.

Gate result: passed (trusted HTTP declarations only; desktop gates remain open).
