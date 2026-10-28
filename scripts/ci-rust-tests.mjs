import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const archive =
  process.env.LAPIS_AI_TEST_ARCHIVE || ".release/tests-canonical.tar.zst";
const features = ["--features", "lapis-agent/test-support", "--locked"];

function cargo(args) {
  const result = spawnSync("pnpm", ["dev:cargo", "--", ...args], {
    stdio: "inherit",
  });
  if (result.error) throw result.error;
  if (result.status) process.exit(result.status);
}

export function archiveTests() {
  mkdirSync(".release", { recursive: true });
  cargo([
    "nextest",
    "archive",
    "--workspace",
    ...features,
    "--archive-file",
    archive,
  ]);
}

/** Same-tree run: required because tests bake env!("CARGO_BIN_EXE_*") paths. */
export function runWorkspace() {
  cargo(["nextest", "run", "--workspace", ...features]);
}

if (import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const mode = process.argv[2];
  if (mode === "archive") archiveTests();
  else if (mode === "run") runWorkspace();
  else throw new Error("Usage: ci-rust-tests.mjs archive|run");
}
