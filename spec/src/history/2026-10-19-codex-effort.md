# Codex compact effort compatibility

Revision: enclosing Jujutsu change `qyqnxumr`, based on `e12d303c`.

Milestone review: Notes local-file Codex chat is the user-selected priority.
This private Control V3 adapter profile retains M0–M5 and all publication gates.
It does not qualify JJ transcript capture or application tools. The user deferred
broader verification while the functional plugin is delivered.

## Contract review

LAI-RUN-006/009 and LAI-CTRL-009 gain explicit compatibility for the identified
`@agentclientprotocol/codex-acp` adapter. It combines an explicitly selected,
advertised model with requested effort by selecting an exact advertised variant
within that family. Independent/keyed effort surfaces still take precedence.
Generic LAI-PROTO-003 parsing remains opaque, including its original positional
bracket refusal test. No public Rust type/signature or wire fields change.
Both model and effort report the actual model-setting outcome, including refusal.

## Commands

- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-acp-protocol --test codex_effort --locked` (initial discarded generic proposal)
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-acp-protocol -p lapis-acp-agent -p lapis-acp-control --locked` (finds deliberate generic refusal)
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-acp-control codex_split_catalog --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-acp-control codex_profile --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- test -p lapis-acp-protocol -p lapis-acp-control -p lapis-acp-runner --locked`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- clippy -p lapis-acp-control -p lapis-acp-protocol -p lapis-acp-runner --all-targets --locked -- -D warnings`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 pnpm dev:cargo -- build -p lapis-acp --bin lapis-acp --locked`
- `pnpm package:stage -- --profile debug --destination dist/codex-local-review --native-runtime /Users/stevejuma/code/lapismd/jj-store/.local/lapis-ai/native-runtime`

## Results

Initial private profile passes two exact tests and 88 directly affected tests.
A live no-prompt Notes probe confirms user-approval mode and exact model, but
reports unsupported effort: Codex advertises base stable model options and
compact variants under `models.availableModels`. Added that exact catalog case;
Final three profile cases, all89 affected protocol/control/runner tests, the
CLI fleet case, strict affected Clippy, source audits, progress, first gate and
all5 spec validators pass. Notes cold native Codex replies with the exact
synthetic smoke text and accepts cancellation with partial output. Its final
attachment/restart work remains in Notes E-AI-042. Artifacts are ignored `.local`.

The staged private bundle is `dist/codex-local-review-v2`; it contains all four
helpers and verified native ABI0.1.44. Only ACP changed. ACP SHA256:
`e78e569ce07ceaa2713ea3fa0df8c2a16d7d2ba0bb378a22f07fe695c883e5be`.
Source logs: `.local/codex-profile-{final-exact,final-affected,cli-fleet,
clippy-final,stage-v2,spec-first,spec,source}.log`.

## Failures

The discarded generic-parser proposal fails two new tests before editing. It
then passes them but fails the original positional-bracket refusal case (13/14).
Restored generic source and tests unchanged; the final implementation belongs
in the identified Codex profile. The first source build command incorrectly used
package `lapis-acp-cli`; corrected to the manifest's package `lapis-acp`.
The split-catalog regression fails one case before the profile reads both
catalogs. No assertion or generic contract was weakened. The first live profile read
only stable options; the live adapter stores variants in the separate models
list. Its exact new split-catalog case failed once, then passed after adding
that list. Staging refused an existing destination; selected a fresh v2 bundle
and preserved the prior bundle.

## Gaps

Full unrelated workspace/compute/platform/release qualification is deferred by
the user. Control V2 and one-shot configuration keep their existing generic
behavior; this compatibility profile is private to Control V3. Notes E-AI-042
owns cold native chat, local persistence and restart acceptance. No credentials,
auth files, model weights, tools or prompts enter source evidence.

Next action: finish Notes local file-context/restart review. Retain broader
source/platform/publication qualification as deferred.

Gate result: passed (focused Control V3 Codex compatibility only)
