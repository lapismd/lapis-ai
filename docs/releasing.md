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

The public repository is `lapismd/lapis-ai`. Enable Actions permission to
create pull requests. Workflows keep default token access read-only and grant
write scopes to the jobs that need them. Pages uses **GitHub Actions** as its
build source and the `github-pages` environment. The spec publishes at
<https://lapismd.github.io/lapis-ai/> with site prefix `/lapis-ai/`.

Create a protected **release** environment restricted to version tags and a
maintainer approval. Credentials belong only in that environment. Never add
credentials to TOML, checkout files, artifacts or command arguments.

## Crate consumption

Downstream repositories consume the crates from the git repository. The Release
workflow does not publish them to crates.io. Depend on a tag or revision:

```toml
lapis-ai-runtime = { git = "https://github.com/lapismd/lapis-ai", tag = "v0.2.0" }
```

The compute helper stays out of crates.io while its pinned Mesh SDK has no
matching registry version.

## Immutable release and retry

After human merge, orchestration runs exact-source CI before creating one
`v<version>` tag and dispatching Release. Release verifies tag/version,
checkout identity, main ancestry, no pending notes and coordinated metadata.
Supported ARM64 jobs qualify real inference, permissions, clients and the staged
release bundle. Artifacts retain TOML inventories, SHA-256 checksums and licenses.

GitHub release assets are uploaded after those archives. Existing assets must
match byte-for-byte. Tags and differing assets are never replaced.

Resume a failed release by dispatching **Release** against its existing tag.
Do not delete or recreate a release tag. Native archives currently support
macOS ARM64 Metal and Linux ARM64 CPU (glibc 2.38+); model weights are never
included. Signing,
notarization, installers and other platform profiles remain separate work.
