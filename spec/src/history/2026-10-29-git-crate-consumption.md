# Git crate consumption

Revision: enclosing Jujutsu change.

Milestone review: Release no longer publishes crates. Publication authority for
GitHub release assets is unchanged. Downstream repositories consume the crates
from the git repository.

## Contract review

LAI-REL-005 no longer requires the Release workflow to publish admitted crates
or to compare registry bytes. The workflow still qualifies an immutable tag.
Crates stay in this repository and are consumed with a git dependency.

## Commands and source results

- `node --test scripts/release-orchestration.test.mjs`
- `node scripts/ci-spec.mjs`

## Unchanged-contract confirmation

Immutable tags, human Version Packages merges, and the native archive gates
remain required. `lapis-compute` stays unpublished.
