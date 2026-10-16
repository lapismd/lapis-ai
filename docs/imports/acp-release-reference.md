# Publication and release

The public repository is stevejuma/lapis-acp. The initial version is 0.1.0. The CLI
package is lapis-acp and installs the lapis-acp executable; the existing CLI source
directory and lapis_acp_cli library name remain. The layered libraries and dev-only
testkit are released at the same version. The npm package remains a private
development entry point.

## Requirements

| ID          | Requirement                                                                                                                                                                                                                                |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| LAI-REL-001 | Published crates MUST declare versioned registry dependencies, portable metadata, packaged fixtures where applicable, and Apache-2.0 license and upstream notices.                                                                         |
| LAI-REL-002 | CI MUST check committed base/head ranges in Git checkouts, preserve local Jujutsu checks, and run canonical and native platform qualification without model credentials.                                                                   |
| LAI-REL-003 | GitHub Pages MUST publish the canonical mdBook from main with the repository site prefix; pull requests MUST build without deployment authority.                                                                                           |
| LAI-REL-004 | Releases MUST match workspace versions and main ancestry, validate package contents and native executable archives, and include SHA-256 checksums.                                                                                         |
| LAI-REL-005 | Initial crate publication MUST use explicitly configured token authentication; subsequent automation MUST use scoped trusted publishing and refuse mismatched existing package contents.                                                   |
| LAI-REL-006 | Changesets MUST prepare one coordinated private coordinator/Cargo version and changelog; release PRs MUST pass canonical/native checks before a new immutable tag dispatches trusted publishing; released history MUST NOT remain pending. |

## Supported initial profile

Native CI covers Ubuntu 22.04 and macOS 15 on ARM64 and x86-64. GNU/Linux
archives require an Ubuntu 22.04-compatible userspace. macOS distributions do
not include Developer ID notarization. Windows has no initial support claim.
Control v3 remains experimental; release tooling cannot close M12's live-adapter
gates. Deterministic fixtures remain separate from installed-runner evidence.

## Release gates and recovery

Release tags are `v<workspace-version>` on main history. The release workflow
qualifies the source, builds native archives, verifies registry package
identity, publishes missing versions and tests registry installation before
publishing GitHub assets. All published packages retain the same source
revision. Existing versions are accepted only when their archive checksum
matches the intended package; a different checksum blocks publication. Partial
publication may resume only for missing packages. Published versions and release
tags are never replaced.

The initial bootstrap uses local Cargo authentication. Each crate then
configures stevejuma/lapis-acp, release.yml and the release environment as its
trusted publisher. No provider or registry credentials belong in tracked files,
logs or artifacts. GitHub workflow tokens have minimum job permissions; private
reports use GitHub's private vulnerability reporting feature. Operational steps
are in docs/releasing.md.

The canonical CI job installs mdBook through the pinned Rust tooling Action and
actionlint 1.7.12 from its official release archive with an explicit SHA-256
check. actionlint is not a supported taiki-e/install-action tool. The initial
hosted job failed at tool setup before source gates; corrected setup must pass
those gates before release.

Registry existence is established by the version metadata endpoint before
downloading an archive. An unpublished CDN object can return HTTP 403; that is
not a credential failure and must not be used to decide existence. Metadata 404
admits publication, while any error downloading a known version blocks
publication and any checksum mismatch still refuses replacement.

## Initial publication evidence

Version 0.1.0 is published for all six crates at tag v0.1.0. The four native
archives and SHA256SUMS are public, registry installation and standalone testkit
fixtures passed, and all six Rust API documentation builds are available. M14
records the hosted source, Pages and release run IDs. A crates.io new-crate
quota interrupted the sixth upload; checksum verification admitted only the
missing CLI after the published retry time. All six crates enforce the
configured trusted publisher. M12 remains experimental with its existing
live-adapter gaps.

The user confirmed bootstrap-token revocation after local credentials were
removed. Future publication uses the six configured trusted publishers. GitHub
protects main with the canonical and three native CI checks, forbids
force-push/deletion, and prevents updates or deletion of v* release tags.
Contributors submit future source updates through a passing pull request;
release tags continue to identify the exact published source.

## Changesets orchestration

The private @lapismd/lapis-acp development package coordinates all six Cargo
crates. Pending notes target that package with patch, minor or major intent.
Changesets generates the changelog and coordinator version; release:version
synchronizes Cargo workspace/internal versions, both lockfiles, installation
examples and the packaged CLI README. Recognized Dependabot and Changesets PRs
skip same-change specification/evidence bookkeeping under LAI-GOV-002's
automation exception; all other qualification remains required. Version
preparation does not modify canonical chapters or the ledger and never claims
product qualification or publication.

The main workflow creates or updates a Version Packages pull request. Explicit
CI and Pages dispatches qualify bot-created PRs, because events produced with
GITHUB_TOKEN do not otherwise start those workflows. Maintainers merge a passing
version PR. With no pending notes, a missing version tag is admitted only after
the canonical/native lanes pass; the workflow creates that tag once and
dispatches release.yml. An existing tag is retained without repackaging newer
main. Failed releases resume through the existing tagged release workflow.

Version 0.1.0 is backfilled in CHANGELOG.md and docs/releases/0.1.0.md as
already published history. The archive is outside the active note directory.
Installing orchestration creates no pending bump, new tag or registry upload.

Cold workflows fetch locked Cargo dependencies during setup. Version preparation
requires an offline locked fetch before Changesets consumes notes, so missing
registry metadata aborts without changing the coordinator or release queue. The
bootstrap is dependency installation, separate from offline source gates.
