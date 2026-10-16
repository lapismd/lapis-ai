import { execFileSync } from "node:child_process";
import {
  crates,
  cratePath,
  manifest,
  read,
  version,
} from "./release-version.mjs";
for (const name of crates) {
  const root = cratePath(name),
    cargo = manifest(".", `${root}/Cargo.toml`);
  if (
    cargo.package.repository?.workspace !== true ||
    cargo.package.publish?.workspace !== true ||
    !cargo.package.description
  )
    throw Error(`${name}: publication metadata missing`);
  const files = execFileSync(
    "node",
    [
      "scripts/managed-cargo.mjs",
      "package",
      "-p",
      name,
      "--list",
      "--locked",
      ...(process.argv.includes("--allow-dirty") ? ["--allow-dirty"] : []),
    ],
    { encoding: "utf8" },
  ).split("\n");
  for (const required of ["README.md", "LICENSE.md", "NOTICE.md", "Cargo.toml"])
    if (!files.includes(required))
      throw Error(`${name}: package omits ${required}`);
  for (const path of files)
    if (
      /^(?:\.local|target|dist|node_modules)\//.test(path) ||
      /\.(?:pem|key|gguf)$/.test(path)
    )
      throw Error(`${name}: private/generated package content`);
  if (name === "lapis-acp-testkit")
    for (const file of ["fixture", "interactive", "fleet"])
      if (!files.includes(`tests/fixtures/${file}_agent.py`))
        throw Error("testkit fixture missing");
  if (name === "lapis-agent") {
    for (const name of ["model-capabilities.json", "normative-corpus.json"])
      if (!files.includes(`data/${name}`))
        throw Error("agent embedded data missing");
    if (cargo.features.default.length)
      throw Error("agent test support must not be enabled by default");
    for (const binary of cargo.bin)
      if (
        binary.name !== "lapis-agent" &&
        binary["required-features"]?.join(",") !== "test-support"
      )
        throw Error("agent test helper could install by default");
  }
  if (!read(".", `${root}/NOTICE.md`).includes("Apache"))
    throw Error("attribution missing");
}
console.log(
  `Publication metadata and contents verified for ${crates.length} crates at ${version()}`,
);
