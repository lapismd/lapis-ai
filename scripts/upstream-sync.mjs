import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildLock,
  digestOf,
  driftReport,
  lockPath,
  readDisposition,
  readLock,
  scratchDir,
  validateDisposition,
  validateLock,
} from "./upstream-lock.mjs";

const API = "https://api.github.com/repos";
const RAW = "https://raw.githubusercontent.com";
const HEADERS = {
  "User-Agent": "jj-acp-upstream-sync",
  Accept: "application/vnd.github+json",
};
const CONCURRENCY = 6;

async function getJson(url) {
  const response = await fetch(url, { headers: HEADERS });
  if (!response.ok) throw new Error(`${url} failed: HTTP ${response.status}`);
  return response.json();
}

/** Resolve the pinned crate directory to its own tree so listing stays small. */
async function crateTreeSha(disposition) {
  const url = `${API}/${disposition.repository}/contents/${dirname(disposition.path)}?ref=${disposition.commit}`;
  const entries = await getJson(url);
  const name = disposition.path.split("/").pop();
  const entry = entries.find(
    (item) => item.name === name && item.type === "dir",
  );
  if (!entry)
    throw new Error(`${disposition.path} is absent at the pinned commit`);
  return entry.sha;
}

async function listUpstreamFiles(disposition) {
  const sha = await crateTreeSha(disposition);
  const tree = await getJson(
    `${API}/${disposition.repository}/git/trees/${sha}?recursive=1`,
  );
  if (tree.truncated)
    throw new Error(
      "upstream tree listing was truncated; the pin is too broad",
    );
  return tree.tree
    .filter((entry) => entry.type === "blob")
    .map((entry) => entry.path)
    .sort();
}

async function download(disposition, path) {
  const url = `${RAW}/${disposition.repository}/${disposition.commit}/${disposition.path}/${path}`;
  const response = await fetch(url, {
    headers: { "User-Agent": HEADERS["User-Agent"] },
  });
  if (!response.ok) throw new Error(`${path} failed: HTTP ${response.status}`);
  return Buffer.from(await response.arrayBuffer());
}

async function materialize(root, disposition, paths) {
  const lock = {
    repository: disposition.repository,
    commit: disposition.commit,
  };
  const target = scratchDir(root, lock);
  const contents = new Map();
  for (let index = 0; index < paths.length; index += CONCURRENCY) {
    const batch = paths.slice(index, index + CONCURRENCY);
    const results = await Promise.all(
      batch.map(async (path) => [path, await download(disposition, path)]),
    );
    for (const [path, buffer] of results) contents.set(path, buffer);
  }
  for (const [path, buffer] of contents) {
    const absolute = join(target, path);
    mkdirSync(dirname(absolute), { recursive: true });
    writeFileSync(absolute, buffer);
  }
  return { target, contents };
}

function observe(contents) {
  const digests = {};
  const sizes = {};
  for (const [path, buffer] of contents) {
    digests[path] = digestOf(buffer);
    sizes[path] = buffer.byteLength;
  }
  return { digests, sizes };
}

function classify(disposition, paths) {
  const classified = Object.keys(disposition.files);
  const unclassified = paths.filter((path) => !classified.includes(path));
  const absent = classified.filter((path) => !paths.includes(path));
  if (unclassified.length || absent.length) {
    const problems = [
      ...unclassified.map((p) => `unclassified upstream file: ${p}`),
      ...absent.map((p) => `classified file is absent upstream: ${p}`),
    ];
    throw new Error(
      `${problems.join("\n")}\nUpdate docs/upstream/disposition.json before locking.`,
    );
  }
}

async function write(root) {
  const disposition = readDisposition(root);
  const paths = await listUpstreamFiles(disposition);
  classify(disposition, paths);
  const { target, contents } = await materialize(root, disposition, paths);
  const { digests, sizes } = observe(contents);
  const lock = buildLock(disposition, digests, sizes);
  const errors = validateLock(lock, disposition);
  if (errors.length) throw new Error(errors.join("\n"));
  const absolute = join(root, lockPath);
  mkdirSync(dirname(absolute), { recursive: true });
  writeFileSync(absolute, `${JSON.stringify(lock, null, 2)}\n`);
  console.log(
    `upstream lock written for ${disposition.repository}@${disposition.commit.slice(0, 12)} ` +
      `(${paths.length} files materialized in ${target})`,
  );
}

async function verify(root) {
  const disposition = readDisposition(root);
  if (!existsSync(join(root, lockPath)))
    throw new Error(
      `${lockPath} is absent; run --write to record the pin first`,
    );
  const lock = readLock(root);
  const lockErrors = validateLock(lock, disposition);
  if (lockErrors.length) throw new Error(lockErrors.join("\n"));
  const paths = await listUpstreamFiles(disposition);
  classify(disposition, paths);
  const { contents } = await materialize(root, disposition, paths);
  const { digests } = observe(contents);
  const report = driftReport(lock, digests);
  for (const path of report.changed) console.log(`changed:   ${path}`);
  for (const path of report.missing) console.log(`missing:   ${path}`);
  for (const path of report.unexpected) console.log(`unexpected: ${path}`);
  console.log(
    `upstream drift check: ${report.unchanged.length} unchanged, ` +
      `${report.changed.length} changed, ${report.missing.length} missing, ` +
      `${report.unexpected.length} unexpected`,
  );
  if (report.drifted)
    throw new Error(
      "pinned upstream source drifted from the recorded lock; re-review the port map",
    );
}

/** Offline gate: prove the pin and port map are internally consistent. */
function selfTest(root) {
  const source = readFileSync(
    join(root, "docs/upstream/disposition.json"),
    "utf8",
  );
  const errors = validateDisposition(JSON.parse(source));
  if (errors.length) throw new Error(errors.join("\n"));
  const disposition = JSON.parse(source);
  const counts = {};
  for (const entry of Object.values(disposition.files))
    counts[entry.disposition] = (counts[entry.disposition] ?? 0) + 1;
  if (!existsSync(join(root, lockPath))) {
    console.log(
      `upstream pin self-test ok (${disposition.repository}@${disposition.commit.slice(0, 12)}, ` +
        `${Object.keys(disposition.files).length} classified regions; no lock recorded yet)`,
    );
    return;
  }
  const lock = readLock(root);
  const lockErrors = validateLock(lock, disposition);
  if (lockErrors.length) throw new Error(lockErrors.join("\n"));
  console.log(
    `upstream pin self-test ok (${Object.keys(lock.files).length} digests verified against the curated map: ` +
      `${Object.entries(counts)
        .map(([kind, count]) => `${count} ${kind}`)
        .sort()
        .join(", ")})`,
  );
}

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const mode = process.argv[2];
try {
  if (mode === "--self-test") selfTest(root);
  else if (mode === "--write") await write(root);
  else if (mode === "--fetch") await verify(root);
  else
    throw new Error("usage: upstream-sync.mjs --self-test | --write | --fetch");
} catch (err) {
  console.error(err.message);
  process.exitCode = 1;
}
