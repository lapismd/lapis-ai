import { sourceBaseline } from "./initial-intake.mjs";
import { sourceIntakeReader } from "./source-intake-reader.mjs";
import {
  managedMaintenanceNote,
  readMaintenanceNote,
  verifyMaintenanceNotes,
  verifyMaintenanceScope,
} from "./maintenance-note.mjs";
import { readFileSync, realpathSync } from "node:fs";
import { createHash } from "node:crypto";
import { isDeepStrictEqual as equal } from "node:util";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { parse } from "yaml";
import { changesFromVcs } from "@lapismd/spec-validator";

const core = "@lapismd/spec-validator";
const policyPath = ".github/dependency-automation/canonical-intake.json";
const chapters = [
  "spec/src/current/governance.md",
  "spec/src/current/contracts/release.md",
];
const expectedPolicy = {
  schemaVersion: 1,
  kind: "single-development-dependency",
  repository: "lapismd/lapis-ai",
  dependency: core,
  slot: "validator-development",
  chapters,
};
const fail = (reason) => {
  throw Error(`Lapis AI canonical intake: ${reason}`);
};
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
export function validatorFacts(files, policyBytes) {
  if (!equal(JSON.parse(policyBytes), expectedPolicy))
    fail("source policy changed or unsupported");
  const manifest = JSON.parse(files["package.json"]),
    workspace = parse(files["pnpm-workspace.yaml"]),
    lock = parse(files["pnpm-lock.yaml"]);
  const version = manifest.devDependencies?.[core],
    peer = manifest.peerDependencies?.[core] ?? null;
  const integrity =
    lock.packages?.[`${core}@${version}`]?.resolution?.integrity;
  const importers = Object.fromEntries(
    Object.entries(lock.importers ?? {})
      .filter(([, p]) => p.devDependencies?.[core])
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([path, p]) => [path, p.devDependencies[core]]),
  );
  if (
    !/^0\.2\.\d+$/.test(version ?? "") ||
    peer !== null ||
    workspace.overrides?.[core] !== version ||
    lock.overrides?.[core] !== version ||
    !/^sha512-[A-Za-z0-9+/]{86}==$/.test(integrity ?? "") ||
    !importers["."] ||
    Object.values(importers).some(
      (v) => v.specifier !== version || v.version?.split("(")[0] !== version,
    )
  )
    fail(
      "Validator catalog, peer support, override or registry resolution mismatch",
    );
  return {
    schemaVersion: 1,
    kind: "development-dependency-intake",
    dependency: core,
    version,
    peer,
    integrity,
    importers,
    policySha256: hash(policyBytes),
  };
}
function slot(text) {
  const start = "<!-- dependency-intake:validator-development:start -->",
    end = "<!-- dependency-intake:validator-development:end -->";
  if (
    typeof text !== "string" ||
    text.split(start).length !== 2 ||
    text.split(end).length !== 2 ||
    text.indexOf(end) < text.indexOf(start)
  )
    fail("missing or ambiguous source slot");
  const body = text.slice(
    text.indexOf(start) + start.length,
    text.indexOf(end),
  );
  const match = /^\s*```json\n([\s\S]*?)\n```\s*$/.exec(body);
  if (!match) fail("manual or malformed source slot");
  return {
    facts: JSON.parse(match[1]),
    outside:
      text.slice(0, text.indexOf(start)) +
      text.slice(text.indexOf(end) + end.length),
  };
}
// Lapis AI owns its catalog audit; it compares package identities, metadata and
// direct dependency edges independently from the coordinator's string renderer.
function catalog(lock, version) {
  const coreIdentity = new RegExp(
    `(^|\\()${core}@${version.replaceAll(".", "\\.")}(?=$|[()])`,
    "g",
  );
  const identity = (s) =>
    s
      .replace(coreIdentity, (_match, prefix) => `${prefix}${core}@<Validator>`)
      .replace(/\([a-f0-9]{32}\)/g, "(<peer-context>)");
  const reference = (name, value) =>
    name === core
      ? value.split("(")[0] === version
        ? `<Validator>${identity(value.slice(version.length))}`
        : fail("unexpected Validator lock reference")
      : identity(value);
  const entries = (rows, make) =>
    Object.fromEntries(
      Object.entries(rows ?? {}).map(([name, row]) => [
        identity(name),
        make(row),
      ]),
    );
  const packages = entries(lock.packages, (row) => ({
    ...row,
    ...(row.peerDependencies
      ? {
          peerDependencies: Object.fromEntries(
            Object.entries(row.peerDependencies).map(([n, v]) => [
              n,
              n === core && v === version ? "<Validator>" : v,
            ]),
          ),
        }
      : {}),
  }));
  const selected = packages[`${core}@<Validator>`];
  if (!selected) fail("missing Validator package");
  selected.resolution = { ...selected.resolution, integrity: "<Validator>" };
  const snapshots = {};
  for (const [name, row] of Object.entries(lock.snapshots ?? {})) {
    const key = identity(name),
      normalized = { ...row };
    for (const section of ["dependencies", "optionalDependencies"])
      if (row[section])
        normalized[section] = Object.fromEntries(
          Object.entries(row[section]).map(([n, v]) => [n, reference(n, v)]),
        );
    if (Object.hasOwn(snapshots, key)) fail("ambiguous opaque peer contexts");
    snapshots[key] = normalized;
  }
  const importers = Object.fromEntries(
    Object.entries(lock.importers ?? {}).map(([path, row]) => [
      path,
      Object.fromEntries(
        Object.entries(row).map(([section, items]) => [
          section,
          Object.fromEntries(
            Object.entries(items).map(([name, item]) => [
              name,
              {
                ...item,
                ...(name === core ? { specifier: "<Validator>" } : {}),
                version: reference(name, item.version),
              },
            ]),
          ),
        ]),
      ),
    ]),
  );
  return {
    ...lock,
    overrides: { ...lock.overrides, [core]: "<Validator>" },
    packages,
    snapshots,
    importers,
  };
}
export function verifyValidatorCanonicalIntake({
  files,
  policyBytes,
  documents,
  before,
  basePolicyBytes,
  baseDocuments,
  changedFiles = [],
}) {
  const facts = validatorFacts(files, policyBytes);
  verifyMaintenanceNotes(changedFiles, files);
  for (const p of chapters)
    if (!equal(slot(documents[p]).facts, facts))
      fail("managed facts differ from actual candidate");
  if (!before) return facts;
  const previous = JSON.parse(before["package.json"]),
    candidate = JSON.parse(files["package.json"]);
  const old = parse(before["pnpm-lock.yaml"]).importers?.[
    "."
  ]?.devDependencies?.[core]?.version?.split("(")[0];
  if (old === facts.version) {
    const originalIntegrity = parse(before["pnpm-lock.yaml"]).packages?.[
      `${core}@${old}`
    ]?.resolution?.integrity;
    if (originalIntegrity !== facts.integrity)
      fail("immutable Validator version integrity changed");
    return facts;
  }
  if (
    basePolicyBytes !== policyBytes ||
    !/^0\.2\.\d+$/.test(old ?? "") ||
    Number(facts.version.split(".")[2]) <= Number(old.split(".")[2])
  )
    fail("missing, changed or stale source opt-in");
  verifyMaintenanceScope(changedFiles, files, [
    "package.json",
    "pnpm-workspace.yaml",
    "pnpm-lock.yaml",
    ...chapters,
  ]);
  const strip = (value, section) => {
    const copy = structuredClone(value);
    delete copy[section][core];
    return copy;
  };
  if (
    before["packages/ai/package.json"] !== files["packages/ai/package.json"] ||
    !equal(
      strip(previous, "devDependencies"),
      strip(candidate, "devDependencies"),
    ) ||
    !equal(
      strip(parse(before["pnpm-workspace.yaml"]), "overrides"),
      strip(parse(files["pnpm-workspace.yaml"]), "overrides"),
    ) ||
    !equal(
      catalog(parse(before["pnpm-lock.yaml"]), old),
      catalog(parse(files["pnpm-lock.yaml"]), facts.version),
    )
  )
    fail("unexpected public contract or registry catalog change");
  const oldFacts = validatorFacts(before, basePolicyBytes);
  for (const p of chapters) {
    const base = slot(baseDocuments[p]),
      current = slot(documents[p]);
    if (!equal(base.facts, oldFacts) || base.outside !== current.outside)
      fail("manual source chapter edits or stale baseline");
  }
  return facts;
}
export function checkValidatorCanonicalIntake(root = process.cwd()) {
  const read = (path) => {
    const target = resolve(root, path);
    if (realpathSync(target) !== target) fail("non-regular source path");
    return readFileSync(target, "utf8");
  };
  const inputs = [
    "package.json",
    "pnpm-workspace.yaml",
    "pnpm-lock.yaml",
    "packages/ai/package.json",
  ];
  const files = Object.fromEntries(inputs.map((p) => [p, read(p)])),
    documents = Object.fromEntries(chapters.map((p) => [p, read(p)]));
  const base = sourceIntakeReader(root);
  const context = sourceBaseline({
    readPrevious: base,
    sourceSha: base.sourceSha,
    policyBytes: read(policyPath),
    files,
    metadataBytes: read(".github/dependency-automation/bootstrap.json"),
  });
  const changedFiles = changesFromVcs({}, root).map((c) => c.path);
  for (const path of changedFiles.filter(managedMaintenanceNote))
    files[path] = readMaintenanceNote(resolve(root, path));
  return verifyValidatorCanonicalIntake({
    files,
    policyBytes: read(policyPath),
    documents,
    ...context,
    changedFiles,
  });
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  checkValidatorCanonicalIntake();
  console.log(
    "Lapis AI Validator canonical intake matches actual source facts.",
  );
}
