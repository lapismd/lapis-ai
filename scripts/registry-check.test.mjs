import test from "node:test";
import assert from "node:assert/strict";
import { verifyExisting } from "./registry-check.mjs";
test("missing_version_checks_metadata_before_cdn_download", async () => {
  const calls = [];
  assert.equal(
    await verifyExisting("new-crate", "0.1.0", async (url) => {
      calls.push(url);
      return new Response(null, {
        status: url.endsWith("/download") ? 403 : 404,
      });
    }),
    false,
  );
  assert.equal(calls.length, 1);
});
test("existing_version_download_errors_never_become_missing_versions", async () => {
  await assert.rejects(
    verifyExisting(
      "crate",
      "0.1.0",
      async (url) =>
        new Response(null, { status: url.endsWith("/download") ? 403 : 200 }),
    ),
    /download.*403/,
  );
});
test("existing_version_must_match_the_verified_package", async () => {
  const request = async (url) =>
    new Response(url.endsWith("/download") ? "verified" : "metadata");
  assert.equal(
    await verifyExisting("crate", "0.1.0", request, () =>
      Buffer.from("verified"),
    ),
    true,
  );
  await assert.rejects(
    verifyExisting("crate", "0.1.0", request, () => Buffer.from("different")),
    /different package contents/,
  );
});
