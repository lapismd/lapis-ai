# Docker portable validation

Date: 2026-10-25. Scope: LAI-GOV-100–105 development tooling. No runtime public
API, provider, model, execution backend or original M0–M5 gate changes.

## Source and workflow

The sole default workspace selects the existing neutral ACP/execution source.
An origin fetch observes only the new v0.1.1 tag: 24 selected source commits are
ahead of main, with no upstream-only changes. No rebase is needed.

Twenty-four portable root lanes default to bounded Docker workers. Reviewed
native exceptions preserve Cargo-host preparation, staging, formatting and
Changeset authoring. Provisioned CI and native qualification explicitly select
native execution. Desktop's `prepare-ai.mjs` already uses the retained
`dev:cargo` and `package:stage` exceptions, so its artifact paths remain intact.

Workers transfer the exact tracked Jujutsu revision/ancestry with SHA-256,
reject private/model paths recursively in that tree and history, install the
frozen registry lock without local hooks, and fetch locked Cargo dependencies.
Only an owned cache volume is mounted. There are no model/credential/sibling
inputs, Docker socket mounts or automatic host fallbacks. This repository has
no Storybook, so no catalog or browser-specific guards are invented here.

Tool-image keys include only recipe/install inputs, selected tool versions and
architecture. Dependency/native cache keys are separate. The active tool image
is `2935940257ca17437e3d`; its cache is `c5a41267f66ea1410e8c`. Each worker has
4 CPUs, 12 GiB memory, 2 GiB shared memory and one Cargo job, with an init
reaper and all Linux capabilities dropped. Reports retain exact input hashes,
versions, logs and source identity; native targets/models are not copied back.

## Before and after

| Boundary            | Exact command or case                                 | Before                                                           | After                                                                 |
| ------------------- | ----------------------------------------------------- | ---------------------------------------------------------------- | --------------------------------------------------------------------- |
| Routing             | `node --test scripts/validation.test.mjs`             | One failed file: absent runner module                            | Nine passes initially; ten after key separation                       |
| Image reuse         | Tool-image/dependency-key regression in the same file | Nine passes, one failure                                         | Ten passes, zero failures                                             |
| Worker inventory    | `pnpm validation:smoke`                               | One failure: regex counted dependency paths as workspace members | TOML workspace inventory: 15 packages, smoke passes                   |
| Offline preparation | Real Changesets prepare case in `pnpm validation:all` | Governance 55/56; missing locked wasm-bindgen target dependency  | Explicit locked fetch: governance 57/57                               |
| Descendant cleanup  | CLI `cleanup.rs` in the full worker                   | Four passes, two cleanup failures with Node as PID 1             | `pnpm test -- --test cleanup`: six passes with Docker init            |
| Unix permissions    | Mode-000 directory read probe                         | One failure: root worker bypasses directory permission           | Restricted worker denies both read/write; effective capabilities zero |

The original full run `b98b51e3-86e4-4cb1-82c4-ef1d6ced474d` passed Clippy
and 43 renderer cases before the offline prerequisite failure. The rerun
`84070cc1-ebf5-4c81-bac9-b5de0810be33` passed all 57 governance cases and
43 renderer cases, then exposed the two descendant-cleanup failures. Product
code and assertions remain unchanged; the runner receives the missing setup.

Restricted smoke `aef1ff7c-727e-43e4-b7b0-7cc552f1c57e` proves init,
permission denial, ten runner cases and all 15 Cargo workspace identities.
Exact cleanup `b614ea50-8c65-4ee3-bb63-5fa735555e7a` passes all six cases.
Owned cleanup collected/removed only the two earlier completed failed workers;
reports remain in `.local/validation/runs`. Unknown/prepared/active runs retain
leases, and cleanup uses exact container IDs to avoid releasing a successor.

## Final qualification

The complete restricted `pnpm validation:all` run
`21da29d8-fe8c-4df3-8667-ca1708a5e50a` passes from
2026-10-25T16:54:34.832Z to 16:56:56.661Z (2m21.8s with warm dependencies).
Its source revision is `db7198621bf42666d63f107fa684a49ffb919deb`; tool image
ID is `sha256:68f28bef97b28deae2d12487527583d936a88ce9668a51b5b5b5b01a5651b5ec`.

- Strict check/Clippy, formatting, architecture, boundaries, release/progress and
  upstream checks pass with no findings.
- All 1,076 Rust cases pass, with zero failures and two inherited ignored cases:
  the corpus writer and explicit prepared-native/GGUF model qualification.
- Renderer cases pass 43/43 and governance passes 57/57, including ten runner
  regressions. The repeated specification lane is not counted twice.
- Client, Cargo workspace and native-client examples build. Workflow lint and
  public package contents pass: 24 client files and 14 publishable Rust crates.
- Specification checks pass all 11 lanes, including spec-first and mdBook;
  client dist/spec HTML, exact input/tool receipts and logs are collected.

Tools: Node 24.15.0, pnpm 10.34.5, Jujutsu 0.45.1, Rust/Cargo 1.99.0,
mdBook 0.5.4 and actionlint 1.7.12. Final documentation passes default Docker
`pnpm spec:check` (all 11 lanes and 57 governance cases), run
`e4b24644-9b15-40e9-8fb5-b85b39615131`. No functional, interaction or
accessibility gate is waived.
Native Metal/C++ inference, provider, native Tauri, actual model acceptance,
broader platforms and release publication retain separate gates.
