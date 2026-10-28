# Local portable validation

Root validation commands use Docker by default. A run checks the exact tracked
Jujutsu revision and ancestry in a fresh worker, installs the committed frozen
pnpm lock without local hooks, fetches every locked Rust target dependency for
offline governance checks, and runs the selected lane. Docker, transfer,
installation and test failures stay failures. There is no host fallback.

Choose a Docker context in ignored `.local/validation/config.json`:

```json
{
  "context": "omarchy",
  "transport": "ssh-cli",
  "cpus": 4,
  "memoryGiB": 12,
  "shmGiB": 2
}
```

`ssh-cli` uses the selected Docker SSH endpoint with batch authentication. Omit
it to use Docker's normal context transport. No daemon socket, host directories,
models or credentials are mounted in workers. Only the owned tool/dependency
cache volume is shared. Workers use Docker’s process reaper and drop all Linux
capabilities, so orphan cleanup and permission-denial checks remain meaningful. Recursive source guards reject local caches, secrets
and model payloads in the selected tree and transferred ancestry. The
self-contained Cargo workspace requires no sibling sources or package archives.

```sh
pnpm validation:doctor
pnpm validation:smoke
pnpm validation:all
pnpm check
pnpm test
pnpm build
pnpm spec:check
pnpm package:check
```

Focused arguments are passed literally after `--`. `pnpm validation:images`
prepares or reuses the fingerprinted image; `-- --rebuild` forces an image rebuild.
Tools are pinned by image digest or archive checksum: Rust 1.99.0, pnpm 10.34.5,
Jujutsu 0.45.1, mdBook 0.5.4 and actionlint 1.7.12. No browser is installed: this
repository has no Storybook or UI catalog. Renderer tests remain Node fixtures.

The default worker has 4 CPUs, 12 GiB memory, 2 GiB shared memory and one Cargo
job. Coordinate concurrent repositories so their combined resource limits fit
the selected host. Tool-image keys include only recipe/tool-install inputs, extracted pnpm/Rust
versions and architecture. Dependency/native cache keys separately include the
lockfiles and workspace manifests, so ordinary script or dependency intake does
not rebuild tool images. Exact source revisions remain distinct from caches.

Each run writes `.local/validation/runs/<run>/request.json`, `result.json` and
`console.log`; client `dist` and specification HTML are collected there too.
Native Cargo targets and models are never copied back. `pnpm validation --
collect <run>` retries collection. `pnpm validation:clean` previews only this
checkout's completed runs; `-- --apply` collects their results before removing
them. Active, prepared and uncertain runs retain their leases. Cleanup does not
prune global Docker state or other checkouts' work.

Use `LAPIS_AI_VALIDATION_RUNTIME=native pnpm <lane>` for explicit host validation.
Formatting/Changeset authoring, spec search, artifact staging and `dev:cargo`
remain native commands. The existing provisioned CI explicitly selects native
execution to preserve its artifact paths. `scripts/native-qualification.sh`
also selects native execution before testing actual providers/models and bundles.
Hosted CI runs `spec:check` and Node governance before any Rust toolchain or
`cargo fetch`. Rust build archives tests with nextest; the Rust test job only
runs that archive so failed-job re-runs skip compile. Hosted native jobs build
once with the same Cargo features as the test lane, then set
`LAPIS_AI_SKIP_NATIVE_BUILD=1` so qualification does not recompile.
Portable Docker checks do not replace macOS Metal, C++ runtime, real-model,
provider, native Tauri, release or broader platform qualification. No publication
or execution-backend change is authorized by this development tooling.
