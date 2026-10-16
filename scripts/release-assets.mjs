import { readdirSync, readFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
const tag = process.env.RELEASE_TAG;
const assets = JSON.parse(
  execFileSync("gh", ["release", "view", tag, "--json", "assets"], {
    encoding: "utf8",
  }),
).assets;
const digest = (file) =>
  createHash("sha256").update(readFileSync(file)).digest("hex");
const pending = [];
const temporary = mkdtempSync(join(tmpdir(), "lapis-ai-assets-"));
try {
  for (const name of readdirSync("dist").filter(
    (name) => name.endsWith(".tar.gz") || name === "SHA256SUMS",
  )) {
    const local = join("dist", name);
    if (!assets.some((asset) => asset.name === name)) {
      pending.push(local);
      continue;
    }
    execFileSync("gh", [
      "release",
      "download",
      tag,
      "--pattern",
      name,
      "--dir",
      temporary,
    ]);
    if (digest(local) !== digest(join(temporary, name)))
      throw Error(
        `existing release asset differs: ${name}; refuse replacement`,
      );
  }
  if (pending.length)
    execFileSync("gh", ["release", "upload", tag, ...pending], {
      stdio: "inherit",
    });
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
