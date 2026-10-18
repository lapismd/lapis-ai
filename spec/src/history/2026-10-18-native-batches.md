# Deferred native text groups

Revision: enclosing Jujutsu change `sxqvvslu`, based on `87958267`.

Milestone review: NOTES-06C2 adds native transport for every frame returned by
an injected capture batch. The source runtime, scoped factory and typed client
share one acknowledgement unit. Focused and full source qualification pass; cold
packaged batch acceptance remains pending. M0–M5 and publication authorization
are unchanged. Notes privacy/storage adoption remains separate.

## Commands and results

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-runtime --test acp_batches --locked`
- `node --test packages/ai/tests/batches.test.mjs`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- fmt --all`
- `pnpm build:client && node --test packages/ai/tests/batches.test.mjs`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --test batches --locked`
- `pnpm check:architecture`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-ai-runtime --tests --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p tauri-plugin-lapis-ai --tests --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm check`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm test`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm build`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm package:check --allow-dirty`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm spec:check`
- `node --test --test-name-pattern='tail on non-text content' packages/ai/tests/batches.test.mjs`

The initial before target includes a fixture-only E0277 from `Value::repeat`;
fixed that before production edits. The completed final baseline reports 16
missing-capability errors (E0432/E0407/E0046/E0609/E0599): batch exports,
callback, group fields and whole-event iteration are absent. The renderer before
target fails because its public helper export is absent. The first production
attempt reports E0596 on a non-mutable iterator; fixed it without changing any
assertion. Final focused results: **seven runtime**, **two scoped-host** and
**twelve renderer** cases pass. The additional exact non-text-tail case first
fails because client validation accepted a tail attached to non-text primary
content. Require the same text shape, then rerun it and the full client lane;
no assertion was weakened.

One preserved primary frame and bounded deferred text form a native event group.
Native event cursors still count validated incoming events. Before-text precedes
the current primary frame. Facts-triggered text appends to the preceding startup
event before the session becomes observable, without allocating a facts cursor.
Default processors retain their single-frame callback and default event JSON
omits empty group fields.

Derived frames must match the previous sanitized text identity and carry no
invented provider sequence. They cannot introduce a permission or terminal
outcome. All payload bytes count toward existing retention bounds. Invalid
identity, count or capacity refuses the entire group; terminal failure retains
the original unresolved prompt without replay or raw fallback.

Scoped factories recheck current admission around the complete batch callback.
Revocation during a blocked callback withholds the entire group and leaves an
independent window usable. Startup cancellation and cancelled close waits retain
their supervised drain. The typed client validates every group member before
calling its consumer; consumption must finish the complete order before native
cursor advancement or acknowledgement. The public iterator retains a frame
index for application provenance.

Logs live under jj-store `.local/reusable-conversations/native-batches-*.log`.
Fixture capture files are synced test records, not JJ publication or power-loss
evidence. No runtime knows a Notes filesystem journal, privacy policy or store.

## Full source qualification

The runtime suite passes **50 cases** and scoped Tauri host **29**. Complete
source qualification passes **1,013 Rust tests**, **two inherited ignored**,
**45 governance** and **39 renderer** cases. Strict Clippy/TypeScript/check,
workspace/example builds, 12 Cargo package inventories, the 24-file private
renderer inventory and all ten specification lanes pass. Validation covers
five validators, 34 chapters and 95 requirements.

The first aggregate specification attempt reports two missing mapped canonical
chapters, runtime and governance. Added their new batching boundary and repeated
the complete source qualification. The original failure remains in
`native-batches-spec.log`; final lanes use `native-batches-*-final.log`.
The imported compute linker warning and two inherited ignored native/corpus
cases remain distinct from passing tests. No cleanup or user-cache changes.
Shared runtime, host and client changes require these complete source lanes;
source fixture success does not replace the downstream cold gate.

Final document commands: `pnpm spec:validate && pnpm check:progress && pnpm
spec:first`; selected Markdown formatting also passes. Five validators,
34 chapters and 95 requirements; 186 retained import records; spec-first covers
18 protected files and three canonical chapters. LAI-TAURI-008's stale Planned
verification row now reflects its independently qualified factory evidence.

## Gaps and next action

Exercise actual packaged WebView batch
serialization and acknowledgement in the isolated desktop consumer. Connect
Notes-owned privacy/durable storage through this port afterward. Preserve
AgentStore/ThreadStore, unknown-write recovery, native managed conversation,
provider/platform, unattended activation, visual and release gaps separately.

Gate result: passed for the shared runtime/host/client source boundary; cold
native batch qualification and Notes privacy/durability remain pending.
