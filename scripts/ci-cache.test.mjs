import assert from "node:assert/strict";
import { test } from "node:test";
import { cacheWritable } from "./ci-cache.mjs";

const base = {
  event: "pull_request",
  repository: "lapismd/lapis-ai",
  headRepository: "lapismd/lapis-ai",
};

test("same-repo pull requests may write Rust dependency caches", () => {
  assert.equal(cacheWritable(base), true);
});

test("fork pull requests stay read-only for Rust caches", () => {
  assert.equal(
    cacheWritable({ ...base, headRepository: "outsider/lapis-ai" }),
    false,
  );
});

test("main pushes and workflow dispatch may write caches", () => {
  assert.equal(
    cacheWritable({
      event: "push",
      repository: "lapismd/lapis-ai",
    }),
    true,
  );
  assert.equal(
    cacheWritable({
      event: "workflow_dispatch",
      repository: "lapismd/lapis-ai",
    }),
    true,
  );
});

test("foreign repositories never write this cache", () => {
  assert.equal(
    cacheWritable({
      event: "push",
      repository: "other/lapis-ai",
    }),
    false,
  );
});
