import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
export function sourceIntakeReader(
  root,
  execute = execFileSync,
  hasJj = existsSync(resolve(root, ".jj")),
) {
  const run = (binary, args) =>
    execute(binary, args, {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 3 * 1024 * 1024,
    });
  const parent = (
    hasJj
      ? run("jj", [
          "--ignore-working-copy",
          "log",
          "-r",
          "@",
          "--no-graph",
          "-T",
          'parents.map(|p| p.commit_id()).join("\\n")',
        ])
      : run("git", ["rev-parse", "HEAD^1"])
  )
    .trim()
    .split("\n")[0];
  if (!/^[a-f0-9]{40}$/.test(parent))
    throw Error("Source intake ancestry unavailable");
  const read = (path) =>
    hasJj
      ? run("jj", [
          "--ignore-working-copy",
          "file",
          "show",
          "-r",
          parent,
          "--",
          path,
        ])
      : run("git", ["show", `${parent}:${path}`]);
  read.sourceSha = parent;
  return read;
}
