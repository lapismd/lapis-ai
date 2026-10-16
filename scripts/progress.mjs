import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
export function ledgerErrors(source, readEvidence) {
  const errors = [];
  const rows = source
    .split("\n")
    .filter((line) => /^\| M[0-9]/.test(line))
    .map((line) => line.split("|").map((s) => s.trim()));
  const statuses = new Map(rows.map((row) => [row[1], row[2]]));
  if (rows.length !== 6 || statuses.size !== 6)
    errors.push("Expected exactly six governed milestones");
  for (const row of rows) {
    const [task, status, dependencies, , evidence] = row.slice(1);
    if (!["Planned", "In progress", "Done"].includes(status))
      errors.push(`${task}: invalid status`);
    if (status !== "Done") continue;
    for (const dependency of dependencies
      .split(/[, ]+/)
      .filter((value) => value !== "none")) {
      if (statuses.get(dependency) !== "Done")
        errors.push(`${task}: incomplete dependency ${dependency}`);
    }
    const links = [...evidence.matchAll(/\]\(([^)]+\.md)\)/g)];
    if (!links.length) errors.push(`${task}: Done requires linked evidence`);
    for (const [, path] of links) {
      const text = readEvidence(path);
      if (!text) {
        errors.push(`${task}: missing evidence`);
        continue;
      }
      for (const field of [
        "Revision:",
        "Milestone review:",
        "## Commands",
        "## Results",
        "## Failures",
        "## Gaps",
        "Next action:",
        "Gate result: passed",
      ]) {
        if (!text.includes(field))
          errors.push(`${task}: missing evidence field ${field}`);
      }
      if (!text.includes(task))
        errors.push(`${task}: evidence omits milestone`);
    }
  }
  return errors;
}
export function check(root = process.cwd()) {
  const manifest = JSON.parse(
    readFileSync(resolve(root, "docs/imports/source-manifest.json")),
  );
  const destinations = new Map();
  const identities = new Set();
  for (const file of manifest.files) {
    if (
      !/^[a-f0-9]{64}$/.test(file.sha256) ||
      !/^[a-f0-9]{40}$/.test(file.revision)
    )
      throw new Error("Invalid source pin");
    if (
      identities.has(`${file.source}:${file.revision}:${file.path}`) ||
      !existsSync(resolve(root, file.destination))
    )
      throw new Error("Duplicate or absent imported destination");
    identities.add(`${file.source}:${file.revision}:${file.path}`);
    destinations.set(file.destination, file);
  }
  const baseline = JSON.parse(
    readFileSync(resolve(root, "scripts/architecture-baseline.json")),
  );
  for (const [path, size] of Object.entries(baseline)) {
    const original = destinations.get(path)?.counted_lines;
    if (
      !Number.isInteger(original) ||
      !Number.isInteger(size) ||
      size > original
    )
      throw new Error(`Unproven or growing baseline: ${path}`);
  }
  const ledger = resolve(root, "spec/src/current/progress/milestones.md");
  const errors = ledgerErrors(readFileSync(ledger, "utf8"), (path) => {
    const full = resolve(dirname(ledger), path);
    if (!full.startsWith(resolve(root, "spec/src/history") + "/"))
      return undefined;
    return existsSync(full) ? readFileSync(full, "utf8") : undefined;
  });
  if (errors.length) throw new Error(errors.join("\n"));
  console.log(
    `Progress and import pins pass (${manifest.files.length} source records)`,
  );
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  check();
