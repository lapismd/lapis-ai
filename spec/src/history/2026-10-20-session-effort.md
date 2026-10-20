# Session-owned ACP reasoning effort

Revision: enclosing Jujutsu change `sztwktxu`, based on `a8075601`.

LAI-LOCAL-001 now advertises the bundled agent’s existing reasoning effort
through ACP thought-level configuration. Idle sessions may select supported
provider values independently; busy/unknown sessions and unknown values refuse
without mutation. A prompt snapshots its own effort. Provider credentials,
permission gates, model policy and M0–M5 milestone gates are unchanged.

## Verification

- `pnpm dev:cargo -- test -p lapis-agent --test session_configuration --locked`:
  one real-process regression passes, without external provider calls.
- `pnpm test > .local/m10-effort-tests.log 2>&1`: client tests pass (43),
  governance passes; the workspace stops at one activity-fairness failure
  under concurrent native compilation (active output consumer lost coverage).
- `pnpm dev:cargo -- test -p lapis-acp-control --test activity_fairness --locked`:
  all three cases pass in isolation; no assertion or deadline changed.
- `pnpm dev:cargo -- clippy -p lapis-agent --all-targets --features test-support --locked -- -D warnings`:
  passes.
- Final workspace rerun with `CARGO_BUILD_JOBS=2 pnpm test` is recorded below.

Desktop M10.2–M10.4 integration and real-provider/native inference acceptance
remain consumer-owned. No package, tag or application is published or installed.
Visual and other-platform qualification remain separate.

The bounded-job workspace rerun passes (43 renderer clients, 45 governance,
all Rust/doc targets; two inherited ignored native/fixture cases). The first
canonical check refused the missing mdBook navigation entry; it is added before
rerunning. No product assertion was relaxed.
