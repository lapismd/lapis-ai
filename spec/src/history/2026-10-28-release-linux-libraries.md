# Release Linux system libraries

Revision: enclosing Jujutsu change.

Milestone review: hosted Linux release archive and crate jobs. Publication
authority, the immutable `v0.2.0` tag and admitted crates are unchanged.

## Contract review

LAI-REL-007 native bundles still require the hosted Linux ARM64 gate. The
v0.2.0 archive job failed while compiling the workspace release profile because
`glib-2.0` was absent. Native CI already installs GTK, WebKit and librsvg;
release hosts installed only `pkg-config` and OpenSSL. Release now installs the
same three development packages before the archive build and the crate package
step.

## Commands and source results

- Compared `.github/workflows/release.yml` with the native job in
  `.github/workflows/ci.yml`.
- `pnpm dev:cargo -- test -p lapis-agent --test handoff_cap_steer --locked`
  repeated after withholding the steered turn's next provider response until
  the steer reply is accepted. PR CI otherwise returned `steer: no active run
to steer` when the turn finished first.
- `node scripts/ci-spec.mjs`
- A failed Rust build or native job cancels the rest of that CI run. The native
  matrix also uses fail-fast, so one platform does not keep running after the
  other has failed.

## Unchanged-contract confirmation

Exact-source CI reuse, immutable tags, human Version Packages merges and scoped
trusted publishing stay required. This change only supplies the system
libraries the existing Tauri plugin build already needs.
