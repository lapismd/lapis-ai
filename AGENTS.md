# Lapis AI agent guide

This repository owns shared native ACP, agent, compute, configuration and helper
client code. Apply parent workspace guidance. Other repositories are reference
sources only; do not modify them as part of an implementation slice here.

Read the owning canonical contract under `spec/src/current` before changing code.
Protected changes update their owning requirement or record an explicit
unchanged-contract confirmation, actual validation and milestone review in the
same Jujutsu change. Commit each verified slice using an explicit message.

New production Rust, TypeScript and scripts stay within 300 counted lines;
tests stay within 500. Imported exceptions in `scripts/architecture-baseline.json`
never grow. Do not introduce new exceptions. New tests live in separate files.
Authored spec Markdown stays within 400 physical lines and 32768 UTF-8 bytes.

Use `pnpm dev:cargo -- ...` for Cargo work. Run the exact regression and affected
crate checks before widening. Final integration requires the complete workspace
suite. Spec checks include formatting, boundaries, architecture and progress.
Record native/model/platform gaps separately from deterministic fixture passes.

ACP owns no network, identity, credentials or durability. Applications own trust,
workspace admission, secrets, context and history. No uncertain prompt is replayed.
Tauri integration wraps the native runtime in its own adapter crate; application
admission and history remain host-owned. Node and Python are development tools only.
The private renderer client lives in `packages/ai`; consume its built public
exports. Keep its generic entry independent of the optional Tauri transport.
Release automation follows `spec/src/current/contracts/release.md`. Never merge
version PRs or publish packages/tags without the governing release authorization.
Do not install services, mutate user caches, or update source repositories.
