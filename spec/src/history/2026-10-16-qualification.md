# Native, platform, interoperability and staging qualification

Evidence: E-AI-QUALIFICATION-001.
Revision: enclosing Jujutsu change `srssqlou`.
Requirements: LAI-LOCAL-001, LAI-COMPUTE-001, LAI-COMPUTE-002,
LAI-PACKAGE-001, LAI-PACKAGE-002, LAI-RUNTIME-001, LAI-RUNTIME-002,
LAI-RUNTIME-003, LAI-RUNTIME-004, LAI-GOV-103, LAI-GOV-104.
Milestone review: M4 qualifies the profiles below; M0–M3 retain their separately
recorded [foundation acceptance](2026-10-16-foundation.md).
Gate result: passed for M4.

## Profiles and immutable inputs

- macOS 27.0 ARM64, Rust 1.99.0 (`b940084d7`), Node 26.9.0, pnpm 10.34.5.
- Linux aarch64 CPU in isolated Rust 1.99.0 Debian Trixie containers. Image:
  `rust@sha256:81015672d6939b0265f09adf96d5de125482bb1330106076956b0518e74edc90`.
  The container's glibc is 2.41; this pinned native artifact requires at least 2.38.
- Mesh source: `9f192c9d821991ba43d79bb15fd2f2f12e597571`, Mesh version
  `0.76.0-rc9`, Skippy ABI `0.1.44`. Native backends: darwin-aarch64 Metal and
  linux-aarch64 CPU. Vendor manifests retain all native file hashes and patches.
- Real inference model: `unsloth/Qwen3-0.6B-GGUF:Q4_K_M`, GGUF SHA-256
  `ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a`.
- Pristine legacy helper: JJ Store source revision
  `01ea0741ff608bbe14daa8d929448e3ec0790afd`, original compute Cargo.toml,
  Cargo.lock and seven source files, materialized under `.local/legacy-compute`.
  Executable SHA-256:
  `a8f2d0ff7399981802cdd54114ebe81d1fceee5a19c2b026d05c413a8426dd3d`.
  The original helper source and lock were not changed for acceptance.

## Commands

Commands ran from the new repository. Paths below identify ignored isolated test
roots, never an installed service or user model cache.

```sh
python3 scripts/local-compute.acceptance.py --compute target/debug/lapis-compute --root .local/macos-native --acp target/debug/lapis-acp --agent target/debug/lapis-agent --tools target/debug/lapis-agent-tools
python3 scripts/local-compute.acceptance.py --compute dist/macos-arm64-debug/lapis-compute --root .local/macos-bundled-native --native-runtime dist/macos-arm64-debug/native-runtime --acp dist/macos-arm64-debug/lapis-acp --agent dist/macos-arm64-debug/lapis-agent --tools dist/macos-arm64-debug/lapis-agent-tools
pnpm dev:cargo -- build --manifest-path .local/legacy-compute/Cargo.toml --target-dir target --locked
python3 scripts/compute-interop.py --lapis target/debug/lapis-compute --legacy target/debug/jj-store-compute --legacy-sha256 a8f2d0ff7399981802cdd54114ebe81d1fceee5a19c2b026d05c413a8426dd3d --native-runtime .local/macos-native/native-runtime/0.76.0-rc9/meshllm-native-runtime-darwin-aarch64-metal --root .local/interop-hf
pnpm package:stage -- --profile debug --destination dist/macos-arm64-final --native-runtime .local/macos-native/native-runtime/0.76.0-rc9/meshllm-native-runtime-darwin-aarch64-metal
python3 scripts/client-acceptance.py --acp dist/macos-arm64-final/lapis-acp --agent dist/macos-arm64-final/lapis-agent --tools dist/macos-arm64-final/lapis-agent-tools --client target/debug/examples/native-session --catalog-client target/debug/examples/agent-catalog --bundle dist/macos-arm64-final --allow
python3 scripts/compute-client-acceptance.py --client target/debug/examples/native-compute --compute dist/macos-arm64-v1/lapis-compute --native-runtime dist/macos-arm64-v1/native-runtime --root .local/rust-bundle-final --bundle dist/macos-arm64-v1
```

Source materialization used `jj file show -r PIN runtime/compute/PATH` for every
original file and the committed lock. `scripts/prepare-legacy.py` provides the
same reproducible materialization with source and lock checksum verification.
The legacy artifact hash was checked before interop startup.

Linux full qualification used `docker run --rm --name lapis-ai-linux-final`
with `CARGO_HOME=/cargo`, mounts of this repository at `/work` and its ignored
`.local/linux-cargo` at `/cargo`, workdir `/work`, image `rust:1.99.0-trixie`,
and `sh scripts/linux-qualification.sh`. Environment
`LAPIS_AI_NATIVE_ACCEPTANCE_ROOT=/work/.local/linux-cold-final` selected a new
private root. This script built the workspace/examples, ran the workspace suite,
ran client allow/deny and cold native inference plus approved ACP editing.

A subsequent isolated `lapis-ai-linux-bundle` container with the same mounts and
image ran the following after installing Node, Python, pkg-config and libssl-dev.
`CARGO_TARGET_DIR=/work/.local/linux-target`, `CARGO_BUILD_JOBS=1`:

```sh
node scripts/managed-cargo.mjs build --workspace --locked
node scripts/managed-cargo.mjs build -p lapis-ai-runtime --examples --locked
node scripts/managed-cargo.mjs test -p lapis-ai-runtime -p lapis-ai-config --locked
node scripts/stage.mjs --profile debug --destination dist/linux-aarch64-v1 --target-dir .local/linux-target --native-runtime .local/linux-cold-final/native-runtime/0.76.0-rc9/meshllm-native-runtime-linux-aarch64-cpu
python3 scripts/client-acceptance.py --acp dist/linux-aarch64-v1/lapis-acp --agent dist/linux-aarch64-v1/lapis-agent --tools dist/linux-aarch64-v1/lapis-agent-tools --client .local/linux-target/debug/examples/native-session --catalog-client .local/linux-target/debug/examples/agent-catalog --bundle dist/linux-aarch64-v1 --allow
python3 scripts/compute-client-acceptance.py --client .local/linux-target/debug/examples/native-compute --compute dist/linux-aarch64-v1/lapis-compute --native-runtime dist/linux-aarch64-v1/native-runtime --root .local/linux-rust-bundle --bundle dist/linux-aarch64-v1
```

Focused native diagnostics also ran
`pnpm dev:cargo -- test -p lapis-compute
native_installed_model_starts_and_serves_without_public_discovery --locked --
--ignored --nocapture` with explicit private test roots/bundles. Selected imported
GGUF diagnostics used `LAPIS_AI_NATIVE_TEST_MODEL`; later direct test-binary runs
used a private HOME and the same exact imported ID. These are separate from the
cold helper and platform acceptance above.

## Results

macOS cold preparation/download/start, real inference and the exact approved ACP
file write passed. Bundled-runtime acceptance passed from a fresh root and left
the native download cache empty. Model download remained an explicit operation.
The independent Rust compute client passed actual native inference after the
managed-cache fix. Agent discovery and single-document metadata passed through
the reusable Rust client and actual native helper using a fixture provider.

Linux full deterministic qualification passed 931 tests with two explicit ignored
tests. The subsequent affected-crate run passed 17 configuration/client tests,
including the new EOF-framing regression added after that full run. The macOS
final full run passed 932 Rust tests and 18 governance tests. Linux cold native
CPU inference and approved ACP file editing passed with the unchanged terminal
completion, permission and file-content assertions. No fixture inference was
substituted for either native platform gate.

Both `lapis-compute -> jj-store-compute` and the reverse passed authenticated
QUIC model discovery and real native inference. They refused missing mutual
admission and sharing-off requests, refused a revoked peer, and preserved local
inference after revocation. This qualifies the retained `jj-store/compute/1`
wire profile without editing or deploying JJ Store.

The macOS and Linux bundles contain exactly four production helpers, native
resources, manifests and notices, including maintained Buzz, Mesh Apache and
pinned llama.cpp MIT texts. Model weights and test executables are absent.
Rust bundle loading and native client acceptance verified target, inventory,
checksums, paths and native ABI. Final macOS notice-only restaging retained the
exact previously qualified helper and native-file hashes.

Manifest SHA-256:

- `dist/macos-arm64-final/runtime-manifest.toml`:
  `b74d6e05beb622aa7427658b5d8aacf99228e725e82eaf46b65cd1404a7111ce`.
- `dist/linux-aarch64-v1/runtime-manifest.toml`:
  `3a29fa80584d2b1053761ac0cfbaf6fb15250b98ad88ed310af5af1d1cc9c735`.

## Failures and observations

- The first Linux parallel test link was killed for memory pressure. Serial
  linking passed; the Linux launcher now uses one Cargo build job.
- Debian Bookworm deterministic tests passed, but native loading failed because
  its glibc 2.36 lacks `GLIBC_2.38`. Trixie passed actual loading/inference. This
  is a pinned artifact host prerequisite; no dependency or loader was patched.
- An initial warm Trixie native-agent turn reached permission but did not finish
  successfully. Its terminal classification was not captured. The subsequent
  fresh cold qualification passed without changing the completion assertion or
  the 180-second turn bound. Native model output remains nondeterministic.
- Two initial interop attempts using an imported-only GGUF failed legacy helper
  startup. Explicit Lapis test diagnostics with the same imported bytes passed,
  including the 4 GiB limit. The qualified mixed-version profile uses an explicit
  named-model download; imported-only legacy startup remains unqualified.
- macOS linking reports an inherited oversized `__eh_frame` warning. It did not
  prevent workspace checks, tests or real inference. It remains an observed
  native dependency/toolchain warning rather than a suppressed gate.
- Initial resource staging lacked recursive directory-copy configuration;
  staging was corrected before successful artifact acceptance.
- Initial spec evidence used a requirement-shaped evidence identifier and omitted
  its navigation entry. A distinct evidence prefix and SUMMARY entry corrected
  the document checks.

## Gaps

Qualification is macOS ARM64 Metal and Linux aarch64 CPU with the pinned small
model, debug helpers and native ABI. No x86 platform, other model/backend, release
build, distribution signing or installer is qualified. Native output is
nondeterministic; the earlier failed Linux turn and imported-only legacy gap are
retained above. Hosted CI is configured but has not run. Tauri adapters remain a
contract. Cloud providers, streaming inference, cross-application daemons,
consumer migration and public release remain separate work.

Next action: integrate an explicitly admitted application adapter in a separate
change, and qualify broader native/model/release profiles before distribution.
