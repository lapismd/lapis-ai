# Coordinated Cargo versions

Run `pnpm changeset` and select the private `@lapismd/lapis-ai` coordinator.
A patch/minor/major note versions every Cargo package together. This package is
not published to npm. Changesets opens a Version Packages PR; maintainers merge
it after its checks pass. Automation never merges that PR.

`pnpm release:version` consumes notes, updates Cargo metadata/lockfiles and the
changelog, and preserves canonical specifications and evidence. Read
[the release guide](../docs/releasing.md) before publishing or creating tags.
