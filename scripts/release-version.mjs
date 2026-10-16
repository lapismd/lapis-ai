import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import TOML from "@iarna/toml";
export const read = (root, path) => readFileSync(join(root, path), "utf8");
export const manifest = (root, path) => TOML.parse(read(root, path));
export const json = (root, path) => JSON.parse(read(root, path));
export const policy = (root = ".") => manifest(root, "release.toml");
export const crates = policy().crates;
export const allCrates = [...crates, ...policy().native_only];
export const coordinator = policy().coordinator;
export const cratePath = (name) =>
  `crates/${name === "lapis-acp" ? "lapis-acp-cli" : name}`;
export function version(root = ".") {
  const selected = manifest(root, "Cargo.toml").workspace.package.version;
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(selected))
    throw Error("invalid stable workspace version");
  return selected;
}
export function validateTag(tag, selected) {
  if (tag !== `v${selected}`)
    throw Error("tag does not match workspace version");
}
export function validateReleaseSource(root = ".") {
  const directory = join(root, ".changeset");
  if (
    !existsSync(directory) ||
    readdirSync(directory, { withFileTypes: true }).some(
      (entry) =>
        entry.isDirectory() ||
        entry.name === "pre.json" ||
        (entry.name.endsWith(".md") && entry.name !== "README.md"),
    )
  )
    throw Error("tagged source contains pending or invalid changesets");
}
