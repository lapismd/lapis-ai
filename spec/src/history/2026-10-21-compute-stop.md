# Compute stop followed by supervised cleanup

Revision: enclosing Jujutsu change, based on `f8daaa45`.

LAI-RUNTIME-001/003 retain bounded native ownership. Desktop uses the public
independent command sender to interrupt active model work. Once that stop has
closed the child input, final cleanup must use the successful supervised reap
as its proof. It must still reject a failed reap or unsuccessful child exit.
An interrupted operation remains unknown; cleanup cannot complete its receipt.

## Regression and verification

- The first exact run was stopped by the 30 GiB disk preflight at 29.3 GiB.
  Guarded jj-store cleanup reclaimed only eligible workspace artifacts.
- `pnpm dev:cargo -- test -p lapis-ai-runtime --test host_clients compute_progress_and_stop --locked`:
  fails (one case) with `submission_unknown` after the helper has exited.
- A second build was stopped at 29.8 GiB. With native compilation idle, removed
  only lapis-ai’s generated `target/debug/incremental` directory. No source,
  fixtures, installed runtime, credentials or user cache was removed.
- `pnpm dev:cargo -- test -p lapis-ai-runtime --test host_clients --locked`:
  all three cases pass with the same stop-before-cleanup assertion.

The complete workspace rerun and canonical checks are recorded below after
completion. Desktop native/model/pairing acceptance is distinct from these
scripted helper tests. Original M0–M5 status and release authority are unchanged.

The final `CARGO_BUILD_JOBS=2 pnpm test` passes: 1062 Rust/doc
cases across 102 targets, two inherited ignored, 43 renderer-client and
45 governance cases. No deadline or assertion is relaxed.

The first strict runtime Clippy run rejected an unnecessary return binding.
After removing it, `pnpm dev:cargo -- clippy -p lapis-ai-runtime --all-targets
--locked -- -D warnings` and the exact three-case host-client target pass.
`pnpm spec:check` passes all ten canonical lanes. Final document checks rerun
after adding these results.
