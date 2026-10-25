import assert from "node:assert/strict";
import { test } from "node:test";
import { readFileSync } from "node:fs";
import {
  auditRouting,
  dispatch,
  imageKey,
  nativeInvocation,
  sourcePathViolation,
  validationConfig,
} from "./validation/model.mjs";

test("portable validation defaults to bounded Docker and accepts only explicit native selection", () => {
  assert.deepEqual(validationConfig({}, {}), {
    runtime: "docker",
    cpus: 4,
    memoryGiB: 12,
    shmGiB: 2,
  });
  assert.equal(
    validationConfig({}, { LAPIS_AI_VALIDATION_RUNTIME: "native" }).runtime,
    "native",
  );
  assert.throws(() =>
    validationConfig({}, { LAPIS_AI_VALIDATION_RUNTIME: "auto" }),
  );
});

test("invalid resource limits, transports and archive inputs cannot enter workers", () => {
  for (const value of [0, -1, NaN, Infinity, "4"])
    assert.throws(() => validationConfig({ cpus: value }, {}));
  assert.throws(() => validationConfig({ context: "" }, {}));
  assert.throws(() => validationConfig({ transport: "automatic" }, {}));
  assert.throws(() => validationConfig({ archives: "models.json" }, {}));
});

test("focused test names and shell syntax remain literal arguments", () => {
  const command = nativeInvocation(
    { test: "node --test" },
    "test",
    ["file with spaces.test.mjs", "$(touch unsafe)"],
    "/repo",
    {},
  );
  assert.deepEqual(command.args, [
    "-c",
    'node --test "$@"',
    "validation",
    "file with spaces.test.mjs",
    "$(touch unsafe)",
  ]);
  assert.equal(command.env.LAPIS_AI_VALIDATION_RUNTIME, "native");
  assert.throws(() => nativeInvocation({}, "missing", [], "/repo", {}));
});

test("dependency and tool identities determine the cache key independently of key order", () => {
  assert.equal(
    imageKey({ lock: "one", arch: "arm64" }),
    imageKey({ arch: "arm64", lock: "one" }),
  );
  assert.notEqual(imageKey({ lock: "one" }), imageKey({ lock: "two" }));
});

test("every existing root validation script is classified and Docker-routed", () => {
  const pkg = JSON.parse(readFileSync("package.json", "utf8"));
  const lanes = JSON.parse(
    readFileSync("scripts/validation/lanes.json", "utf8"),
  );
  assert.deepEqual(auditRouting(pkg.scripts, lanes), []);
});

test("new names and changed native exceptions cannot silently bypass Docker", () => {
  assert.notDeepEqual(
    auditRouting({ "check:new": "node unchecked.mjs" }, {}),
    [],
  );
  assert.notDeepEqual(
    auditRouting({ test: "node --test" }, { test: "node --test" }),
    [],
  );
  assert.notDeepEqual(auditRouting({ "dev:cargo": "cargo test" }, {}), []);
  assert.deepEqual(
    auditRouting(
      { test: "node scripts/validation.mjs run test" },
      { test: "node --test" },
    ),
    [],
  );
});

test("Docker errors remain failures without calling the host implementation", async () => {
  let native = false;
  await assert.rejects(
    dispatch(
      { runtime: "docker" },
      () => {
        native = true;
      },
      () => {
        throw Error("daemon unavailable");
      },
    ),
    /daemon unavailable/,
  );
  assert.equal(native, false);
});

test("private cache, secret and model payload paths reject recursively while source modules remain valid", () => {
  for (const path of [
    ".env",
    "packages/ai/.env.local",
    "nested/.local/private.json",
    "crates/helper/target/debug/binary",
    "packages/ai/node_modules/file.js",
    ".ssh/id_ed25519",
    "nested/secrets/provider.json",
    "nested/token.pem",
    "assets/model.gguf",
    "fixtures/model.onnx",
    "nested/model.safetensors",
    "nested/.cache/value.json",
    "nested/credentials.json",
    "../outside.rs",
  ])
    assert.ok(sourcePathViolation(path), path);
  for (const path of [
    "crates/lapis-compute/src/models.rs",
    "Cargo.lock",
    "scripts/validation.test.mjs",
    "packages/ai/src/index.ts",
    "crates/lapis-agent/data/model-capabilities.json",
  ])
    assert.equal(sourcePathViolation(path), false, path);
});

test("cleanup retains active, prepared and uncertain leases", async () => {
  const { leaseReleaseAllowed, stoppedRunState } =
    await import("./validation/model.mjs");
  assert.equal(leaseReleaseAllowed([]), true);
  for (const value of [undefined, null, ["active-id"]])
    assert.equal(leaseReleaseAllowed(value), false);
  for (const state of [
    "running",
    "created",
    "paused",
    "restarting",
    "removing",
    undefined,
  ])
    assert.equal(stoppedRunState(state), false);
  for (const state of ["exited", "dead"])
    assert.equal(stoppedRunState(state), true);
});

test("tool images ignore dependency-only changes while native caches keep their own identity", async () => {
  const { validationKeys } = await import("./validation/model.mjs");
  const inputs = {
    "scripts/validation/Dockerfile": "recipe",
    "scripts/validation/install-tools.mjs": "tools",
    packageManager: "pnpm@10.34.5",
    toolchain: "1.99.0",
    architecture: "x86_64",
    "package.json": "manifest",
    "pnpm-lock.yaml": "one",
    "Cargo.lock": "rust-one",
  };
  const initial = validationKeys(inputs);
  const dependencies = validationKeys({
    ...inputs,
    "pnpm-lock.yaml": "two",
    "Cargo.lock": "rust-two",
    "package.json": "changed-script",
  });
  assert.equal(initial.image, dependencies.image);
  assert.notEqual(initial.cache, dependencies.cache);
  for (const key of [
    "scripts/validation/Dockerfile",
    "scripts/validation/install-tools.mjs",
    "packageManager",
    "toolchain",
    "architecture",
  ])
    assert.notEqual(
      initial.image,
      validationKeys({ ...inputs, [key]: "changed" }).image,
    );
});
