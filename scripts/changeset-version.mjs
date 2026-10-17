import { writeFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import TOML from "@iarna/toml";
import {
  allCrates,
  cratePath,
  manifest,
  read,
  version,
  json,
} from "./release-version.mjs";
import { checkRelease, pendingChangesets } from "./release-state.mjs";
export function synchronizeVersion(root, selected) {
  const previous = version(root);
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(selected))
    throw Error("stable version required");
  const before = previous.split(".").map(BigInt),
    after = selected.split(".").map(BigInt);
  const changed = after.findIndex((value, index) => value !== before[index]);
  if (changed < 0 || after[changed] < before[changed])
    throw Error("version preparation requires a newer stable version");
  const cargo = manifest(root, "Cargo.toml");
  const client = json(root, "packages/ai/package.json");
  client.version = selected;
  writeFileSync(
    join(root, "packages/ai/package.json"),
    JSON.stringify(client, null, 2) + "\n",
  );
  cargo.workspace.package.version = selected;
  for (const [name, dependency] of Object.entries(cargo.workspace.dependencies))
    if (allCrates.includes(name)) dependency.version = selected;
  writeFileSync(join(root, "Cargo.toml"), TOML.stringify(cargo));
  for (const path of [
    "README.md",
    ...allCrates.map((name) => `${cratePath(name)}/README.md`),
  ])
    if (existsSync(join(root, path)))
      writeFileSync(
        join(root, path),
        read(root, path).replaceAll(
          `--version ${previous} --locked`,
          `--version ${selected} --locked`,
        ),
      );
}
export async function prepareVersion(
  root = ".",
  execute = (command, args) =>
    execFileSync(command, args, { cwd: root, stdio: "inherit" }),
) {
  const original = checkRelease(root),
    notes = await pendingChangesets(root);
  if (!notes.some((note) => note.releases.length))
    return { changed: false, version: original };
  const run = (command, args) => execute(command, args);
  try {
    run("node", [
      "scripts/managed-cargo.mjs",
      "fetch",
      "--locked",
      "--offline",
    ]);
  } catch (cause) {
    throw Error(
      "Run pnpm dev:cargo -- fetch --locked before offline version preparation",
      { cause },
    );
  }
  run("pnpm", ["exec", "changeset", "version"]);
  const selected = json(root, "package.json").version;
  synchronizeVersion(root, selected);
  run("node", [
    "scripts/managed-cargo.mjs",
    "update",
    "--workspace",
    "--offline",
  ]);
  run("pnpm", ["install", "--lockfile-only", "--offline", "--ignore-pnpmfile"]);
  run("pnpm", ["release:check"]);
  run("pnpm", [
    "exec",
    "prettier",
    "--write",
    "package.json",
    "packages/ai/package.json",
    "pnpm-lock.yaml",
    "README.md",
    "CHANGELOG.md",
    ...allCrates.map((name) => `${cratePath(name)}/README.md`),
  ]);
  return { changed: true, version: selected };
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  console.log(await prepareVersion());
