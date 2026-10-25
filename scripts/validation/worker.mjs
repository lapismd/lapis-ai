import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { hash } from "./model.mjs";
import { execute, importPack } from "./docker.mjs";

const root = "/work/lapis-ai",
  input = "/input",
  reports = "/results";
const source = JSON.parse(await readFile(join(input, "source.json"), "utf8"));
const request = JSON.parse(await readFile(join(input, "request.json"), "utf8"));
const run = (command, args, options = {}) =>
  execFileSync(
    command,
    command === "jj"
      ? [
          "--config",
          'user.name="Lapis AI validation"',
          "--config",
          'user.email="validation@localhost"',
          ...args,
        ]
      : args,
    { cwd: root, stdio: "inherit", ...options },
  );
const jj = (args) =>
  run("jj", ["--ignore-working-copy", ...args], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
const result = {
  schema: 1,
  ...request,
  source,
  started: new Date().toISOString(),
  status: "running",
};
try {
  if (hash(await readFile(join(input, "source.pack"))) !== source.packSha256)
    throw Error("Source pack checksum mismatch");
  await writeFile(join(root, ".gitignore"), "node_modules/\n.local/\n");
  run("jj", ["git", "init", "--colocate", root]);
  // Import immutable objects; Jujutsu selects the exact checked source revision.
  await importPack(root, join(input, "source.pack"));
  run("git", ["update-ref", "refs/heads/validation-snapshot", source.revision]);
  run("jj", ["git", "import"]);
  run("jj", ["edit", source.revision]);
  if (
    jj(["log", "-r", "@", "--no-graph", "-T", "commit_id"]) !== source.revision
  )
    throw Error("Source revision mismatch");
  const parents = jj([
    "log",
    "-r",
    "@-",
    "--no-graph",
    "-T",
    'commit_id ++ "\\n"',
  ])
    .split("\n")
    .filter(Boolean);
  if (
    JSON.stringify(parents.sort()) !==
    JSON.stringify([...source.parents].sort())
  )
    throw Error("Source ancestry mismatch");
  process.env.LAPIS_AI_VALIDATION_RUNTIME = "native";
  process.env.LAPIS_AI_VALIDATION_WORKER = "1";
  process.env.CI = "true";
  await execute(
    "pnpm",
    [
      "install",
      "--frozen-lockfile",
      "--ignore-pnpmfile",
      "--store-dir=/cache/pnpm",
    ],
    { cwd: root },
  );
  await execute("pnpm", ["dev:cargo", "--", "fetch", "--locked"], {
    cwd: root,
  });
  result.tools = Object.fromEntries(
    ["node", "pnpm", "jj", "git", "mdbook", "rustc", "cargo", "actionlint"].map(
      (tool) => [
        tool,
        run(tool, ["--version"], {
          encoding: "utf8",
          stdio: ["ignore", "pipe", "pipe"],
        }).trim(),
      ],
    ),
  );
  const { runNative } = await import(join(root, "scripts/validation.mjs"));
  await runNative(request.lane, request.args, root);
  result.status = "passed";
  result.exitCode = 0;
} catch (error) {
  console.error(error);
  result.status = "failed";
  result.error = error.message;
  result.exitCode = 1;
} finally {
  result.finished = new Date().toISOString();
  for (const path of ["spec/book", "packages/ai/dist"])
    if (existsSync(join(root, path))) {
      try {
        await execute("cp", [
          "-a",
          join(root, path),
          join(reports, path.replaceAll("/", "_")),
        ]);
      } catch (error) {
        result.collectionError = error.message;
      }
    }
  await writeFile(
    join(reports, "result.json"),
    JSON.stringify(result, null, 2),
  );
}
process.exitCode = result.exitCode;
