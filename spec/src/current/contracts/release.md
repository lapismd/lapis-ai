# CI, versioning and release

The canonical repository is `lapismd/lapis-ai`. Repository transfers MUST update
owner-bound automation, package metadata, remote URLs and trusted-publisher
instructions while preserving public visibility, history and open version PRs. Version PRs require human review and are never
merged by automation. No package or tag is published during this setup.

## Requirements

| ID          | Requirement                                                                                                                                                                                                                                                                       |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-REL-001 | Cargo publication MUST include portable metadata, registry-versioned internal dependencies, notices, licenses and required fixture sources. Test helper binaries MUST require an explicit development feature and MUST NOT install by default.                                    |
| LAI-REL-002 | CI MUST qualify exact committed revisions with explicit Git base/head ranges, retain local Jujutsu checks and run formatting, specifications, boundaries, release/package checks, Clippy, tests, builds and supported native acceptance.                                          |
| LAI-REL-003 | Changesets MUST coordinate the private npm coordinator and renderer client, all fifteen Cargo versions, internal version ranges, lockfiles and changelog. Version preparation MUST NOT fabricate specification or milestone evidence.                                             |
| LAI-REL-004 | A same-repository Changesets or Dependabot PR MAY omit same-change spec/evidence bookkeeping only after exact bot, branch, base, repository and revision checks. Every product, packaging and canonical integrity gate MUST remain required.                                      |
| LAI-REL-005 | Cargo release MUST qualify an immutable version tag on main history and reject pending Changesets. The Release workflow MUST NOT publish crates. Downstream repositories consume the crates from the git repository.                                                              |
| LAI-REL-006 | GitHub Pages MUST build the canonical mdBook for pull requests and deploy only main through the GitHub Pages environment with the `/lapis-ai/` site prefix.                                                                                                                       |
| LAI-REL-007 | Native releases MUST include supported target inventories, SHA-256 checksums and licenses. The pinned compute executable MUST remain native-only while its exact Mesh dependency revision has no matching registry release.                                                       |
| LAI-REL-008 | Version preparation MUST validate its qualified Node runtime, Changesets bytes and exact recipe/configuration inputs through the pinned shared guard after existing bootstrap and before App minting. Drift MUST fail closed without replacing the source recipe or native gates. |

## Publication scope

`release.toml` declares fourteen crates.io packages and one native-only package.
The initial registry API check found all Lapis names unallocated. The exact
Mesh source version `0.76.0-rc9` is absent from crates.io, so `lapis-compute`
retains `publish = false`; the SDK pin is unchanged. GitHub native bundles
include compute alongside ACP, agent and tools. They do not include model weights.
The npm coordinator and the `@lapismd/ai` renderer client stay private and have
no npm publication or tag lane. The client shares the coordinated version;
version preparation checks and updates its manifest alongside Cargo versions.

Changesets MUST generate Version Packages PRs on canonical main using the
existing dependency App, scoped to this repository with contents and pull-request
write permissions only. Missing App credentials MUST fail closed. Checkout MUST
not persist credentials. CI dispatch, tag orchestration and release publication
MUST retain their existing workflow credentials and exact-source/native gates.
Changesets MUST NOT create GitHub releases directly.

LAI-REL-008 consumes the source-owned public qualification file through Lapis CI
`2f90eb7a035f841f0757d3a2cffc959cbecdfff0`. The guard retains Node22,
CLI2.31.1, pnpm10.34.5, Rust1.99.0 and the actual offline version recipe.
The successful [whole-recipe proof](https://github.com/lapismd/lapis-ci/actions/runs/43814544020)
precedes adoption; its historical note is disposable validation input only.
Input drift requires owning-source review and requalification before writer
authority. The guard adds no credentials or publication/deployment authority.

A passing PR is merged by a human
through the main merge queue, which MUST run the same required checks on the
exact post-merge commit before that commit reaches main. The reviewed version on
main then becomes a candidate for exact-source CI and one immutable
`v<version>` tag. Changesets tag qualification MUST reuse a
successful exact-source CI run for that candidate SHA (Plan, Spec, Rust build,
both admitted native targets and Canonical checks) and MUST NOT duplicate native
acceptance by re-invoking the reusable CI workflow when those checks are already
green. When no non-`workflow_call` CI run exists for the main tip SHA,
orchestration MAY dispatch CI once and await it. The Release workflow MUST reuse
the same exact-source CI await for the immutable tag SHA and MUST NOT re-invoke
the reusable CI workflow. Existing tags are never moved.
Release resumes against the existing tag. It publishes GitHub release assets
and does not publish crates. Downstream repositories consume the crates from
the git repository.

This setup includes an initial patch note and deliberately leaves the generated
version PR open. Native profile claims remain macOS ARM64 Metal and Linux ARM64
CPU with glibc 2.38 or later. Linux release archive hosts install the same
GTK, WebKit and librsvg development packages as native CI so the workspace
release build can link the Tauri plugin. Public release claims require actual
hosted gates; local historical qualification does not substitute for hosted
results.

Coordinated preparation includes formatting regenerated lockfiles. A disposable
real version-preparation test checks the resulting pnpm lock against the normal
formatter, in addition to synchronized versions and preserved canonical evidence.

The coordinated registry cohort includes the independent `lapis-ai-capture`
source library. It inherits the same workspace version and publication gates;
adding it does not publish a package or merge a release change.

The cohort includes `lapis-ai-app-tools`, an injectable library with no new
helper binary or native access. This source addition does not publish it; its
namespace/distribution and native consumer qualification remain separate gates.

`lapis-ai-service` joins the coordinated library cohort. Extracting the existing
host implementation does not publish a crate or grant hosted execution.

## Bounded specification-tool intake

LAI-GOV-100/103/105 and LAI-REL-008 retain the actual private Node22/Rust
version recipe and native/release gates. Only compatible Node-only validator0.2
development patches are admitted. Independent normal source checks validate the
complete catalog and both managed slots outside trusted-bot spec exemptions.
One exact managed maintenance note may contain only an empty YAML release map;
null, nonempty, malformed, multiple and arbitrary notes reject. Version
consumption may remove that note only with unchanged complete dependency facts;
present notes always validate, and actual SDK intake still requires its bytes.
This does not change the private empty-only whole-version recipe or grant
publication authority. A one-time exact
reviewed baseline plus before/after input hashes binds initial SDK0.2.0→0.2.1
adoption; missing/foreign ancestry and future initialization fail closed. Every
ordinary later update requires its existing immutable opt-in and factual slots.

<!-- dependency-intake:validator-development:start -->

```json
{
  "schemaVersion": 1,
  "kind": "development-dependency-intake",
  "dependency": "@lapismd/spec-validator",
  "version": "0.2.1",
  "peer": null,
  "integrity": "sha512-aHLByppZkay1axSm8KvCwOJz0si8aDyZBXDGWW+d2VQw+66f1KLimw2rEMjYIpLdoSG9+yN5Kd1MKlAJw7xUVw==",
  "importers": {
    ".": {
      "specifier": "0.2.1",
      "version": "0.2.1"
    }
  },
  "policySha256": "3aa73330b177ffb38c07e8096b526ff9ba66b32a973a36d0339f16c0e09ccb72"
}
```

<!-- dependency-intake:validator-development:end -->

Exact current remote-main qualification distinguishes CLI status from the raw
no-note macro: changed private packages need an empty maintenance note for status
admission even when the raw macro returns unchanged. The empty map still yields
false intent, preserving App/writer admission and all native/publisher gates.

Canonical dependency validation reads baseline facts and changed paths from the
same captured immutable parent. Clean Git CI compares that parent with committed
HEAD; local JJ compares the same parent with its selected working revision.
Managed development notes and unexpected source paths remain checked in both
transports under LAI-REL-002/008; a clean working tree is not an empty candidate.
