# Typed ACP renderer client

Revision: based on `1f3de80d`, change `qrnuosyp`.

Milestone review: the private renderer source slice passes. Aggregate Tauri
transport and desktop M10.1 remain In progress until cold packaged WebView
qualification. M5 publication authorization is unchanged.

## Commands

- `pnpm build:client`
- `node --test --test-name-pattern='a send detached before dispatch' packages/ai/tests/client.test.mjs`
- `node --test packages/ai/tests/tauri.test.mjs`
- `pnpm test:client`
- `node --test --test-name-pattern='real Changesets' scripts/changesets.test.mjs`
- `node --test scripts/changesets.test.mjs`
- `pnpm check`
- `pnpm test`
- `pnpm build`
- `pnpm package:check --allow-dirty`
- `pnpm spec:check`
- `pnpm check:workflow`

## Results

The private `@lapismd/ai` package builds its root and optional `./tauri` public
exports. Generic code imports no native runtime. **27 renderer tests pass**
against built output: early channel/submit replies, exact custom permission IDs,
null cancellation, lost open recovery through list, no uncertain replay, native
refusals, pending-request teardown, cursor retention, subscription fencing,
coverage gaps, identity/version/order checks, count/byte bounds, failed consumer
and acknowledgement, response deadlines and the public Tauri command wire.

The full workspace passes **969 Rust tests, zero failures, two inherited
ignored cases**, plus **42 governance tests, zero failures**, and the 27 client
cases. Strict checks, TypeScript, workspace/examples build and specification
lanes pass. All **11 Cargo publication inventories** pass. The private npm dry
run contains **22 files**, including both JS/declaration entrypoints and legal
notices, excluding sources/tests/dependencies. No package is published.

The real coordinated-version fixture passes **4 cases**, including synchronized
private client/Cargo versions and untouched canonical evidence. Root workflows
run client checks/tests/builds and retain Cargo gates. Linux CI installs the
native GTK/WebKit prerequisites introduced by the Tauri dependency; this does
not claim a new qualified platform.

## Failures

- The additive client target initially did not exist; there was no prior client
  regression to run. This is absence of implementation, not a product failure.
- `a send detached before dispatch cannot launch deferred work` first failed
  **0 passed / 1 failed**: a queued microtask still invoked after detach.
  Dispatch now checks the current owner and lifecycle. The exact regression
  passed **1 passed / 0 failed**, then all 27 client cases passed.
- The public Tauri wire test initially failed because its Node test fixture
  omitted `window.crypto`. Supplying the standard Web Crypto fixture made the
  exact test pass. Production transport required no workaround.
- The first real Changesets workspace test had **3 passed / 1 failed** because
  the new workspace patterns omitted the existing root coordinator. Explicitly
  including the root restored the exact regression (**1 passed / 0 failed**)
  and the full four-case version suite. No release was prepared in this checkout.
- The inherited compute unwind-table linker warning remains; build succeeds.

## Gaps

Deterministic transport/public-build tests do not qualify a WebView, real
provider/model, desktop policy/UI, app tools, credentials, compute or another
platform. LAI-TAURI-004–006 remain Planned until the native consumer gate.
Client detach retains native workers. Requests with unknown outcomes are never
replayed. App-owned persistence/context/memory remain consumer work.

Next action: consume these verified built public exports and the Rust plugin in
cold packaged desktop acceptance, then continue the desktop-owned AI migration.

Gate result: passed for renderer source; aggregate transport pending native gate.
