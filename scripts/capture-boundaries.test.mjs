import test from "node:test";
import assert from "node:assert/strict";
import { allowed, boundaryErrors } from "./boundaries.mjs";

test("source capture cannot acquire repository, process or transport dependencies", () => {
  const packages = Object.keys(allowed).map((name) => ({
    name,
    dependencies: [],
  }));
  const capture = packages.find((p) => p.name === "lapis-ai-capture");
  for (const name of [
    "jj-store-model",
    "lapis-ai-runtime",
    "lapis-agent",
    "tokio",
    "reqwest",
    "tauri",
  ]) {
    capture.dependencies = [{ name }];
    assert.ok(boundaryErrors(packages).length, name);
  }
  capture.dependencies = [
    { name: "serde" },
    { name: "serde_json" },
    { name: "tempfile", kind: "dev" },
  ];
  assert.deepEqual(boundaryErrors(packages), []);
});
