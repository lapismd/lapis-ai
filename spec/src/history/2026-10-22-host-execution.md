# Host execution backend

Revision: enclosing change, parent `9cadb47d`.

Milestone review: M0–M5 retain their status. H04 gains a native execution port;
hosted admission, transport and durable conversations remain consumer gates.
Docker execution stays deferred. Nothing is published or installed.

## Implementation

`ExecutionBackend` accepts only a policy-admitted prepared launch, session
options, cancellation receiver and optional owned processor. Its returned
`ExecutionSession` retains the exact worker identity and negotiated facts,
typed requests, event snapshots, acknowledgements, subscription and confirmed
close. A future backend owns its complete execution resource lifetime through
this port; application admission, credentials and durable state remain external.

`HostExecution` wraps the existing supervised ACP session. Existing constructors
select it, preserving Desktop's Tauri adapter. Rust hosts can inject another
backend with an explicit live Tokio handle. No browser payload selects an
executable or backend. Refusal never falls back or replays the request, and
a substituted worker identity is closed before exposure.

## Qualification

Initial targeted Cargo execution was blocked at 29.1 GiB free by the unchanged
30 GiB guard. Only the inactive, previously approved
`lapis-ai/target/debug/incremental` compiler cache was removed after checking
that no Cargo, Rust or Clippy process was active. Binaries and model data remain.
The focused test then failed to compile against the absent execution port.

`pnpm dev:cargo -- test -p lapis-ai-service --test execution --locked`
passes all four new real-process cases: selected host execution and retirement,
denied scope/target before backend entry, refusal without fallback or duplicate
open, and substituted identity cleanup. The existing complete service/adapter
suite passes before the additional identity case. Final full qualification
passes all 1,076 Rust cases (two inherited ignored), 43 renderer cases and
47 governance cases, plus strict workspace Clippy and all builds.

Final commands use `CARGO_INCREMENTAL=0` to avoid regenerating that compiler
cache: `pnpm check`, `pnpm test`, `pnpm build` and `pnpm spec:check`.
The inherited Compute linker unwind-table warning remains visible.
The first check rejected the verification status `Pending`; it now uses the
repository's accepted status vocabulary. The first specification check also
required the mapped runtime chapter; its unchanged protocol/semantics are now
recorded there. The final specification check passes all ten lanes.
No validator, disk threshold, process bound or permission rule is relaxed.

This record qualifies no Docker adapter, hosted HTTP binding, durable turn,
real installed provider or loaded model. Those retain the migration gates.
