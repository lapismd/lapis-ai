# Milestones

| Task | Status      | Dependencies | Gate                                                                   | Evidence                                                                   | Next action                                                |
| ---- | ----------- | ------------ | ---------------------------------------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------- |
| M0   | Done        | none         | Standalone workspace, governance, imports and provenance               | [Foundation](../../history/2026-10-16-foundation.md)                       | Maintain source pins and frozen budgets                    |
| M1   | Done        | M0           | Renamed ACP and retained deterministic suites                          | [Foundation](../../history/2026-10-16-foundation.md)                       | Consumer adoption is separate                              |
| M2   | Done        | M1           | Agent, tools and compute imports with fixture acceptance               | [Foundation](../../history/2026-10-16-foundation.md)                       | Preserve provider and wire boundaries                      |
| M3   | Done        | M2           | TOML, clients, isolation and Rust examples                             | [Foundation](../../history/2026-10-16-foundation.md)                       | Maintain direct client acceptance                          |
| M4   | Done        | M3           | macOS ARM64 Metal, Linux aarch64 CPU, mixed-version and staged bundles | [Qualification](../../history/2026-10-16-qualification.md)                 | Qualify broader native and release profiles separately     |
| M5   | In progress | M4           | Hosted CI, Changesets, Cargo release automation and spec Pages         | [Organization transfer](../../history/2026-10-16-organization-transfer.md) | Verify hosted product CI and retain version PR #1 unmerged |

## Consumer-driven native adapter

This work does not depend on or change M5 release authorization. Desktop M10.1
uses local verified public outputs and remains incomplete until Tauri/native
acceptance passes.

| Slice           | Status      | Gate                                                                                        | Evidence                                                              | Next action                                            |
| --------------- | ----------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------ |
| ACP session     | Done        | Correlation, exact permissions, ordered bounded reconnect, isolation and supervised cleanup | [Owned ACP session](../../history/2026-10-17-acp-session.md)          | Complete; continue to host-scoped Tauri transport      |
| Rust Tauri host | Done        | Host-scoped native policy, bounded commands/channels and supervised startup revocation      | [Native Tauri transport](../../history/2026-10-17-tauri-transport.md) | Continue to renderer and native consumer qualification |
| Tauri transport | In progress | Host-scoped commands/channels, permissions, renderer client and cold native acceptance      | [Native Tauri transport](../../history/2026-10-17-tauri-transport.md) | Implement renderer client and cold packaged acceptance |
