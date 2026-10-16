import {
  cp,
  mkdir,
  readFile,
  readdir,
  realpath,
  rename,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { createHash } from "node:crypto";
import { join, resolve, relative } from "node:path";
import { execFileSync } from "node:child_process";
import TOML from "@iarna/toml";
const args = process.argv.slice(2).filter((arg) => arg !== "--");
const options = {};
while (args.length) {
  const name = args.shift();
  if (!name.startsWith("--") || !args.length)
    throw new Error(
      "Expected --profile debug|release --destination PATH [--native-runtime DIR]",
    );
  if (
    ![
      "--profile",
      "--destination",
      "--native-runtime",
      "--target-dir",
    ].includes(name)
  )
    throw new Error("Unknown staging option");
  if (options[name.slice(2)] !== undefined)
    throw new Error("Duplicate staging option");
  options[name.slice(2)] = args.shift();
}
const profile = options.profile ?? "release";
if (!["debug", "release"].includes(profile)) throw new Error("Invalid profile");
const target = execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(
  /^host: (.+)$/m,
)?.[1];
const destination = resolve(options.destination ?? `dist/${target}`);
if (
  destination === process.cwd() ||
  !destination.startsWith(process.cwd() + "/dist/")
)
  throw new Error(
    "Destination must be inside this repository's dist directory",
  );
const temporary = destination + `.pending-${process.pid}`;
const targetRoot = resolve(
  options["target-dir"] ?? process.env.CARGO_TARGET_DIR ?? "target",
);
if (!targetRoot.startsWith(process.cwd() + "/"))
  throw new Error("Build outputs must belong to this repository");
const binaries = [
  "lapis-acp",
  "lapis-agent",
  "lapis-agent-tools",
  "lapis-compute",
];
for (const name of binaries)
  if (!(await stat(join(targetRoot, profile, name))).isFile())
    throw new Error(`Build ${name} before staging`);
await mkdir(temporary, { recursive: true, mode: 0o700 });
const manifest = {
  version: 1,
  package_version: JSON.parse(await readFile("package.json", "utf8")).version,
  target,
  binary: [],
  native_file: [],
  upstream: {
    acp_revision: "47946b870b18652964d94c0af110ea177c66a80c",
    store_revision: "01ea0741ff608bbe14daa8d929448e3ec0790afd",
    mesh_revision: "9f192c9d821991ba43d79bb15fd2f2f12e597571",
    buzz_agent_revision: "ef2aa1ae38fadcc0bc22b8bf6ed96b35933146be",
  },
};
const hash = async (path) =>
  createHash("sha256")
    .update(await readFile(path))
    .digest("hex");
try {
  for (const name of binaries) {
    await cp(join(targetRoot, profile, name), join(temporary, name));
    manifest.binary.push({
      name,
      path: name,
      sha256: await hash(join(temporary, name)),
    });
  }
  await cp("LICENSE.md", join(temporary, "LICENSE.md"));
  await cp("docs", join(temporary, "notices"), {
    recursive: true,
    filter: (path) =>
      !path.includes("/imports/") && !path.includes("/upstream/"),
  });
  await cp("crates/lapis-agent/LICENSE", join(temporary, "LICENSE.agent"));
  await cp("crates/lapis-acp-cli/NOTICE.md", join(temporary, "NOTICE.acp.md"));
  if (options["native-runtime"]) {
    const source = await realpath(options["native-runtime"]);
    const native = JSON.parse(
      await readFile(join(source, "manifest.json"), "utf8"),
    );
    if (
      native.runtime?.skippy_abi !== "0.1.44" ||
      native.runtime?.mesh_version !== "0.76.0-rc9" ||
      native.runtime?.platform?.target !== target
    )
      throw new Error("Native runtime target/version/ABI mismatch");
    await cp(source, join(temporary, "native-runtime"), {
      recursive: true,
      dereference: true,
    });
    async function inventory(directory) {
      for (const entry of await readdir(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) await inventory(path);
        else if (entry.isFile()) {
          if (
            /\.(?:gguf|safetensors|key|pem)$|^\.env(?:\.|$)/i.test(entry.name)
          )
            throw new Error(
              "Model weights or secret files cannot enter a runtime bundle",
            );
          manifest.native_file.push({
            path: relative(temporary, path),
            sha256: await hash(path),
          });
          if (
            /\.(json|toml)$/.test(path) &&
            (await stat(path)).size < 1024 * 1024
          ) {
            try {
              const raw = await readFile(path, "utf8");
              const value = path.endsWith("toml")
                ? TOML.parse(raw)
                : JSON.parse(raw);
              if (value.runtime?.skippy_abi)
                manifest.native_abi = String(value.runtime.skippy_abi);
            } catch {}
          }
        }
      }
    }
    await inventory(join(temporary, "native-runtime"));
    if (!manifest.native_abi)
      throw new Error(
        "Native bundle must retain a manifest containing runtime.skippy_abi",
      );
  }
  await writeFile(
    join(temporary, "runtime-manifest.toml"),
    TOML.stringify(manifest),
  );
  // Do not silently replace a previously staged bundle.
  try {
    await stat(destination);
    throw new Error("Destination exists; choose a new destination");
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  await rename(temporary, destination);
  console.log(`Staged ${binaries.length} helpers at ${destination}`);
} catch (error) {
  await rm(temporary, { recursive: true, force: true });
  throw error;
}
