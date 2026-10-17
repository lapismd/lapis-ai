# Packaged ACP Tauri qualification

Revision: based on `e128baaf`, change `sztkorno`; consuming desktop
`1362e3b0` records immutable `spec/src/history/E-AI-002.md`.

Milestone review: the reusable ACP Tauri transport gate passes on macOS ARM64.
Desktop M10.1 is Done. M5 hosted release authority and desktop M10.2-M10.4
application gates are unchanged.

## Commands

- Source: `pnpm check`, `pnpm test`, `pnpm build`,
  `pnpm package:check --allow-dirty`, `pnpm spec:check`.
- Desktop: `pnpm test:desktop`, `pnpm check`, `pnpm test`,
  `pnpm test:storybook`, `pnpm build-storybook`, ordinary bundle and strict audit.
- Desktop refresh at `e128baaf`: prepare native resources, strict e2e Clippy,
  cold actual Tauri bundle and `node --test tests/desktop/acp-transport.test.mjs`.
- Documentation promotion: `pnpm spec:check`.

## Results

Committed source qualification passes **973 Rust**, **42 governance** and
**27 renderer** cases, with zero failures and two inherited Rust ignores.
All 18 Tauri Rust cases, 11 Cargo inventories and 22 private client inventory
files pass. The source records retain exact cleanup failure regressions.

The desktop's complete acceptance/build boundary passes **119 native cases**,
**85 JavaScript/governance**, **77 Rust** and **84 catalog interaction/axe** cases.
Desktop Rust's two ignored subprocess helpers are exercised by their real tests.
Catalog static build and ordinary ARM64 bundle audit pass. The ordinary artifact
has 356 renderer JavaScript files, zero acceptance markers and no test resources.

After the AI-only owned-retirement correction, the complete directly affected
**six-case** ACP native lane was rebuilt and passed against `e128baaf`. It uses
actual packaged WebViews, the built public `@lapismd/ai/tauri` export, the native
plugin, actual `lapis-acp` and a compiled deterministic Rust ACP provider. It
covers exact permissions/context/order, cancellation, unknown outcomes without
replay, retained reload, scoped revocation/disable, independent workers,
second-WebView rejection, cancelled close retention and actual-exit reaping.
The application process has no Node/Deno on PATH.

LAI-TAURI-004-006 now have native consumer evidence. App policy, conversations,
context, memory and tools stay outside this reusable source component.

## Failures

The immutable desktop record preserves initial reload/provider-fixture failures,
the stale generated acceptance-permission metadata caught by the ordinary audit,
and their exact passing repairs. The source owned-retirement record preserves
**0 passed / 2 failed** before correction, then four passing retirement cases.
No assertion, guard, permission scope, visual threshold or baseline was weakened.
Inherited linker, Node and Vite/Storybook notices remain recorded separately.

## Gaps

Qualification is macOS ARM64 with a deterministic compiled ACP provider, not a
real model/provider, desktop AI UI, app-tool/credential handover, compute Tauri
integration, another platform or release. The private client remains a local
public-build prerequisite. No package/tag is published and signing is deferred.

Next action: the desktop continues M10.2 UI/conversations/context/memory and
M10.3 native app tools/credentials; optional compute retains M10.4.

Gate result: passed for the reusable ACP Tauri transport.
