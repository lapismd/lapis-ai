# CI, versioning and release

The canonical repository is `lapismd/lapis-ai`. Repository transfers MUST update
owner-bound automation, package metadata, remote URLs and trusted-publisher
instructions while preserving public visibility, history and open version PRs. Version PRs require human review and are never
merged by automation. No package or tag is published during this setup.

## Requirements

| ID          | Requirement                                                                                                                                                                                                                                                                            |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-REL-001 | Cargo publication MUST include portable metadata, registry-versioned internal dependencies, notices, licenses and required fixture sources. Test helper binaries MUST require an explicit development feature and MUST NOT install by default.                                         |
| LAI-REL-002 | CI MUST qualify exact committed revisions with explicit Git base/head ranges, retain local Jujutsu checks and run formatting, specifications, boundaries, release/package checks, Clippy, tests, builds and supported native acceptance.                                               |
| LAI-REL-003 | Changesets MUST coordinate the private npm coordinator and renderer client, all thirteen Cargo versions, internal version ranges, lockfiles and changelog. Version preparation MUST NOT fabricate specification or milestone evidence.                                                 |
| LAI-REL-004 | A same-repository Changesets or Dependabot PR MAY omit same-change spec/evidence bookkeeping only after exact bot, branch, base, repository and revision checks. Every product, packaging and canonical integrity gate MUST remain required.                                           |
| LAI-REL-005 | Cargo release MUST qualify an immutable version tag on main history, reject pending Changesets and mismatched registry contents, and publish only explicitly admitted crates. Credentials MUST remain in protected release jobs; normal automation MUST use scoped trusted publishing. |
| LAI-REL-006 | GitHub Pages MUST build the canonical mdBook for pull requests and deploy only main through the GitHub Pages environment with the `/lapis-ai/` site prefix.                                                                                                                            |
| LAI-REL-007 | Native releases MUST include supported target inventories, SHA-256 checksums and licenses. The pinned compute executable MUST remain native-only while its exact Mesh dependency revision has no matching registry release.                                                            |

## Publication scope

`release.toml` declares twelve crates.io packages and one native-only package.
The initial registry API check found all Lapis names unallocated. The exact
Mesh source version `0.76.0-rc9` is absent from crates.io, so `lapis-compute`
retains `publish = false`; the SDK pin is unchanged. GitHub native bundles
include compute alongside ACP, agent and tools. They do not include model weights.
The npm coordinator and the `@lapismd/ai` renderer client stay private and have
no npm publication or tag lane. The client shares the coordinated version;
version preparation checks and updates its manifest alongside Cargo versions.

Changesets generates a Version Packages PR. A passing PR is merged by a human.
The reviewed version on main then becomes a candidate for exact-source CI and
one immutable `v<version>` tag. Existing tags are never moved. Release resumes
against the existing tag and accepts existing crates only after checksum
verification; retries publish only missing packages.

Initial crates.io namespace creation needs explicit bootstrap credentials before
per-crate trusted publishers can be configured. The optional bootstrap mode is
manual, protected and limited to the initial release version. It is never an
automatic fallback when OIDC fails. Later releases use the repository,
`release.yml` and `release` environment as trusted-publisher scope.

This setup includes an initial patch note and deliberately leaves the generated
version PR open. Native profile claims remain macOS ARM64 Metal and Linux ARM64
CPU with glibc 2.38 or later. Public release claims require actual hosted gates;
local historical qualification does not substitute for hosted results.

Coordinated preparation includes formatting regenerated lockfiles. A disposable
real version-preparation test checks the resulting pnpm lock against the normal
formatter, in addition to synchronized versions and preserved canonical evidence.

The coordinated registry cohort includes the independent `lapis-ai-capture`
source library. It inherits the same workspace version and publication gates;
adding it does not publish a package or merge a release change.
