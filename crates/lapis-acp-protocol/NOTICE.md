# Third-party references

`buzz-acp` (Block, Inc. and contributors), crate `crates/buzz-acp` of the
`block/buzz` repository at commit `2664d14316790a57ea44b3f97c57440b70e43436`,
Apache-2.0. This repository is a derived work: its architecture, its ACP wire
handling, its turn and session lifecycle, its one-shot task document and
terminal record contract, its exit code assignments and its bounded cancellation
and process-group cleanup semantics are taken from that crate. The pinned
commit, the per-region disposition of the port and the observed digests are
recorded in `docs/upstream-lock.md`, `docs/upstream/disposition.json` and
`docs/upstream/lock.json`.

No upstream implementation source is copied verbatim into this repository.
Upstream source is materialized only into the ignored `.local/upstream/` scratch
subtree for comparison, is never committed, is never edited in place and is
never referenced by a build. Ported regions are rewritten into decomposed
modules that satisfy this repository's counted-line budget, with the Nostr relay
intake, identity, author gating, key custody, commit and tag signing, relay
publication and onboarding removed rather than stubbed.

Where a ported region retains upstream structure closely enough to be a
derivative of specific expression rather than of design, the Apache-2.0
copyright and license notices of `block/buzz` apply to that region and are
preserved here. Apache License 2.0 text is available at
https://www.apache.org/licenses/LICENSE-2.0. This repository is itself
Apache-2.0; see `LICENSE.md`.

Upstream repository-level agent instructions are not adopted. They are written
for a relay-connected harness and contradict the boundaries recorded in
`spec/src/security.md`.

The Agent Client Protocol specification is an independent published protocol
described at https://agentclientprotocol.com/. This repository implements the
protocol directly over newline-delimited JSON-RPC and depends on no ACP library,
so there is no upstream protocol crate to attribute.

Runtime and build dependencies retain their own licenses. Rust toolchain
components are pinned by `rust-toolchain.toml`, and every crate resolution is
recorded with its checksum in `Cargo.lock`. `@lapismd/spec-validator` is
consumed as a released npm package under its own MIT license, and Prettier is
consumed as a released npm package under its own MIT license. No dependency
source is vendored, patched, aliased or shimmed.
