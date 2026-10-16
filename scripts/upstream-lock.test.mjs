import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import {
  DISPOSITIONS,
  buildLock,
  digestOf,
  driftReport,
  scratchDir,
  validateDisposition,
  validateLock,
} from "./upstream-lock.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const curated = JSON.parse(
  readFileSync(
    new URL("../docs/upstream/disposition.json", import.meta.url),
    "utf8",
  ),
);

const pin = {
  repository: "block/buzz",
  path: "crates/buzz-acp",
  commit: "2664d14316790a57ea44b3f97c57440b70e43436",
  license: "Apache-2.0",
  owners: ["crates/jj-acp-protocol", "none"],
  files: {
    "src/acp.rs": {
      disposition: "ported-with-changes",
      owner: "crates/jj-acp-protocol",
      note: "Decomposed under the counted-line budget.",
    },
    "src/relay.rs": {
      disposition: "dropped",
      owner: "none",
      note: "Nostr WebSocket intake and NIP-42 authentication.",
    },
  },
};

test("the_curated_port_map_is_valid", () => {
  assert.deepEqual(validateDisposition(curated), []);
  assert.equal(curated.commit.length, 40);
  assert.deepEqual(DISPOSITIONS, [
    "ported",
    "ported-with-changes",
    "dropped",
    "not-ported",
  ]);
});

test("every_curated_region_names_a_disposition_and_owner", () => {
  const owners = new Set(curated.owners);
  for (const [path, entry] of Object.entries(curated.files)) {
    assert.ok(DISPOSITIONS.includes(entry.disposition), path);
    assert.ok(owners.has(entry.owner), `${path} owner ${entry.owner}`);
    assert.ok(entry.note.length >= 8, path);
  }
});

test("dropped_regions_carry_a_reason_and_no_owner", () => {
  for (const [path, entry] of Object.entries(curated.files)) {
    if (entry.disposition !== "dropped") continue;
    assert.equal(entry.owner, "none", path);
    assert.match(entry.note, /Nostr|relay|signing|Pi harness/i, path);
  }
});

test("rejects_a_malformed_pin", () => {
  assert.match(
    validateDisposition({ ...pin, commit: "deadbeef" }).join(),
    /40-character lowercase hexadecimal/,
  );
  assert.match(
    validateDisposition({ ...pin, repository: "buzz" }).join(),
    /owner\/name slug/,
  );
  assert.match(validateDisposition({ ...pin, license: "" }).join(), /license/);
  assert.match(
    validateDisposition({ ...pin, path: "/etc/passwd" }).join(),
    /safe relative/,
  );
});

test("rejects_an_unsafe_upstream_path", () => {
  const files = { "../escape.rs": { ...pin.files["src/acp.rs"] } };
  assert.match(
    validateDisposition({ ...pin, files }).join(),
    /unsafe upstream path/,
  );
});

test("rejects_an_unknown_disposition_or_owner", () => {
  const files = {
    "src/acp.rs": { ...pin.files["src/acp.rs"], disposition: "maybe" },
  };
  assert.match(
    validateDisposition({ ...pin, files }).join(),
    /disposition must be one of/,
  );
  const orphan = {
    "src/acp.rs": { ...pin.files["src/acp.rs"], owner: "crates/unknown" },
  };
  assert.match(
    validateDisposition({ ...pin, files: orphan }).join(),
    /outside the declared vocabulary/,
  );
});

test("rejects_an_owner_disposition_mismatch", () => {
  const droppedWithOwner = {
    "src/relay.rs": {
      ...pin.files["src/relay.rs"],
      owner: "crates/jj-acp-protocol",
    },
  };
  assert.match(
    validateDisposition({ ...pin, files: droppedWithOwner }).join(),
    /must not name an owning module/,
  );
  const portedWithoutOwner = {
    "src/acp.rs": { ...pin.files["src/acp.rs"], owner: "none" },
  };
  assert.match(
    validateDisposition({ ...pin, files: portedWithoutOwner }).join(),
    /only a dropped region may have no owner/,
  );
});

test("digests_are_stable_sha256", () => {
  assert.equal(digestOf(Buffer.from("abc")).length, 64);
  assert.equal(
    digestOf(Buffer.from("abc")),
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
});

const digests = {
  "src/acp.rs": digestOf(Buffer.from("acp")),
  "src/relay.rs": digestOf(Buffer.from("relay")),
};
const sizes = { "src/acp.rs": 3, "src/relay.rs": 5 };

test("build_lock_joins_the_curated_map_with_observed_digests", () => {
  const lock = buildLock(pin, digests, sizes);
  assert.deepEqual(validateLock(lock, pin), []);
  assert.equal(lock.files["src/acp.rs"].bytes, 3);
  assert.equal(lock.commit, pin.commit);
});

test("build_lock_refuses_a_classified_region_without_a_digest", () => {
  assert.match(
    (() => {
      try {
        return buildLock(pin, { "src/acp.rs": digests["src/acp.rs"] }, sizes);
      } catch (err) {
        return err.message;
      }
    })(),
    /no digest observed for classified upstream file src\/relay\.rs/,
  );
});

test("validate_lock_detects_drift_from_the_curated_map", () => {
  const lock = buildLock(pin, digests, sizes);
  assert.match(
    validateLock({ ...lock, commit: "0".repeat(40) }, pin).join(),
    /does not match the pinned disposition map/,
  );
  const redacted = JSON.parse(JSON.stringify(lock));
  redacted.files["src/relay.rs"].disposition = "ported";
  assert.match(
    validateLock(redacted, pin).join(),
    /lock disposition drifted from the curated map/,
  );
  const partial = JSON.parse(JSON.stringify(lock));
  delete partial.files["src/acp.rs"];
  assert.match(validateLock(partial, pin).join(), /missing a digest/);
  const extra = JSON.parse(JSON.stringify(lock));
  extra.files["src/new.rs"] = { ...extra.files["src/acp.rs"] };
  assert.match(validateLock(extra, pin).join(), /unclassified upstream file/);
  const corrupt = JSON.parse(JSON.stringify(lock));
  corrupt.files["src/acp.rs"].sha256 = "nothex";
  assert.match(validateLock(corrupt, pin).join(), /64 lowercase hexadecimal/);
});

test("drift_report_classifies_every_observed_file", () => {
  const lock = buildLock(pin, digests, sizes);
  assert.deepEqual(driftReport(lock, digests), {
    missing: [],
    changed: [],
    unexpected: [],
    unchanged: ["src/acp.rs", "src/relay.rs"],
    drifted: false,
  });
  const changed = { ...digests, "src/acp.rs": "f".repeat(64) };
  assert.deepEqual(driftReport(lock, changed).changed, ["src/acp.rs"]);
  const removed = { "src/acp.rs": digests["src/acp.rs"] };
  assert.deepEqual(driftReport(lock, removed).missing, ["src/relay.rs"]);
  const added = { ...digests, "src/extra.rs": "e".repeat(64) };
  assert.deepEqual(driftReport(lock, added).unexpected, ["src/extra.rs"]);
  assert.equal(driftReport(lock, added).drifted, true);
});

test("scratch_materialization_stays_in_ignored_storage", () => {
  const dir = scratchDir(root, { repository: "block/buzz", commit: "abc123" });
  assert.match(
    dir.replaceAll("\\", "/"),
    /\/\.local\/upstream\/block-buzz@abc123$/,
  );
});

test("the_recorded_lock_matches_the_curated_map", () => {
  const lock = JSON.parse(
    readFileSync(
      new URL("../docs/upstream/lock.json", import.meta.url),
      "utf8",
    ),
  );
  assert.deepEqual(validateLock(lock, curated), []);
  assert.equal(
    Object.keys(lock.files).length,
    Object.keys(curated.files).length,
  );
});
