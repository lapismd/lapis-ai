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
    "turso",
    "sqlx",
  ]) {
    capture.features = { "opencode-sqlite": ["dep:rusqlite"] };
    capture.dependencies = [{ name }];
    assert.ok(boundaryErrors(packages).length, name);
  }
  capture.features = { "opencode-sqlite": ["dep:rusqlite"] };
  capture.dependencies = [
    { name: "serde" },
    { name: "serde_json" },
    { name: "sha2" },
    { name: "rusqlite", optional: true },
    { name: "tempfile", kind: "dev" },
  ];
  assert.deepEqual(boundaryErrors(packages), []);
  capture.dependencies = [{ name: "rusqlite", optional: false }];
  assert.ok(
    boundaryErrors(packages).length,
    "SQLite must remain an explicit optional source feature",
  );
});
