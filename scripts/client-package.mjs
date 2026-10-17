import { execFileSync } from "node:child_process";
import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve("packages/ai");
const manifest = JSON.parse(
  readFileSync(resolve(root, "package.json"), "utf8"),
);
const [inventory] = JSON.parse(
  execFileSync("npm", ["pack", "--dry-run", "--json", "--ignore-scripts"], {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }),
);
const files = new Set(inventory.files.map((entry) => entry.path));
if (manifest.private !== true || inventory.name !== "@lapismd/ai")
  throw Error("Renderer client must remain private");
for (const entry of Object.values(manifest.exports)) {
  for (const path of Object.values(entry)) {
    if (
      !files.has(path.replace(/^\.\//, "")) ||
      !existsSync(resolve(root, path))
    )
      throw Error(`Missing public output ${path}`);
  }
}
if ([...files].some((path) => /^(?:src|tests|node_modules)\//.test(path)))
  throw Error("Private sources leaked into client archive");
for (const path of ["LICENSE.md", "NOTICE.md", "README.md"]) {
  if (!files.has(path)) throw Error(`Missing package document ${path}`);
}
console.log(
  `Private renderer public outputs verified (${files.size} package files, no publication)`,
);
