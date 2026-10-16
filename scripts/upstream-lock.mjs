import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";

export const DISPOSITIONS = [
  "ported",
  "ported-with-changes",
  "dropped",
  "not-ported",
];

export const dispositionPath = "docs/upstream/disposition.json";
export const lockPath = "docs/upstream/lock.json";
export const scratchRoot = ".local/upstream";

const COMMIT = /^[0-9a-f]{40}$/;

function safeRelative(path) {
  return (
    typeof path === "string" &&
    path.length > 0 &&
    !path.startsWith("/") &&
    !path.split(/[\\/]/).includes("..")
  );
}

export function digestOf(content) {
  return createHash("sha256").update(content).digest("hex");
}

/** Validate the curated port map. Returns an array of human-readable errors. */
export function validateDisposition(data) {
  const errors = [];
  if (!data || typeof data !== "object")
    return ["disposition map is not an object"];
  if (!safeRelative(data.repository ?? ""))
    errors.push("repository must be a non-empty owner/name slug");
  else if (!/^[\w.-]+\/[\w.-]+$/u.test(data.repository))
    errors.push(`repository is not an owner/name slug: ${data.repository}`);
  if (!COMMIT.test(data.commit ?? ""))
    errors.push("commit must be a 40-character lowercase hexadecimal SHA");
  if (!data.license) errors.push("license is required");
  if (!safeRelative(data.path ?? ""))
    errors.push("path must be a safe relative upstream crate path");
  const owners = data.owners;
  if (!Array.isArray(owners) || !owners.length)
    errors.push("owners must be a non-empty vocabulary");
  const ownerSet = new Set(owners ?? []);
  const files = data.files;
  if (!files || typeof files !== "object" || Array.isArray(files))
    return [...errors, "files must be an object keyed by upstream path"];
  for (const [path, entry] of Object.entries(files)) {
    if (!safeRelative(path)) {
      errors.push(`unsafe upstream path: ${path}`);
      continue;
    }
    if (!entry || typeof entry !== "object") {
      errors.push(`${path}: entry is not an object`);
      continue;
    }
    if (!DISPOSITIONS.includes(entry.disposition))
      errors.push(
        `${path}: disposition must be one of ${DISPOSITIONS.join(", ")}`,
      );
    if (!ownerSet.has(entry.owner))
      errors.push(
        `${path}: owner is outside the declared vocabulary: ${entry.owner}`,
      );
    if (entry.owner === "none" && entry.disposition !== "dropped")
      errors.push(`${path}: only a dropped region may have no owner`);
    if (entry.owner !== "none" && entry.disposition === "dropped")
      errors.push(`${path}: a dropped region must not name an owning module`);
    if (!entry.note || entry.note.trim().length < 8)
      errors.push(`${path}: note must explain the disposition`);
  }
  return [...new Set(errors)];
}

/** Validate a generated lock record against its curated port map. */
export function validateLock(lock, disposition) {
  const errors = [];
  if (!lock || typeof lock !== "object")
    return ["lock record is not an object"];
  for (const key of ["repository", "path", "commit", "license"])
    if (lock[key] !== disposition[key])
      errors.push(`lock ${key} does not match the pinned disposition map`);
  if (!COMMIT.test(lock.commit ?? ""))
    errors.push("lock commit must be a 40-character hexadecimal SHA");
  const files = lock.files ?? {};
  const expected = Object.keys(disposition.files ?? {}).sort();
  const actual = Object.keys(files).sort();
  for (const path of expected.filter((p) => !actual.includes(p)))
    errors.push(`lock is missing a digest for ${path}`);
  for (const path of actual.filter((p) => !expected.includes(p)))
    errors.push(`lock records an unclassified upstream file: ${path}`);
  for (const [path, entry] of Object.entries(files)) {
    if (!/^[0-9a-f]{64}$/.test(entry?.sha256 ?? ""))
      errors.push(
        `${path}: lock digest must be 64 lowercase hexadecimal characters`,
      );
    if (!Number.isInteger(entry?.bytes) || entry.bytes < 0)
      errors.push(`${path}: lock byte count must be a non-negative integer`);
    if (entry?.disposition !== disposition.files?.[path]?.disposition)
      errors.push(`${path}: lock disposition drifted from the curated map`);
    if (entry?.owner !== disposition.files?.[path]?.owner)
      errors.push(`${path}: lock owner drifted from the curated map`);
  }
  return [...new Set(errors)];
}

/** Compare freshly computed digests against a recorded lock. */
export function driftReport(lock, observed) {
  const recorded = lock?.files ?? {};
  const missing = [];
  const changed = [];
  const unchanged = [];
  for (const [path, entry] of Object.entries(recorded)) {
    if (!(path in observed)) missing.push(path);
    else if (observed[path] !== entry.sha256) changed.push(path);
    else unchanged.push(path);
  }
  const unexpected = Object.keys(observed).filter(
    (path) => !(path in recorded),
  );
  return {
    missing: missing.sort(),
    changed: changed.sort(),
    unexpected: unexpected.sort(),
    unchanged: unchanged.sort(),
    drifted: missing.length + changed.length + unexpected.length > 0,
  };
}

export function scratchDir(root, lock) {
  const slug = String(lock.repository).replaceAll("/", "-");
  return join(root, scratchRoot, `${slug}@${lock.commit}`);
}

export function readDisposition(root) {
  const data = JSON.parse(readFileSync(join(root, dispositionPath), "utf8"));
  const errors = validateDisposition(data);
  if (errors.length) throw new Error(errors.join("\n"));
  return data;
}

export function readLock(root) {
  return JSON.parse(readFileSync(join(root, lockPath), "utf8"));
}

export function buildLock(disposition, digests, sizes) {
  const files = {};
  for (const [path, entry] of Object.entries(disposition.files)) {
    if (!(path in digests))
      throw new Error(
        `no digest observed for classified upstream file ${path}`,
      );
    files[path] = {
      sha256: digests[path],
      bytes: sizes[path],
      disposition: entry.disposition,
      owner: entry.owner,
    };
  }
  return {
    repository: disposition.repository,
    path: disposition.path,
    commit: disposition.commit,
    commitDate: disposition.commitDate,
    license: disposition.license,
    files,
  };
}
