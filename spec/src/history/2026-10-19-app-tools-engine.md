# Injectable native application-tool engine

Revision: enclosing Jujutsu change `skxkwmvu`, based on `d3a1785b`.

Milestone review: TOOLS-01A advances the shared MCP stdio prerequisite.
Notes' native tool capability remains disabled until authenticated host wiring
and mounted consumer acceptance. M0–M5, desktop M10.2/M10.3 and publication
retain their original gates. JJ persistence remains deferred by the user.

## Contract review

LAI-TOOLS-001–004 specify the independent `lapis-ai-app-tools` crate. Hosts inject
an immutable tool snapshot and async handler. The engine has no JJ, Tauri,
filesystem, network, credential or process-selection edge. Current registry,
scope, approvals, schema/privacy policy, durable effects and real execution drain
remain host-owned. LAI-TOOLS-004 stays Planned until native host qualification;
only its snapshot whitelist is proven by this source slice.

The MCP handshake is adapted from the existing coding-tool server at
`crates/lapis-agent-tools/src/main.rs` under the same Apache-2.0 ownership.
Existing MCP coding tools and ACP control-v3 remain unchanged. Trusted TOML
launch tools already support environment references; they are reused, not
reimplemented. No helper binary, tool access, credential configuration or
ordinary application behavior changes in this slice.

LAI-REL-001/003 add the new library to the coordinated publication inventory;
no version, tag or registry publication occurs. Metadata, lock entry, notices,
license, crate README and full embedding guide accompany its public Rust API.

## Commands

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-app-tools --test lifetime inflight_completion -- --nocapture`
- Same exact regression with `--locked` after retaining the partial read future.
- `pnpm dev:cargo -- test -p lapis-ai-app-tools --locked`
- Exact `capacity_refuses` after signalling cancellation before host-future drop.
- Initial `--test blocked_output` regression, then `stalled_output --locked` after
  separating the bounded writer. The temporary standalone test moves into the
  owning lifetime test binary; its assertions are unchanged.
- `pnpm dev:cargo -- clippy -p lapis-ai-app-tools --all-targets --locked -- -D warnings`
- `pnpm dev:cargo -- package -p lapis-ai-app-tools --allow-dirty --locked`
- `pnpm test:governance`
- `pnpm check:architecture; pnpm check:boundaries; pnpm release:check`

## Results

Initial partial-frame regression fails once: a completed call cancels the input
read and loses bytes already consumed from its next packet. Retain that read
future across call completion. The exact regression then passes.

The first crate run finds a second failure: select drops the losing host future
before its deadline branch signals cancellation. Pin it outside the select;
exact capacity/deadline acceptance then passes. No assertion or deadline is
weakened. First strict Clippy reports unnecessary unwrap/collapsible-if; use an
explicit match and let-chain, preserving refusal paths. A helper unused by one
test binary is moved to its actual owner, without warning suppression.

Twelve initial acceptance cases and strict Clippy pass. A new stalled-writer
regression then fails: blocking output prevents new calls and EOF cleanup.
Separate an owned bounded writer, maintain input progress, enforce output
capacity/deadline and reap it on teardown. That exact regression passes. The
15 final engine cases cover partial frames, exact string/u64/i64 IDs,
immutable lists, invalid configuration/schemas, unlisted tools, input/output
bounds, duplicate IDs, no queue/replay, concurrent cancellation, deadline,
unknown effects, EOF/drop and blocked-output capacity/deadline.

The initial Cargo archive verifies independently from `target/package`, with
15 files/52.8 KiB (15.4 KiB compressed). Final archive verifies after the writer change: 16 files / 59.3 KiB
(16.6 KiB compressed), independently rebuilt from its packaged source. All 45
source governance cases pass, including release version and archive
orchestration. The 13-crate package inventory passes, with notices/licenses and
private-content exclusion. Strict crate Clippy and source architecture,
boundaries and coordinated release metadata checks pass.

Final archive SHA-256:
`a3e16fea2373e0e967d3e27ebae91eebc306441ba6f6a8e36dd3f6996edef1f7`.
Source logs use `.local/app-tools-{crate-accepted,clippy-final,
package-accepted,inventory-first,spec-check-second}.log`.

## Failures

Three purposeful acceptance failures retain exact logs:
`.local/app-tools-partial-before.log`, `app-tools-crate-first.log` and
`app-tools-blocked-before.log`. The first strict Clippy failure is in
`app-tools-clippy-first.log`. Initial spec validation/check reports four
unmapped new requirements because a blank line split their verification rows
from the actual table. Join them to that table and rerun the original gate.
Later passes do not erase those observations.

## Gaps

Authenticated native IPC, secret-safe launch/debug handling, exact native
admission/registry/approval wiring, actual Codex reads/guarded edits, recovery and
cold process cleanup remain TOOLS-01B/C. The standalone engine is not a native
consumer success. Existing Notes review app, vaults and verified helper bundle
are unchanged. Broad unrelated workspace/compute/provider/platform/release and
visual qualification remain user-deferred; no full-suite claim is made.

Next action: complete the authenticated Notes bridge and connect its existing
AppToolHost to Codex, keeping local transcript files and no implicit retry.

Final source acceptance: all 15 crate cases and strict all-target Clippy pass.
All 45 governance cases, architecture, boundaries for 14 packages, the package
inventory for 13 crates, coordinated release metadata and the complete 10-lane `pnpm
spec:check` pass. The spec check includes formatting, upstream-map integrity,
governance tests, mdBook and exact spec-first. No tests, limits, warning policy,
baselines or original milestone gates are weakened.

Gate result: passed (injectable MCP engine only; native bridge pending).
