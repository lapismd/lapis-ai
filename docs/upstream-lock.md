# Upstream lock and port audit

The ACP subsystem, imported from `jj-acp`, is a derived work of the `buzz-acp` crate in the `block/buzz`
repository. This document is the audit trail for that relationship. It records
what is pinned, what was taken, what was refused and how to prove the record is
still true.

| Field       | Value                                      |
| ----------- | ------------------------------------------ |
| Repository  | `block/buzz`                               |
| Crate path  | `crates/buzz-acp`                          |
| Commit      | `2664d14316790a57ea44b3f97c57440b70e43436` |
| Commit date | 2026-09-30                                 |
| License     | Apache-2.0                                 |
| Regions     | 40 classified                              |

The machine-readable pin is `docs/upstream/disposition.json`, which is curated
by hand, and `docs/upstream/lock.json`, which
`scripts/upstream-sync.mjs --write` generates by joining that map with observed
SHA-256 digests. The generated lock is committed so a reviewer can see exactly
which bytes the port claims to derive from; the curated map is the authority on
intent.

## Disposition vocabulary

Every classified region carries exactly one disposition. The progress checker
and the lock validator both refuse an unclassified upstream file, so a new
upstream file cannot enter the pin silently.

| Disposition           | Meaning                                                                  |
| --------------------- | ------------------------------------------------------------------------ |
| `ported`              | Taken substantially as-is.                                               |
| `ported-with-changes` | Taken and deliberately altered; the note names the alteration.           |
| `dropped`             | Refused on principle; the note names the reason and the owner is `none`. |
| `not-ported`          | In scope eventually but not yet; the note names the deferred concern.    |

At the pinned commit the distribution is 1 ported, 18 ported-with-changes, 14
dropped and 7 not-ported.

## What was taken

The ACP client is the reason this port exists. Upstream concentrates framing,
correlation, capability negotiation, session setup, model and permission
handling, stop-reason mapping and legacy first-turn framing in one 215 KB file.
Here it is decomposed across `crates/lapis-acp-protocol` under the counted-line
budget, and the port map records the owning crate rather than a single file so a
future comparison is region by region.

The turn machinery is taken next: session and turn lifecycle, isolated
execution, pool lifecycle states, prompt framing and the shared runtime that
owns launch resources. Channel context resolution, relay presence and
relay-driven wake conditions are removed from it rather than stubbed.

The one-shot task runner is taken close to intact, because its contract is
already transport-neutral: one prepared document in, one sanitized terminal
record out, published exit codes, bounded cancellation and process-group
cleanup. Its identity check is retargeted from a Nostr public key to a runner
identifier, which preserves the mis-execution guard without importing an
identity system.

The deterministic ACP peer fixture is taken because it is what makes the whole
suite runnable without a model provider, a network endpoint or a credential.

Session scoping is taken and generalized. Upstream distinguishes channel, thread
and task scopes because a relay has those concepts; here a scope is one opaque
caller-named key, and the controlling application decides what it means.

## What was refused

The ACP crates exclude Nostr; the separately pinned compute dependency is outside this ACP port audit.
`scripts/boundaries.mjs` fails closed on a Nostr, transport, HTTP, TLS or
database dependency, for ACP dependencies.

| Refused region                                              | Reason                                   |
| ----------------------------------------------------------- | ---------------------------------------- |
| Relay intake, subscriptions, REST discovery, NIP-42, NIP-98 | Nostr transport and authentication       |
| Author gate, owner resolution, workflow provenance          | Nostr identity and authorization         |
| Git commit and tag signing, credential helpers, keyfile     | Nostr signing and key custody            |
| Memory fetch, observer publication, usage publication       | Relay publication and relay-keyed memory |
| Setup and onboarding mode                                   | Relay membership registration            |
| Channel and thread session models                           | Relay conversation concepts              |
| Relay recovery and wake tests                               | Tests for refused behaviour              |

Relay reconnection, subscription replay and overflow recovery are refused with
the intake they serve. Durability, retry policy and unknown-outcome recovery are
the controlling application's responsibility, which already owns durable state;
a runner that reinvented them would produce a second, weaker copy.

## What is deferred

Inbound batching, deduplication and overflow replay, the standing agent pool,
and channel-history prompt projection are marked `not-ported` with an owning
crate. They arrive with the persistent control transport rather than with the
one-shot task runner, because none of them is meaningful for a single prepared
turn.

## Refreshing the port

```sh
node scripts/upstream-sync.mjs --self-test   # offline: pin and map consistency
node scripts/upstream-sync.mjs --write       # network: re-pin digests at the recorded commit
node scripts/upstream-sync.mjs --fetch       # network: verify digests and report drift
```

`--self-test` is a normal check lane and never touches the network. `--write`
and `--fetch` are explicit opt-in lanes, matching the way the sibling jj-store
repository separates its self-test from its real-storage qualification.

Materialized upstream source lands only in the ignored `.local/upstream/`
subtree, addressed by repository slug and commit. Upstream source is never
committed, never edited in place and never referenced by a build. A drift report
that lists changed regions means the port map is stale: re-review each changed
region against its owning module, update the note, and record the review in the
ledger. Drift is not resolved by regenerating the lock.

## Rules

Never copy upstream repository-level agent instructions into this repository.
Upstream guidance is written for a relay-connected harness and contradicts the
boundaries recorded in [security](../spec/src/current/contracts/acp/security.md).

Keep adopted-source attribution in [NOTICE](licenses/NOTICE.md). References are
not runtime dependencies, and no upstream crate is added to `Cargo.toml`.
