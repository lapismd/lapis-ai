# E-RELEASE-SETUP-001: public repository and release automation

Revision: enclosing Jujutsu change `zvxxoukm`.

Milestone review: M0–M4 retain their prior results. M5 is In progress until
hosted CI, Changesets and Pages have been observed. Actual Cargo/native
publication remains deliberately unexecuted; the initial version PR must stay open.

## Scope and contracts

LAI-REL-001–007 add public hosting, exact-source CI, coordinated version PRs,
protected Cargo/native release and canonical mdBook deployment. LAI-LOCAL-001,
LAI-RUNTIME-004 and ACP requirements retain provider, permission, wire and
unknown-outcome behavior. Cargo metadata, notices and development-only binary
features change packaging, not production behavior. Embedded agent JSON inputs
move into the Cargo package with identical source hashes. No frozen source-import
budget grows, and all upstream/runtime pins remain unchanged.

Ten crates are admitted to crates.io; `lapis-compute` stays native-only because
its pinned Mesh `0.76.0-rc9` crates are absent from crates.io. Native bundles
retain macOS ARM64 Metal and Linux ARM64 CPU/glibc 2.38+ profiles. The npm
coordinator stays private. No release tag or registry publication is performed.

## Commands

- Inspected Jujutsu status, parent/repository guides, canonical contracts and
  pinned `jj-acp` release workflows/scripts at
  `47946b870b18652964d94c0af110ea177c66a80c`.
- `pnpm install --ignore-pnpmfile`; `pnpm format`.
- `pnpm release:check`; `pnpm package:check --allow-dirty`.
- `node --test scripts/changesets.test.mjs scripts/ci-automation.test.mjs scripts/registry-check.test.mjs scripts/release-orchestration.test.mjs`.
- `pnpm dev:cargo -- fetch --locked`; `node --test scripts/changesets.test.mjs`.
- `node --test scripts/release-identity.test.mjs`.
- `pnpm check:workflow`; `pnpm check:architecture`; `pnpm check:progress`.
- `pnpm spec:validate`; `pnpm spec:first`; `pnpm spec:build`.
- `pnpm check`; `pnpm test`; `pnpm build` (complete integration is required
  because release/test infrastructure and metadata span all packages).
- `pnpm dev:cargo -- test -p lapis-agent --lib model_capabilities --features test-support --locked`.
- `pnpm dev:cargo -- publish --workspace --exclude lapis-compute --locked --dry-run --allow-dirty`.
- `pnpm test:governance` (final 41-test suite); `bash scripts/native-qualification.sh debug` (cold local qualification running during first push).
- Compared SHA-256 of both relocated JSON inputs with immutable import records.
- `gh repo create stevejuma/lapis-ai --public` with description and Pages homepage;
  `jj git remote add origin git@github.com:stevejuma/lapis-ai.git`.
- GitHub API configured read-only default Actions tokens, bot PR creation,
  disabled auto-merge and a maintainer-reviewed, version-tag-only `release`
  environment. No release credential was configured.

## Results

Release metadata, package inventory, workflow lint, architecture, import pins,
canonical validation, spec-first mapping and local mdBook build passed.
The actual Changesets fixture advances all eleven packages to the next patch,
updates locks and internal ranges, consumes notes and preserves canonical evidence.
Offline bootstrap failure preserves both notes and version metadata. Narrow bot
exceptions retain canonical, packaging and product gates. Identity regressions
reject differing tags/checkouts and non-manual/non-initial bootstrap attempts.

Complete integration passed: 932 Rust tests, zero failures, two explicitly
ignored tests; 41 governance tests, zero failures. Full checks/Clippy, builds,
examples and ten spec-check lanes passed. All ten admitted crates passed Cargo
publication dry-run, including isolated package compilation. No upload occurred.
The focused capability target passed 14 tests with one ignored generator.
The new cold native workflow script is still running; its result is recorded
separately after completion.

## Failures

The initial package inventory failed on missing per-crate README/license/notice
files; all admitted packages now carry them. The first Changesets fixture found
an incomplete offline Cargo cache (`wasm-streams`); explicit locked fetch fixed
it without consuming a note. The first Cargo dry-run found the agent's embedded
capability JSON outside its crate. Both embedded files were relocated unchanged,
package inventory now checks them. A second dry-run exposed an undeclared
Reqwest query feature that the workspace had supplied transitively through
compute; the agent now declares it explicitly. The same publication target is
rerun after both portability fixes.

## Gaps

Hosted CI, generated version PR, Pages deployment and the new cold local native
script remain pending. Protected bootstrap credentials and per-crate
trusted publishers need setup when an initial release is authorized. No cloud
provider credential, signing, notarization, new platform or consumer integration
is qualified by this change.

Gate result: pending hosted qualification.

Next action: finish local integration, push main, inspect hosted workflows and
retain the initial Version Packages PR open and unmerged.
