# Native acceptance permission lifecycle and evidence

Revision: enclosing Jujutsu change on main `87629985`, separate from intake PR #17
and consumption PR #18. Milestone review: M0–M5 statuses and native/release
completion gates remain unchanged. This is a private qualification harness and
CI evidence repair, with no public Rust/client/runtime permission-policy change.

## Original failure

PR source `52a8236d` and queue source `78a2eeb2` have identical tree `558f883e`.
Both use macOS image `20261104.0514.1` and runner `2.338.2`. In queue run `44237011008`,
the first unbundled permission/edit fixture passed; bundled direct inference
also passed. The bundled agent then reported Absolute outcome_unknown at 300s,
and its existing fresh-temp retry produced a terminal record before requesting
a tool. Self-cancellation changed the job result from failure to cancelled.
Actual terminal frames/model/runtime byte hashes were not retained; the original
model/hardware cause remains unknown.

## Commands

The exact two-regression boundary is:

```sh
python3 scripts/native-acceptance.test.py NativeFixtureTests.test_repeated_legal_permissions_complete NativeFixtureTests.test_early_terminal_retains_status
```

Broader qualification uses the complete Python protocol/evidence controls,
Node governance/workflow controls and the default bounded portable
validation:all lane. Hosted Spec uses exact immutable base/head refs.

## Results

Before repair, both exact targets fail: a second legal permission is silently
dropped until a terminal timeout, and early terminal status disappears behind
a generic assertion. Both now pass. The loop handles bounded repeated exact
writes plus read_file/search requests within the disposable file/root, selecting
each advertised allow_once identifier independently. Paths, mutation content,
session/request identity, stale IDs, counts, pre-approval effects and terminal
completion remain enforced. One 300000ms absolute deadline never resets.

Unknown outcomes remain failures, with no fresh-turn retry. The reporter keeps
only 64 safe frame summaries, streamed stderr length/hash and bounded hashes for
selected helpers, managed model bytes and runtime assets. Private roots, raw
environment, credentials, raw stderr and model payloads never enter artifacts.
Native CI uploads only these JSON summaries, with attempt-specific names, and
preserves failure status rather than cancelling its own run. Matrix fail-fast,
both ARM64 jobs, all native commands and required branch gates remain in place.

A separate negative lifecycle control also found and closes a reporting error:
a cleanup failure could leave a standalone passed report. It now records failure
and preserves the primary assertion, and stream cleanup never waits on a live
reader's locked file object. This negative case is part of the final fixture suite.

## Failures

The original real native failure remains part of the record; fixture controls
do not prove its internal model/runner cause or relabel it a flake. Unsafe tool,
path and permission cases are intentionally refused. Diagnostic gaps remain
explicit rather than masking a primary acceptance failure.

## Gaps

Portable checks do not prove native Metal, Linux ARM64, real-model/tool behavior,
provider interoperability, bundles or release readiness. New exact-head hosted
native checks and merge-queue checks are mandatory before this fix lands.
No user/provider profile or existing uncertain turn was exercised or replayed.

Next action: normal fresh hosted CI and merge queue; rebase the independent
intake/consumption branches after this repair lands.
Gate result: source fixture and portable qualification passed; native and
release qualification remain separate.

The exact original two regressions now pass, plus 14 Python protocol/evidence
controls and two Node workflow controls. The final default Docker
validation:all run passed check, tests, build, workflow and package checks plus
all 11 spec lanes. It used the unchanged tool image `2935940257ca17437e3d` and
cache `c87aae796164b6f56775` with 4 CPUs/12 GiB and one Cargo job.
Rust passed 1078 tests with no failures; two existing opt-in ignores remain the
model corpus writer and the isolated installed-runtime/GGUF test. Client tests
passed 43, intake tests 46 and governance tests 73. No native/model ignore is
reported as passing. Doctor/QMD and final exact-source Node CI checks also passed; they remain
separate from native/model qualification.

The first aggregate found one missing SUMMARY entry for this evidence page;
the source index was corrected and both later aggregate runs passed. No rule,
assertion, source limit, audit or native platform gate was relaxed.

## Fresh native rejection and diagnostic follow-up

Fresh PR #19 source `aabc743e` ran CI `44244495959`. macOS job `134600793907`
failed in the first source-native fixture at `permission_choice`'s tool/input
predicate; ARM Linux was cancelled by the unchanged matrix fail-fast. Spec,
Rust and canonical jobs passed. The retained JSON artifact `14401821000` proves
15 frames ending at permission, no terminal, no diagnostic errors, and the
managed model's 396705472 bytes with SHA256
`ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a`.
It did not retain the refused arguments, so the exact failed predicate remains
unknown. This is distinct from the earlier queue timeout and does not prove
an environmental failure or justify widening permissions.

The follow-up changes diagnostics only. A refused tool/input predicate adds a
known tool enum, typed input shape and keyset enum, lexical path class, and
content-marker/length flags. Exact relative, normalized relative, normalized
absolute same-fixture, other inside, outside and invalid paths are distinct.
Normalization is classification only: all admission predicates, exact relative
path/content checks, denial, the original 300000ms bound and no-replay behavior
remain unchanged. Raw tools, unknown keys, paths and content are never retained.

`scripts/fixtures/native-permission-v2.json` was captured from the real owning
ACP/agent/tools binaries in the same bounded Docker image and read-only source
build cache. A deterministic local HTTP provider supplied only synthetic tool
arguments; no model, user profile or provider credentials were involved. The
actual emitted v2 request passes the independent fixture unchanged; the full
turn completed with the exact permitted file effect. That fixture also drives
refusal controls, rather than assuming a hand-authored request shape is the
actual emitted protocol.

The narrow missing-classification regression failed once before the change
(KeyError for absent rejection metadata) and now passes. All 17 Python controls
pass, including actual emitted-wire admission, classified rejection and secret
absence. A self-review control also proved that invalid UTF-8 surrogate arguments
could mask rejection during diagnostic encoding; it failed once before the
classifier was made non-throwing for those values, and now passes. Full portable and exact-source Node gates are recorded with the enclosing
change. Fresh native qualification must establish the refused predicate before
any admission change is considered. No old native run was retried.

## Exact native tool instruction

Linux ARM64 run `44249353447`, source `0861b518`, failed at source-native
permission admission: `write_file`, correct keyset, 12-byte matching marker,
`other_inside_fixture`. The actual path was not retained and is unknown. The
three compiled session-root controls prove unchanged arguments and correct
session rooting; no transport or product defect is evidenced. The original
macOS queue timeout remains separately unexplained.

The prompt now specifies the exact relative tool JSON and explicitly forbids
an added directory component. This is a new committed-source qualification
attempt of the same native model coding/permission task. The real model must
still emit the call, obtain a fresh scoped allow_once, perform the actual write
and complete. No file is precreated or model response replaced in native
qualification. The admission predicate, safe target/content (128-byte bound),
symlink/outside/shell restrictions, fresh identity checks, MAX16, one 300000ms
absolute deadline, no uncertain replay/retry and terminal/cleanup checks are
unchanged. Diagnostics remain bounded classifications and hashes.

Exact regression command (default bounded Docker image, read-only built cache):
`python3 -B scripts/native-instruction.test.py /cache/target/debug NativeInstructionTests.test_exact_instruction_reaches_provider_and_real_tools_write`.
Before the prompt change: one failure, missing exact JSON on actual provider
wire. After: one pass, zero failures. The full compiled controls pass three
cases: exact instruction/file effect plus added-session-directory and other
inside-path rejection, with unchanged wire arguments, absent preapproval target,
no launch-root effect and no provider retry. These synthetic path names are
negative controls, not inferred values of the unknown native path. Existing
17 Python lifecycle/evidence controls pass unchanged.

The explicit bounded compiled-control command runs against public binaries built
by the existing build lane. It remains separate from Node-only governance.
They preserve the previously confirmed session-root checks rather than changing
cwd behavior. Full portable and exact-source validation results follow in the
verified enclosing change. Milestone review: M0–M5 statuses, runtime contract,
platform/model and release gates remain unchanged. Actual Linux native remains
one observed failure with no after-run yet; both ARM source/bundle hosted
qualifications remain Root-owned. Portable passes cannot claim this native
failure fixed. No original failed run or uncertain side effect was replayed.

The first full matrix found one intake failure: adding a root task alias changed
the bootstrap manifest frozen by the existing intake guard. The alias was removed
without changing the manifest. No
bootstrap digest, test assertion or guard was relaxed. Its immutable run report
is `lapis-ai-run-955b8551-39fd-472a-97b0-49871b4081a0`.

The second full matrix found one whole-recipe binding failure because the
validation lanes are part of the 16 frozen qualified release inputs. The lane
change was removed, preserving that binding and keeping compiled controls an
explicit bounded acceptance command after build. The exact whole-recipe test
now passes without changing its assertions or qualification metadata. Immutable
report: `lapis-ai-run-6e4311f0-9ba4-46fc-ada9-9f9a8127a776`.

Final default 4-CPU/12-GiB Docker `validation:all` passed in
`lapis-ai-run-86457f33-a28a-4981-8b46-c31e2b0fcea3`: check, tests, build,
workflow, package and all 11 spec lanes. Rust: 1078 passed, zero failed, two
existing opt-in ignores; client: 43; intake: 46; governance: 73. The final three
compiled instruction controls also pass against the resulting public helpers.
The exact-source Node CI gate passes. Both newly exposed binding regressions
are now zero failures; native Linux remains one observed failure without an
after-run, and actual macOS/Linux model/bundle qualification stays Root-owned.
