import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  mkdtemp,
  rm,
  writeFile,
  symlink,
  rename,
  realpath,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import TOML from "@iarna/toml";
import { helperNames, nativeTarget, verifyBundle } from "./bundle.mjs";

async function fixture(t) {
  const root = await realpath(
    await mkdtemp(join(tmpdir(), "lapis-ai-bundle-")),
  );
  t.after(() => rm(root, { recursive: true, force: true }));
  const manifest = {
    version: 1,
    package_version: "fixture",
    target: nativeTarget(),
    binary: [],
  };
  for (const name of helperNames) {
    await writeFile(join(root, name), name);
    manifest.binary.push({
      name,
      path: name,
      sha256: createHash("sha256").update(name).digest("hex"),
    });
  }
  const save = () =>
    writeFile(join(root, "runtime-manifest.toml"), TOML.stringify(manifest));
  await save();
  return { root, manifest, save };
}
test("development bundles verify exact resources before consumer staging", async (t) => {
  const { root, manifest, save } = await fixture(t);
  const bundle = await verifyBundle(root, { version: "fixture" });
  assert.equal(bundle.helpers["lapis-agent"], join(root, "lapis-agent"));
  await assert.rejects(
    verifyBundle(root, { version: "other" }),
    /Incompatible/,
  );
  manifest.target = "unsupported";
  await save();
  await assert.rejects(
    verifyBundle(root, { version: "fixture" }),
    /Incompatible/,
  );
  manifest.target = nativeTarget();
  await save();
  await writeFile(join(root, "lapis-agent"), "tampered");
  await assert.rejects(verifyBundle(root, { version: "fixture" }), /checksum/);
});
test("development bundle verification rejects internal symlinks", async (t) => {
  const { root } = await fixture(t);
  await rename(join(root, "lapis-agent"), join(root, "actual-agent"));
  await symlink("actual-agent", join(root, "lapis-agent"));
  await assert.rejects(
    verifyBundle(root, { version: "fixture" }),
    /regular files/,
  );
});
