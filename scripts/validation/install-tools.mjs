import { createHash } from "node:crypto";
import { mkdir, writeFile, rm } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

// Reuse the checksum-qualified upstream tools selected by Notes/Plugins.
const pins = {
  x64: [
    [
      "actionlint",
      "1.7.12",
      "amd64",
      "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8",
    ],
    [
      "jj",
      "0.45.1",
      "x86_64",
      "f35438350b5d61963aac5dd74ede510b31d6b9690769d1a6268cf058cc825f72",
    ],
    [
      "mdbook",
      "0.5.4",
      "x86_64",
      "5222beabd3e37dc5be0d18ff99b79058469354db5c220153a1b92db5ba12be89",
    ],
  ],
  arm64: [
    [
      "actionlint",
      "1.7.12",
      "arm64",
      "325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6",
    ],
    [
      "jj",
      "0.45.1",
      "aarch64",
      "7349a43dd5a20dbc998b10114daa0ee63d2ab863fb822c7eb6b0ebca5903cc69",
    ],
    [
      "mdbook",
      "0.5.4",
      "aarch64",
      "753e5c5c363ee8a56972344dcf91466f005a51db84a7aeffe427ae3ef83d6d44",
    ],
  ],
};
export async function install(directory = "/opt/tools") {
  if (process.platform !== "linux" || !pins[process.arch])
    throw Error("Unsupported validation tool platform");
  await mkdir(directory, { recursive: true });
  for (const [name, version, arch, digest] of pins[process.arch]) {
    const asset =
      name === "actionlint"
        ? `actionlint_${version}_linux_${arch}.tar.gz`
        : `${name}-v${version}-${arch}-unknown-linux-musl.tar.gz`;
    const owner =
      name === "jj"
        ? "jj-vcs/jj"
        : name === "actionlint"
          ? "rhysd/actionlint"
          : "rust-lang/mdBook";
    const response = await fetch(
      `https://github.com/${owner}/releases/download/v${version}/${asset}`,
    );
    if (!response.ok) throw Error(`Tool download failed: ${response.status}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    if (createHash("sha256").update(bytes).digest("hex") !== digest)
      throw Error(`Tool archive checksum mismatch: ${name}`);
    const archive = join(directory, asset);
    await writeFile(archive, bytes);
    const members = execFileSync("tar", ["-tzf", archive], { encoding: "utf8" })
      .split("\n")
      .filter((member) => member.replace(/^\.\//, "") === name);
    if (members.length !== 1)
      throw Error(`Tool archive must contain exactly one root ${name}`);
    execFileSync("tar", ["-xzf", archive, "-C", directory, members[0]]);
    const actual = execFileSync(join(directory, name), ["--version"], {
      encoding: "utf8",
    });
    if (
      !(name === "actionlint"
        ? actual.startsWith(version)
        : actual.startsWith(name + (name === "mdbook" ? " v" : " ") + version))
    )
      throw Error(`Tool version mismatch: ${name}`);
    await rm(archive);
  }
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  await install();
}
