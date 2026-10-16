# Qualification and local packaging

Run `pnpm check`, `pnpm test`, `pnpm build` and `pnpm spec:check` for workspace
integration. `pnpm test` separates governance tests from the Rust suite. The
[ledger](../spec/src/current/progress/milestones.md) records actual native and
platform evidence; a successful build is not a native gate.

## Client and native acceptance

```sh
python3 scripts/client-acceptance.py --acp target/debug/lapis-acp --agent target/debug/lapis-agent --tools target/debug/lapis-agent-tools --client target/debug/examples/native-session --allow
python3 scripts/client-acceptance.py --acp target/debug/lapis-acp --agent target/debug/lapis-agent --tools target/debug/lapis-agent-tools --client target/debug/examples/native-session
python3 scripts/local-compute.acceptance.py --compute target/debug/lapis-compute --root .local/native-fresh --cold --acp target/debug/lapis-acp --agent target/debug/lapis-agent --tools target/debug/lapis-agent-tools
```

The first two use a deterministic local inference fixture. The third explicitly
prepares the pinned Mesh native runtime, downloads the configured small GGUF
model into its own private root, serves real inference and requires an approved
ACP tool edit. Use a fresh root for a cold claim. It never changes a user model
cache. The pinned Linux native artifact requires glibc 2.38 or newer; the
qualified container uses Debian Trixie. Debian Bookworm is not a supported native
runtime host for this artifact. Linux CPU runs use an isolated Rust 1.99 Debian container and
`scripts/linux-qualification.sh`; the script is container-only because it
installs system build dependencies. Linux links serially to bound memory use.

The independent Rust compute example is built by `pnpm build`. Qualify its
managed-cache boundary with `python3 scripts/compute-client-acceptance.py --help`:
select its binary, the compute helper, a pinned native runtime, a new private root
and optionally a staged bundle. This exercises TOML, credential references,
resource verification, explicit preparation/download/start, real inference and
supervised teardown through the Rust client.

## Mixed-version compute

Materialize the immutable legacy helper in this repository's ignored scratch:

```sh
python3 scripts/prepare-legacy.py --source /absolute/path/to/jj-store
pnpm dev:cargo -- build --manifest-path .local/legacy-compute/Cargo.toml --target-dir target --locked
```

The script reads the pinned revision through Jujutsu and checks imported source
hashes. It never edits the source repository. Record the legacy executable's
SHA-256 before running `scripts/compute-interop.py --help`. Supply both binaries,
the downloaded native-runtime directory, an explicitly selected model, a new
isolated root and the recorded `--legacy-sha256`. Both directions must prove
mutual admission, sharing-off refusal, model discovery, native inference,
revocation and preserved local inference. ALPN remains `jj-store/compute/1`.

## Staging

```sh
pnpm package:stage -- --profile debug --destination dist/my-platform-debug --native-runtime /absolute/path/to/native-runtime
```

Build the four production helpers first. Release is the default staging profile;
this foundation's recorded local acceptance uses debug binaries. Native runtime
input is optional; omitted runtime requires a later explicit preparation step.
Staging never replaces an existing destination. It copies only named production
helpers, notices/licenses and the explicitly selected native runtime. The TOML
manifest records target, package version, helper/native-file SHA-256, upstream
pins and native ABI 0.1.44. Model weights and test binaries are excluded.

Run client acceptance with the staged helper paths and `--bundle dist/...` to
exercise the Rust bundle verifier. Then run native acceptance using staged helper
paths, a fresh root and `--native-runtime dist/.../native-runtime`. The bundled
preparation check requires the native download cache to stay empty. Model download
remains explicit. Resource tests independently reject checksum corruption,
foreign targets, missing native manifests, ABI mismatches and escaping paths.

The workflow in `.github/workflows/ci.yml` defines deterministic, native and staging
lanes. Hosted results must be observed in GitHub Actions and recorded in the ledger;
local platform qualification does not establish a hosted result. Cargo/native
release automation is described in [releasing](releasing.md). Signing, installers,
live cloud credentials, third-party installed adapters and Tauri integration
remain separate acceptance scopes.
