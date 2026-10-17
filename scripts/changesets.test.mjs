import test from "node:test";
import assert from "node:assert/strict";
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
  readdirSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { prepareVersion } from "./changeset-version.mjs";
import {
  checkRelease,
  coordinator,
  pendingChangesets,
  json,
} from "./release-state.mjs";
import {
  validateReleaseSource,
  allCrates,
  cratePath,
  manifest,
} from "./release-version.mjs";
const source = resolve(".");
function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "lapis-ai-changesets-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const path of [
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "packages",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "release.toml",
    "README.md",
    "CHANGELOG.md",
    "scripts",
    "crates",
    "spec/src",
  ])
    cpSync(join(source, path), join(root, path), {
      recursive: true,
      filter: (path) =>
        !["node_modules", "dist"].includes(path.split("/").at(-1)),
    });
  mkdirSync(join(root, ".changeset"));
  for (const path of ["config.json", "README.md"])
    cpSync(join(source, ".changeset", path), join(root, ".changeset", path));
  symlinkSync(join(source, "node_modules"), join(root, "node_modules"), "dir");
  return root;
}
function snapshot(root, paths) {
  return paths.map((path) => readFileSync(join(root, path), "utf8"));
}
const metadata = ["Cargo.toml", "Cargo.lock", "package.json", "CHANGELOG.md"];
test("no notes preserve coordinated version without spawning commands", async (t) => {
  const root = fixture(t),
    before = snapshot(root, metadata);
  assert.equal(
    (
      await prepareVersion(root, () => {
        throw Error("must not execute");
      })
    ).changed,
    false,
  );
  assert.deepEqual(snapshot(root, metadata), before);
  assert.doesNotThrow(() => validateReleaseSource(root));
});
test("missing offline Cargo bootstrap preserves notes and all version metadata", async (t) => {
  const root = fixture(t),
    before = snapshot(root, metadata);
  writeFileSync(
    join(root, ".changeset/cold.md"),
    `---\n"${coordinator}": patch\n---\n\nInitial release.\n`,
  );
  await assert.rejects(
    prepareVersion(root, () => {
      throw Error("cache unavailable");
    }),
    /fetch --locked before offline version preparation/,
  );
  assert.deepEqual(snapshot(root, metadata), before);
  assert.equal((await pendingChangesets(root)).length, 1);
});
test("real Changesets prepare every governed Cargo version without editing canonical evidence", async (t) => {
  const root = fixture(t),
    canonical = snapshot(root, [
      "spec/src/current/contracts/release.md",
      "spec/src/current/progress/milestones.md",
    ]);
  writeFileSync(
    join(root, ".changeset/initial.md"),
    `---\n"${coordinator}": patch\n---\n\nInitial coordinated release.\n`,
  );
  const previous = json(root, "package.json").version.split(".").map(Number);
  const next = `${previous[0]}.${previous[1]}.${previous[2] + 1}`;
  const result = await prepareVersion(root, (cmd, args) =>
    execFileSync(cmd, args, { cwd: root, stdio: "pipe" }),
  );
  execFileSync("pnpm", ["exec", "prettier", "--check", "pnpm-lock.yaml"], {
    cwd: root,
    stdio: "pipe",
  });
  assert.equal(result.version, next);
  assert.equal(checkRelease(root), next);
  assert.equal(json(root, "package.json").version, next);
  assert.equal(json(root, "packages/ai/package.json").version, next);
  const lock = manifest(root, "Cargo.lock").package;
  for (const name of allCrates)
    assert.equal(
      lock.find((pkg) => pkg.name === name && !pkg.source).version,
      next,
    );
  assert.equal((await pendingChangesets(root)).length, 0);
  assert.doesNotThrow(() => validateReleaseSource(root));
  assert.deepEqual(
    snapshot(root, [
      "spec/src/current/contracts/release.md",
      "spec/src/current/progress/milestones.md",
    ]),
    canonical,
  );
  assert.equal(
    manifest(root, `${cratePath("lapis-compute")}/Cargo.toml`).package.publish,
    false,
  );
});
test("arbitrary packages and mismatched coordinator versions are refused", async (t) => {
  const root = fixture(t);
  writeFileSync(
    join(root, ".changeset/wrong.md"),
    '---\n"other-package": patch\n---\n\nWrong coordinator.\n',
  );
  await assert.rejects(
    pendingChangesets(root),
    /coordinated Cargo release|not found|changeset/i,
  );
  const pkg = json(root, "package.json");
  pkg.version = "8.0.0";
  writeFileSync(join(root, "package.json"), JSON.stringify(pkg));
  assert.throws(() => checkRelease(root), /versions must match/);
});
