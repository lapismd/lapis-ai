# Third-party references

The ACP-only notice below is inherited from the pinned jj-acp import. The
separately copied maintained Buzz agent is attributed in `docs/agent-upstream.md`
and `crates/lapis-agent/LICENSE`. The compute dependency Mesh-LLM is pinned to
9f192c9d821991ba43d79bb15fd2f2f12e597571 under Apache-2.0; its full license is
retained in `docs/licenses/LICENSE.mesh-llm`. Native runtime bundles retain their
own manifests and notices; their presence does not extend the ACP-only claims.

The native runtime includes llama.cpp/ggml at the manifest's base revision
cc83d7b4824f73cfdda4dfbb47ee39804f71b328 and Mesh's recorded patch digest.
Its MIT text is retained in `docs/licenses/LICENSE.llama.cpp`. Native manifests
retain the patched revision, backend and library digests. This is a local
qualification bundle; release redistribution audits remain a separate gate.

## ACP-only derivation

`buzz-acp` (Block, Inc. and contributors), crate `crates/buzz-acp` of the
`block/buzz` repository at commit `2664d14316790a57ea44b3f97c57440b70e43436`,
Apache-2.0. This repository is a derived work: its architecture, its ACP wire
handling, its turn and session lifecycle, its one-shot task document and
terminal record contract, its exit code assignments and its bounded cancellation
and process-group cleanup semantics are taken from that crate. The pinned
commit, the per-region disposition of the port and the observed digests are
recorded in `docs/upstream-lock.md`, `docs/upstream/disposition.json` and
`docs/upstream/lock.json`.

The decomposed ACP port does not copy the upstream ACP implementation verbatim.
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
`spec/src/current/contracts/acp/security.md`.

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

Maintained derivative of block/buzz buzz-agent, revision ef2aa1ae38fadcc0bc22b8bf6ed96b35933146be. Original Apache-2.0 LICENSE is retained.
