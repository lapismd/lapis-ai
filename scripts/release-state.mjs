import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import readModule from "@changesets/read";
import {
  allCrates,
  crates,
  coordinator,
  cratePath,
  json,
  manifest,
  policy,
  read,
  version,
} from "./release-version.mjs";
export { allCrates, crates, coordinator, cratePath, json, manifest, read };
const readChangesets = readModule.default ?? readModule;
export function checkRelease(root = ".") {
  const selected = version(root),
    config = policy(root),
    pkg = json(root, "package.json");
  if (
    config.repository !== "lapismd/lapis-ai" ||
    config.version !== 1 ||
    config.coordinator !== coordinator ||
    config.crates.length !== 12 ||
    config.native_only.join(",") !== "lapis-compute"
  )
    throw Error("unexpected release policy");
  if (
    pkg.name !== coordinator ||
    pkg.private !== true ||
    pkg.version !== selected
  )
    throw Error("private coordinator and Cargo versions must match");
  const changesets = json(root, ".changeset/config.json");
  const client = json(root, "packages/ai/package.json");
  if (
    client.name !== "@lapismd/ai" ||
    client.private !== true ||
    client.version !== selected
  )
    throw Error("private renderer client and Cargo versions must match");
  if (
    changesets.privatePackages?.version !== true ||
    changesets.privatePackages?.tag !== false ||
    changesets.baseBranch !== "main"
  )
    throw Error(
      "Changesets must version the private coordinator without npm tags",
    );
  const workspace = manifest(root, "Cargo.toml").workspace;
  const locked = manifest(root, "Cargo.lock").package;
  if (workspace.members.length !== allCrates.length)
    throw Error("release policy omits workspace packages");
  for (const name of allCrates) {
    const path = cratePath(name),
      cargo = manifest(root, `${path}/Cargo.toml`).package;
    if (
      cargo.name !== name ||
      cargo.version?.workspace !== true ||
      !workspace.members.includes(path)
    )
      throw Error(`${name}: must inherit coordinated Cargo version`);
    const entries = locked.filter(
      (entry) => entry.name === name && !entry.source,
    );
    if (entries.length !== 1 || entries[0].version !== selected)
      throw Error(`${name}: lock version differs`);
    if (crates.includes(name) && cargo.publish?.workspace !== true)
      throw Error(`${name}: registry publication missing`);
    if (config.native_only.includes(name) && cargo.publish !== false)
      throw Error(`${name}: pinned native-only helper cannot publish`);
  }
  for (const [name, dependency] of Object.entries(workspace.dependencies))
    if (allCrates.includes(name) && dependency.version !== selected)
      throw Error(`${name}: internal registry version differs`);
  if (!read(root, "CHANGELOG.md").includes(`\n## ${selected}\n`))
    throw Error("current version missing from changelog");
  return selected;
}
export async function pendingChangesets(root = ".") {
  const notes = await readChangesets(resolve(root));
  for (const note of notes)
    for (const release of note.releases)
      if (
        release.name !== coordinator ||
        !["patch", "minor", "major"].includes(release.type)
      )
        throw Error("Changesets must target the coordinated Cargo release");
  return notes;
}
export async function releaseState(root = ".") {
  const selected = checkRelease(root),
    notes = await pendingChangesets(root);
  return {
    version: selected,
    tag: `v${selected}`,
    crates,
    pending: notes.length,
    changesets: notes.map(({ id, summary, releases }) => ({
      id,
      summary,
      releases,
    })),
  };
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  console.log(
    process.argv.includes("--plan")
      ? JSON.stringify(await releaseState(), null, 2)
      : `Coordinated release metadata verified at ${checkRelease()}`,
  );
