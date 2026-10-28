# Host-prepared native clients

Revision: enclosing Jujutsu change `zpmzxpuz`, based on `2d3a8b8c`.

Milestone review: M0–M4 retain their completed gates and M5 remains In progress.
This slice extends LAI-RUNTIME-001–004 and LAI-PACKAGE-001. Existing provider,
compute wire, identity and Tauri contracts are unchanged. jj-store adopts only
Control V3; no V2 compatibility client is introduced.

## Commands

- `pnpm dev:cargo -- test -p lapis-ai-runtime --test host_clients --locked`
- `pnpm dev:cargo -- test -p lapis-ai-runtime --test host_clients`
- `pnpm dev:cargo -- test -p lapis-ai-runtime -p lapis-ai-config --locked`
- `pnpm dev:cargo -- fmt --all`
- `pnpm dev:cargo -- test -p lapis-ai-runtime --test host_bundle --test host_clients --locked`
- `pnpm dev:cargo -- clippy -p lapis-ai-runtime -p lapis-ai-config --all-targets --locked -- -D warnings`
- `pnpm check:architecture`
- `node --test scripts/bundle.test.mjs`
- `pnpm check`
- `pnpm test`
- `pnpm test:governance`
- `pnpm build`
- `pnpm spec:check`
- `pnpm exec prettier --write` on the changed authored Markdown.

## Results

The three new host regressions pass, as do all 38 retained and added config/runtime
cases at the first package run. The subsequent regular-file regression passes.
Prepared launches retain explicit host environment/cwd and exact V3 frames.
Compute progress is consumed before completion and Stop reaps the helper.
Peer URL construction rejects escape inputs; Debug omits the bearer credential.
Independent output bounds reject oversized standalone results. Strict focused
Clippy and architecture checks pass. Complete workspace validation passes:
977 Rust tests, zero failures and two inherited ignores; 27 renderer tests;
42 original governance tests plus the two new bundle-verifier cases. Full
check, build and specification lanes pass. The Node bundle verifier shares the
same regular-file, version, target, hash and ABI rules for consumer development
staging. Gate result: passed for this shared source slice.

## Failures

The initial new consumer test did not compile because the required APIs were
absent. After implementation two cases passed and one failed because its HOME
assertion compared macOS's `/var` alias with canonical `/private/var`; comparing
canonical paths fixes the fixture without changing production behavior.
An early proposed V2 regression was replaced by V3 after the user's correction.
The first full check rejected the missing new SUMMARY entry; adding its exact
history entry repairs that gate. The Node bundle target first passed one case
and failed one on the same macOS path alias; canonicalizing its fixture root
makes both cases pass. The inherited native compute linker unwind-table warning
remains visible, without suppressed diagnostics.

## Gaps

Native/model and jj-store consumer acceptance is recorded separately.
No publication or release authorization is inferred from this change.

Next action: integrate the verified local public clients in jj-store, run the
consumer acceptance and retain separate cold platform evidence.

## Final specification review

The first complete `pnpm spec:check` passed canonical integrity, formatting,
architecture, boundaries, progress, upstream, all 44 governance tests and the
book build, then failed spec-first because the tooling changes also map to
`current/governance.md`. The preceding results paragraph's specification pass
applies only after adding that owning unchanged-contract confirmation and
rerunning the exact gate. This correction and the confirmation belong to the
same enclosing Jujutsu change; no validator or requirement is weakened.
