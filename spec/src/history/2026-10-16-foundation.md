# Shared native AI foundation

Evidence: E-AI-FOUNDATION-001.
Revision: enclosing Jujutsu change `srssqlou`.
Requirements: LAI-GOV-100, LAI-GOV-101, LAI-GOV-102, LAI-GOV-103,
LAI-GOV-104, LAI-CONFIG-001, LAI-CONFIG-002, LAI-CONFIG-003,
LAI-RUNTIME-001, LAI-RUNTIME-002, LAI-RUNTIME-003, LAI-RUNTIME-004,
LAI-LOCAL-001, LAI-COMPUTE-001, LAI-TAURI-001, and imported ACP requirements.
Milestone review: M0, M1, M2 and M3 satisfy their foundation gates. M4 remains
separate platform, native inference, interoperability and staging qualification.
Gate result: passed for M0–M3.

## Scope and contracts

The standalone workspace contains eleven unpublished packages and four production
helpers. The six ACP packages were renamed without changing their Control V1–V3
wire formats or legacy environment namespaces. Agent composition retains the
maintained Buzz fork, metadata/model discovery, provider adapters, permissions,
private token-source IPC and Apache notices. Core-agent tests use a distinct
fixture binary so product discovery does not change their tested contract.

Compute retains `jj-store/compute/1`, invitations, model controls, authenticated
loopback HTTP and admitted inference-only peers. Root configuration is inert,
strict version-1 TOML. JSON wire/state and catalog compatibility inputs remain.
TOML catalogs and MCP declarations resolve file-relative executables and secret
references; unknown runner fields and alias collisions fail closed. Application
trust, context, conversation state and credentials remain consumer-owned.
Tauri is a documented future adapter contract; no implementation or consumer
migration is included. No shared AI controller is introduced.

Supervised clients own bounded subprocess transport, readiness, exact identities,
permission choices, environments and teardown. Unknown outcomes never trigger
prompt replay. Compute HOME, XDG, Hugging Face and Mesh caches follow the private
managed compute directory. Generic subprocess output is bounded at 4 MiB/256
frames, ACP requests at 1 MiB and compute control at 65536 bytes including newline.
Resource loading checks target/version, complete helper inventory, paths, hashes
and optional native ABI. No credentials appear in Debug output or diagnostics.

Imported ACP contracts retain their requirement suffixes under the LAI prefix.
Source publication and historical cloud-runner qualification are explicitly
separated from this repository's results. The frozen twenty-file exception set
is backed by original file hashes/counts; reductions are retained and no baseline
was raised. New Rust/script files are limited to 300 counted lines and tests 500.
Specification Markdown uses 400 physical lines and 32768 bytes.

## Commands

Initial source import commands and pins are recorded in
[import evidence](2026-10-16-import.md). Source bytes and original lock hashes are
recorded in `docs/imports/source-manifest.json` and `lock-inputs.json`.

The following commands were run from the new repository; Cargo always used
`pnpm dev:cargo --` and locked resolution where applicable:

- `pnpm install --frozen-lockfile`.
- `pnpm dev:cargo -- check --workspace --locked`.
- `pnpm dev:cargo -- test -p lapis-ai-config --locked` and
  `test -p lapis-ai-runtime --locked`.
- `pnpm dev:cargo -- test -p lapis-acp-agent --test toml_catalog --locked`.
- `pnpm dev:cargo -- test -p lapis-compute --test control --locked`.
- `pnpm dev:cargo -- test -p lapis-ai-runtime --test lifecycle --locked`.
- `pnpm dev:cargo -- test -p lapis-ai-config
compute_launch_keeps_model_caches_inside_its_managed_working_directory --locked`.
- `pnpm dev:cargo -- clippy --fix --allow-dirty --allow-no-vcs --workspace
--all-targets --locked -- -D warnings`, followed by ordinary non-fix clippy.
- `pnpm format`, `pnpm check`, `pnpm test`, `pnpm build`, `pnpm spec:validate`,
  `pnpm spec:first` and `pnpm spec:check`.
- `python3 scripts/client-acceptance.py --acp target/debug/lapis-acp --agent
target/debug/lapis-agent --tools target/debug/lapis-agent-tools --client
target/debug/examples/native-session`, once with `--allow` and once without.

The final full integration scope is required because this is a new shared
workspace spanning the ACP, native services and client packages. Earlier exact
regressions were run first. Development logs under ignored `.local` support the
record; they are not the durable evidence authority.

## Results

Workspace build, non-fix clippy, formatting, architecture, boundaries, progress,
upstream pin self-test, governance tests and specification checks passed. The
specification defines 78 requirements. The final integration suite passed 932
Rust tests and 18 governance tests, with two explicitly ignored Rust tests. The integration suite retains
provider-free deterministic evidence; platform and native claims live in the
separate qualification record.

Client acceptance proves TOML selection, native-agent launch, exact permission
approval/denial and file-write gating through a separate Rust consumer. Runtime
regressions prove environment/cwd isolation, malformed readiness rejection,
process reaping, exact option IDs, frame limits, no replay on timeout, latched
output coverage loss, compute command bounds and loopback readiness. Agent discovery/metadata acceptance uses the actual native helper and tests
bounded single JSON documents at EOF separately from ACP newline framing. Resource
regressions reject corrupt content, foreign platforms, wrong ABI/missing native
manifest and symlink/path escape. Helper regressions prove startup configuration,
HTTP authentication, no startup model downloads and immediate next-command use.

## Failures and corrections

- Fresh Mesh dependency resolution failed because all `zip ^2` releases were
  yanked. The committed original compute lock seeded resolution; the final
  unified lock preserves that historical resolution and additional workspace
  packages. No dependency source, version pin or safety check was patched.
- Combining source locks initially exposed different Tokio resolutions; that
  approach was discarded in favor of the pinned compute lock seed.
- Original agent test assumptions conflicted with the new product wrapper.
  A separate core fixture preserves those assumptions. A copied Rust-2021 test
  used let-chains; equivalent conditional syntax preserved its assertions.
- macOS temporary paths differed through `/var` and `/private/var`; the isolation
  regression now compares canonical paths.
- Formatting exceeded one imported LLM file's frozen count by four lines.
  Two unchanged classifiers were extracted; the exception was ratcheted down.
- The TOML reader initially changed a JSON diagnostic; the original JSON parser
  and its regression were restored.
- The Rust ACP example initially omitted prompt duration and then execution-mode
  configuration. It now follows the actual V3 warm/open/configure/prompt sequence;
  both allow and deny acceptance pass.
- The output-overflow test initially expected a successful helper exit after its
  output pipe closed. It now checks latched coverage loss and accepts only the
  corresponding success or classified BrokenPipe helper exit during teardown.
- The Rust compute example rejected a download because HOME/cache paths were
  outside its managed cwd. The exact cache-boundary regression failed before
  deriving private cache paths and passes afterward, as does native Rust-client
  inference. A trailing-separator mismatch was fixed without weakening the test.
- The actual agent metadata helper emits one JSON document without a trailing
  newline. The initial newline reader reproduced `helper_protocol_failed`.
  Agent clients now use bounded single-document framing; the actual helper
  acceptance and the EOF/extra-document regression pass. ACP framing is unchanged.
- Progress validation initially rejected two recorded inputs composed into one
  agent destination; it now rejects duplicate source identities while allowing
  that documented composition.
- Spec-first initially excluded the exact `governance.md` name. Its canonical
  pattern and per-layer ownership mappings were corrected; the gate passes.

## Gaps

Node and Python are development tools, not shipped runtimes. Cloud credentials,
installed third-party ACP adapters, hosted CI, release builds, signing,
publication and Tauri implementation remain outside this foundation. The two
explicitly ignored tests require provider or prepared-native prerequisites and
are not silently counted as deterministic passes.

Next action: qualify and record M4 independently, then adopt these libraries in a
separately authorized application integration.
