import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import { parse } from "yaml";

test("native fixture protocol and metadata controls pass without models or provider credentials", () => {
  execFileSync("python3", ["scripts/native-acceptance.test.py"], {
    stdio: "pipe",
    env: {
      PATH: process.env.PATH,
      TMPDIR: process.env.TMPDIR,
      PYTHONDONTWRITEBYTECODE: "1",
    },
    timeout: 30000,
  });
});
test("native failures retain only bounded metadata and never cancel their own workflow", () => {
  const source = readFileSync(".github/workflows/ci.yml", "utf8");
  const workflow = parse(source);
  assert.equal(workflow.permissions.actions, "read");
  assert.equal(workflow.jobs.native.strategy["fail-fast"], true);
  assert.deepEqual(
    workflow.jobs.native.strategy.matrix.include.map((x) => x.target),
    ["aarch64-unknown-linux-gnu", "aarch64-apple-darwin"],
  );
  assert.equal(workflow.jobs.native["timeout-minutes"], 90);
  assert(!source.includes("gh run cancel"));
  const steps = workflow.jobs.native.steps;
  const cold = steps.find(
    (x) => x.run === "bash scripts/native-qualification.sh debug",
  );
  assert(cold);
  assert.equal(cold["continue-on-error"], undefined);
  const upload = steps.find(
    (x) => x.name === "Retain bounded native failure evidence",
  );
  assert.equal(upload.if, "failure()");
  assert.equal(upload.with.path, ".local/native-diagnostics/debug/*.json");
  assert.equal(upload.with["retention-days"], 7);
  assert.equal(upload["continue-on-error"], undefined);
});
