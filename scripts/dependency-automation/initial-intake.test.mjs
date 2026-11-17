import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import test from "node:test";
import { initialIntake, sourceBaseline } from "./initial-intake.mjs";
const read = (p) =>
  readFileSync(new URL("../../" + p, import.meta.url), "utf8");
const metadataBytes = read(".github/dependency-automation/bootstrap.json");
const metadata = JSON.parse(metadataBytes);
const policyBytes = read(".github/dependency-automation/canonical-intake.json");
const { before } = JSON.parse(
  gunzipSync(
    readFileSync(
      new URL("../fixtures/initial-validator-inputs.json.gz", import.meta.url),
    ),
  ),
);
const files = Object.fromEntries(
  Object.keys(metadata.inputs).map((p) => [p, read(p)]),
);
const input = () => ({
  sourceSha: metadata.baselineSource,
  files: structuredClone(files),
  before: structuredClone(before),
  policyBytes,
  metadataBytes,
});
test("only the exact reviewed initial source and before/after inputs bootstrap", () => {
  assert.equal(initialIntake(input()), true);
  assert.equal(
    sourceBaseline({ ...input(), readPrevious: (p) => before[p] }).bootstrap,
    true,
  );
});
for (const [name, mutate] of [
  ["after manifest", (d) => (d.files["package.json"] += " ")],
  ["after lock", (d) => (d.files["pnpm-lock.yaml"] += " ")],
  ["before workspace", (d) => (d.before["pnpm-workspace.yaml"] += " ")],
  ["private client", (d) => (d.files["packages/ai/package.json"] += " ")],
  ["policy", (d) => (d.policyBytes += " ")],
  ["metadata", (d) => (d.metadataBytes += " ")],
  ["unknown inputs", (d) => (d.files["unknown.json"] = "{}")],
  ["missing ancestry", (d) => (d.sourceSha = undefined)],
])
  test("initial intake rejects changed " + name, () => {
    const d = input();
    mutate(d);
    assert.throws(() => initialIntake(d));
  });
test("foreign/future ancestry never reinitializes a missing source policy", () => {
  const d = input();
  d.sourceSha = "f".repeat(40);
  assert.equal(initialIntake(d), false);
  assert.throws(
    () =>
      sourceBaseline({
        ...d,
        readPrevious: (p) => {
          if (p in before) return before[p];
          throw Error("missing prior policy");
        },
      }),
    /missing prior policy/,
  );
});
test("ordinary future source requires its exact policy and both prior slots", () => {
  const d = input();
  d.sourceSha = "f".repeat(40);
  const previous = {
    ...files,
    ".github/dependency-automation/canonical-intake.json": policyBytes,
  };
  for (const p of JSON.parse(policyBytes).chapters) previous[p] = read(p);
  assert.equal(
    sourceBaseline({
      ...d,
      readPrevious: (p) => {
        if (p in previous) return previous[p];
        throw Error("missing");
      },
    }).bootstrap,
    false,
  );
  delete previous[JSON.parse(policyBytes).chapters[1]];
  assert.throws(
    () =>
      sourceBaseline({
        ...d,
        readPrevious: (p) => {
          if (p in previous) return previous[p];
          throw Error("missing prior chapter");
        },
      }),
    /missing prior chapter/,
  );
});
