# Merge queue and compute busy-slot release

Revision: enclosing Jujutsu change.

Milestone review: CI admission and compute control-race repair only. Publication
authority and admitted crates are unchanged.

## Contract review

LAI-REL-002 still requires hosted exact-source CI. Main now uses a merge queue
so required checks run on the exact post-merge commit before that commit is
admitted to main, closing the PR-green / main-red gap from merge commits and
timing flakes. LAI-COMPUTE control still serializes long operations; the busy
flag is released before the terminal completed/error frame so clients that react
to that frame are not rejected while the JoinHandle winds down.

## Commands and source results

- `pnpm dev:cargo -- test -p lapis-compute --test control --locked` — 2 passed;
  acknowledgement case also re-run three times green
- `JJ_STORE_VALIDATION_RUNTIME=native pnpm spec:first` — 3 protected files,
  3 canonical chapters
- `pnpm release:check` — coordinated metadata verified at 0.2.0
- CI and Pages workflows listen for `merge_group`; repository ruleset
  `Main merge queue` is active on `main`

## Unchanged-contract confirmation

Human Version Packages merge, immutable tags, scoped trusted publishing and both
admitted ARM64 native gates remain required. Merge-queue admission does not
waive exact-source CI for release tagging.
