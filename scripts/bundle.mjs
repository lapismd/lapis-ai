import { createReadStream } from "node:fs";
import { lstat, readFile, realpath } from "node:fs/promises";
import { createHash } from "node:crypto";
import { join } from "node:path";
import TOML from "@iarna/toml";

export const helperNames = [
  "lapis-acp",
  "lapis-agent",
  "lapis-agent-tools",
  "lapis-compute",
];
export function nativeTarget(platform = process.platform, arch = process.arch) {
  const architecture = { arm64: "aarch64", x64: "x86_64" }[arch];
  const system = {
    darwin: "apple-darwin",
    linux: "unknown-linux-gnu",
    win32: "pc-windows-msvc",
  }[platform];
  if (!architecture || !system) throw Error("Unsupported Lapis AI target");
  return `${architecture}-${system}`;
}
async function regular(root, relative) {
  if (
    typeof relative !== "string" ||
    !relative ||
    relative
      .split("/")
      .some(
        (part) => !part || part === "." || part === ".." || part.includes("\\"),
      )
  )
    throw Error("Invalid bundle path");
  let path = root;
  const parts = relative.split("/");
  for (let index = 0; index < parts.length; index++) {
    path = join(path, parts[index]);
    const info = await lstat(path);
    if (
      info.isSymbolicLink() ||
      (index === parts.length - 1 ? !info.isFile() : !info.isDirectory())
    )
      throw Error("Bundle requires regular files");
  }
  return path;
}
/** Development packaging counterpart of the native ResourceBundle verifier. */
export async function verifyBundle(directory, options = {}) {
  const root = await realpath(directory);
  const file = await regular(root, "runtime-manifest.toml");
  if ((await lstat(file)).size > 1024 * 1024)
    throw Error("Bundle manifest limit");
  const manifest = TOML.parse(await readFile(file, "utf8"));
  const version =
    options.version ??
    JSON.parse(
      await readFile(new URL("../package.json", import.meta.url), "utf8"),
    ).version;
  if (
    manifest.version !== 1 ||
    manifest.package_version !== version ||
    manifest.target !== (options.target ?? nativeTarget())
  )
    throw Error("Incompatible Lapis AI bundle");
  const binaries = manifest.binary;
  const native = manifest.native_file ?? [];
  if (
    !Array.isArray(binaries) ||
    binaries.length !== helperNames.length ||
    [...new Set(binaries.map((entry) => entry.name))].sort().join() !==
      [...helperNames].sort().join()
  )
    throw Error("Incomplete Lapis AI bundle");
  if (
    !Array.isArray(native) ||
    native.length > 512 ||
    (native.length &&
      (manifest.native_abi !== "0.1.44" ||
        !native.some((entry) => entry.path === "native-runtime/manifest.json")))
  )
    throw Error("Incompatible native runtime");
  const paths = new Set();
  for (const entry of [...binaries, ...native]) {
    if (paths.has(entry.path) || !/^[a-f0-9]{64}$/.test(entry.sha256))
      throw Error("Invalid bundle inventory");
    paths.add(entry.path);
    const path = await regular(root, entry.path);
    const hash = createHash("sha256");
    for await (const bytes of createReadStream(path)) hash.update(bytes);
    if (hash.digest("hex") !== entry.sha256)
      throw Error("Bundle checksum mismatch");
  }
  return {
    root,
    manifest,
    helpers: Object.fromEntries(
      binaries.map((entry) => [entry.name, join(root, entry.path)]),
    ),
    nativeRuntime: native.length ? join(root, "native-runtime") : undefined,
  };
}
