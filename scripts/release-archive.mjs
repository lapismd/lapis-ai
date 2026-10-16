import { mkdirSync, readFileSync, writeFileSync, copyFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { resolve } from "node:path";
import { version } from "./release-version.mjs";
const target = process.argv[2];
if (!["aarch64-apple-darwin", "aarch64-unknown-linux-gnu"].includes(target))
  throw Error("unsupported native release target");
const staged = "dist/qualified-release";
const manifest = readFileSync(`${staged}/runtime-manifest.toml`, "utf8");
if (!manifest.includes(target)) throw Error("bundle target differs");
copyFileSync("README.md", `${staged}/README.md`);
const name = `lapis-ai-${version()}-${target}`;
mkdirSync("dist", { recursive: true });
const epoch = execFileSync("git", ["show", "-s", "--format=%ct", "HEAD"], {
  encoding: "utf8",
}).trim();
const archive = `${name}.tar.gz`;
execFileSync("python3", [
  "-c",
  `
import gzip, tarfile, sys
archive, stage, name, epoch = sys.argv[1:]
def normalize(info):
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    info.mtime = int(epoch)
    info.mode = 0o755 if info.isdir() or info.mode & 0o111 else 0o644
    return info
with open(archive, "wb") as output:
    with gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=int(epoch)) as zipped:
        with tarfile.open(fileobj=zipped, mode="w", format=tarfile.PAX_FORMAT) as bundle:
            bundle.add(stage, arcname=name, filter=normalize)
`,
  resolve("dist", archive),
  resolve(staged),
  name,
  epoch,
]);
const digest = createHash("sha256")
  .update(readFileSync(`dist/${archive}`))
  .digest("hex");
writeFileSync(`dist/${archive}.sha256`, `${digest}  ${archive}\n`);
console.log(archive);
