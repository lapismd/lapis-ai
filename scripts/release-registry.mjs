import { writeFileSync } from "node:fs";
import { verifyExisting } from "./registry-check.mjs";
import { execFileSync } from "node:child_process";
import { crates, version } from "./release-version.mjs";
const selectedVersion = version();
const missing = [];
for (const name of crates) {
  if (!(await verifyExisting(name, selectedVersion))) missing.push(name);
}
if (process.argv.includes("--publish") && missing.length) {
  execFileSync(
    "node",
    [
      "scripts/managed-cargo.mjs",
      "publish",
      ...missing.flatMap((name) => ["-p", name]),
      "--locked",
    ],
    { stdio: "inherit" },
  );
} else if (missing.length && !process.argv.includes("--status")) {
  throw Error(`missing registry releases: ${missing.join(", ")}`);
}
if (process.env.GITHUB_OUTPUT)
  writeFileSync(process.env.GITHUB_OUTPUT, `missing=${missing.length}\n`, {
    flag: "a",
  });
console.log(
  `${crates.length - missing.length}/${crates.length} existing releases verified`,
);
