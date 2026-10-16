# Releasing Lapis AI

The release policy is [LAI-REL-001–007](../spec/src/current/contracts/release.md).
`release.toml` declares ten Cargo packages, one native-only compute helper and
the private `@lapismd/lapis-ai` coordinator. No npm package is published.

## Normal development

1. Run `pnpm changeset`, select `@lapismd/lapis-ai`, choose a bump and describe
   the user-visible result. Do not manually edit package versions.
2. Keep canonical requirements and verification evidence with the implementation.
   Run `pnpm check`, `pnpm test`, `pnpm build`, `pnpm spec:check`,
   `pnpm check:workflow`, `pnpm package:check` and `pnpm release:dry-run`.
3. Push the reviewed source change. Changesets creates/updates **Version Packages**.
   It explicitly dispatches CI and the Pages build because GitHub does not
   automatically run workflows for its own token-created commits.
4. Leave version PRs open until a human authorizes the release. No auto-merge is
   enabled. Version preparation never creates specification evidence.

Local preparation uses `pnpm dev:cargo -- fetch --locked` followed by
`pnpm release:version`. The initial fetch populates all locked dependencies;
version preparation runs offline and refuses to consume notes if that cache is
missing. The real preparation regression runs in a disposable fixture. Verified
same-repository bot changes may skip only same-change specification bookkeeping;
canonical integrity, tests, build, package contents and dry-run gates remain.

## GitHub setup

The public repository is `stevejuma/lapis-ai`. Enable Actions permission to
create pull requests. Workflows keep default token access read-only and grant
write scopes to the jobs that need them. Pages uses **GitHub Actions** as its
build source and the `github-pages` environment. The spec publishes at
<https://stevejuma.github.io/lapis-ai/> with site prefix `/lapis-ai/`.

Create a protected **release** environment restricted to version tags and a
maintainer approval. Credentials belong only in that environment. Never add
credentials to TOML, checkout files, artifacts or command arguments.

## First Cargo publication

New crate names require an initial authenticated namespace creation before
crates.io trusted publishers can be configured. This repository does not carry
or create a token during setup.

1. Review and merge the generated initial `0.1.1` version PR when release is
   authorized. CI qualifies its exact main revision; orchestration creates the
   immutable `v0.1.1` tag. An ordinary OIDC attempt without configured publishers
   can stop without publishing; no token fallback occurs.
2. For the explicit initial bootstrap, add a narrowly scoped crates.io API token
   as environment secret `CARGO_REGISTRY_BOOTSTRAP_TOKEN`. Dispatch **Release**
   with `tag=v0.1.1`, `publish_crates=true`, `bootstrap=true`, then approve the
   protected job. Dispatch from the tag as well as selecting its input, because
   the environment admits only version-tag workflow runs. The identity guard
   refuses this mode for other versions or events. Delete this secret after initial namespace creation.
3. Configure a trusted publisher on every admitted crate for owner `stevejuma`,
   repository `lapis-ai`, workflow `release.yml`, environment `release`.
   Future publication exchanges GitHub OIDC for a short-lived scoped token.

The explicit initial dispatch is:

```sh
gh workflow run release.yml --repo stevejuma/lapis-ai --ref v0.1.1 -f tag=v0.1.1 -F publish_crates=true -F bootstrap=true
```

Cancel any earlier blocked release run before dispatching its retry; release runs
for the same tag are serialized.

The compute helper stays unpublished while its pinned Mesh SDK has no matching
registry version. Do not replace the SDK just to make Cargo publication pass.

## Immutable release and retry

After human merge, orchestration runs reusable exact-source CI before creating
one `v<version>` tag and dispatching Release. Release verifies tag/version,
checkout identity, main ancestry, no pending notes and coordinated metadata.
Supported ARM64 jobs qualify real inference, permissions, clients and the staged
release bundle. Artifacts retain TOML inventories, SHA-256 checksums and licenses.

The protected Cargo job packages the ten admitted crates, compares already
published bytes, then publishes missing crates in Cargo dependency order.
Checksum mismatch stops the release. The job also installs and runs the published
ACP executable. GitHub release assets are uploaded only after registry acceptance;
existing assets must match byte-for-byte. Tags and differing assets are never
replaced.

Resume a failed release by dispatching **Release** against its existing tag.
`publish_crates=false` is verification-only and still requires every registry
version to exist. Do not delete/recreate a release tag or broaden token access to
work around a mismatch. Native archives currently support macOS ARM64 Metal and
Linux ARM64 CPU (glibc 2.38+); model weights are never included. Signing,
notarization, installers and other platform profiles remain separate work.
