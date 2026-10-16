import { readFileSync, readdirSync } from "node:fs";
import { resolve, extname } from "node:path";
import { fileURLToPath } from "node:url";
export function count(source) {
  let block = false;
  return source.split("\n").filter((line) => {
    const value = line.trim();
    if (block) {
      if (value.includes("*/")) block = false;
      return false;
    }
    if (!value || value.startsWith("//") || value.startsWith("# "))
      return false;
    if (value.startsWith("/*")) {
      block = !value.includes("*/");
      return false;
    }
    return true;
  }).length;
}
export function files(root = ".") {
  return readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    if (
      [
        ".git",
        ".jj",
        ".local",
        "target",
        "node_modules",
        "dist",
        "spec",
      ].includes(entry.name)
    )
      return [];
    const path = root === "." ? entry.name : `${root}/${entry.name}`;
    return entry.isDirectory() ? files(path) : [path];
  });
}
export function errors() {
  const baseline = JSON.parse(
    readFileSync("scripts/architecture-baseline.json", "utf8"),
  );
  const paths = files().filter((path) =>
    [".rs", ".mjs", ".ts", ".py"].includes(extname(path)),
  );
  const errors = Object.keys(baseline)
    .filter((path) => !paths.includes(path))
    .map((path) => `${path}: stale baseline`);
  for (const path of paths) {
    const size = count(readFileSync(path, "utf8"));
    const test = /(?:tests?\/|tests?\.rs$|\.test\.)/.test(path);
    const limit = baseline[path] ?? (test ? 500 : 300);
    if (size > limit)
      errors.push(`${path}: ${size} exceeds ${limit} counted lines`);
  }
  return errors;
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const failures = errors();
  if (failures.length) throw new Error(failures.join("\n"));
  console.log("Architecture limits pass");
}
