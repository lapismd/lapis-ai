import { execFileSync } from "node:child_process";
import {
  readFileSync,
  mkdtempSync,
  writeFileSync,
  appendFileSync,
  chmodSync,
  rmSync,
} from "node:fs";
import assert from "node:assert/strict";
import TOML from "@iarna/toml";
import { validationConfig } from "./model.mjs";

if (process.env.LAPIS_AI_VALIDATION_WORKER !== "1")
  throw Error("Docker smoke must run in the isolated worker");
if (validationConfig().runtime !== "native")
  throw Error("Worker command dispatch must be explicit");
const effective = readFileSync("/proc/self/status", "utf8").match(
  /^CapEff:\s*(.+)$/m,
)?.[1];
assert.match(
  effective ?? "",
  /^0+$/,
  "Worker must drop all Linux capabilities",
);
assert.match(
  readFileSync("/proc/1/cmdline", "utf8"),
  /docker-init|tini/,
  "Worker must have a process reaper",
);
const denied = mkdtempSync("/tmp/lapis-ai-permission-"),
  probe = denied + "/probe";
writeFileSync(probe, "private");
chmodSync(denied, 0);
try {
  assert.throws(() => readFileSync(probe), { code: "EACCES" });
  assert.throws(() => appendFileSync(probe, "forbidden"), { code: "EACCES" });
} finally {
  chmodSync(denied, 0o700);
  rmSync(denied, { recursive: true });
}
execFileSync("node", ["--test", "scripts/validation.test.mjs"], {
  stdio: "inherit",
});
const manifests = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--no-deps", "--format-version=1", "--locked"],
    { encoding: "utf8" },
  ),
);
const expected = TOML.parse(readFileSync("Cargo.toml", "utf8")).workspace
  .members.length;
if (manifests.workspace_members.length !== expected)
  throw Error("Cargo workspace inventory mismatch");
console.log(
  `Docker smoke passed: exact source, frozen packages, ${expected} Rust packages, bounded worker`,
);
