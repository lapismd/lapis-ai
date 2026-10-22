import assert from "node:assert/strict";
import { test } from "node:test";
import { allowed, boundaryErrors } from "./boundaries.mjs";

function packages() {
  return Object.keys(allowed).map((name) => ({ name, dependencies: [] }));
}

test("neutral ACP service has only the runtime workspace edge", () => {
  assert.deepEqual(allowed["lapis-ai-service"], ["lapis-ai-runtime"]);
});

test("neutral ACP service rejects Tauri, network and database authority", () => {
  for (const name of ["tauri", "reqwest", "turso"]) {
    const graph = packages();
    graph.push({
      name: "lapis-ai-service",
      dependencies: [{ name, kind: null }],
    });
    assert.ok(
      boundaryErrors(graph).some(
        (error) => error === `lapis-ai-service: excluded ${name}`,
      ),
    );
  }
});
