import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { exportPack } from "./docker.mjs";
import { hash, sourcePathViolation } from "./model.mjs";

const jj = (root, args) =>
  execFileSync("jj", ["--no-pager", "--color=never", ...args], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  }).trim();
export async function snapshot(root, directory) {
  const revision = jj(root, [
    "log",
    "-r",
    "@",
    "--no-graph",
    "-T",
    "commit_id",
  ]);
  const parents = jj(root, [
    "log",
    "-r",
    `${revision}-`,
    "--no-graph",
    "-T",
    'commit_id ++ "\\n"',
  ])
    .split("\n")
    .filter(Boolean);
  const files = jj(root, ["file", "list", "-r", revision]).split("\n");
  const gitDirectory = jj(root, ["git", "root"]);
  const historical = execFileSync(
    "git",
    ["--git-dir", gitDirectory, "rev-list", "--objects", revision],
    {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    },
  )
    .split("\n")
    .filter((line) => line.includes(" "))
    .map((line) => line.slice(line.indexOf(" ") + 1));
  const rejected = [...files, ...historical].find(sourcePathViolation);
  if (rejected)
    throw Error(
      `Private/local/model state must not enter source snapshot: ${rejected}`,
    );
  await exportPack(
    root,
    gitDirectory,
    revision,
    join(directory, "source.pack"),
  );
  const source = {
    schema: 1,
    revision,
    parents,
    fileCount: files.length,
    packSha256: hash(await readFile(join(directory, "source.pack"))),
  };
  await writeFile(
    join(directory, "source.json"),
    JSON.stringify(source, null, 2),
  );
  return source;
}
