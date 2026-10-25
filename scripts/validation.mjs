import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  dispatch,
  nativeInvocation,
  validationConfig,
} from "./validation/model.mjs";
import { execute, output } from "./validation/docker.mjs";
import { image, remoteRun } from "./validation/runs.mjs";
import { clean, collect } from "./validation/reports.mjs";

export const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const json = async (path, fallback) => {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return fallback;
    throw error;
  }
};
export async function runNative(lane, args = [], directory = root) {
  const lanes = await json(
    join(directory, "scripts/validation/lanes.json"),
    {},
  );
  const invocation = nativeInvocation(lanes, lane, args, directory);
  return execute(invocation.command, invocation.args, {
    cwd: directory,
    env: invocation.env,
  });
}
export async function main(argv = process.argv.slice(2)) {
  if (argv[0] === "--") argv.shift();
  const [command, ...args] = argv;
  const config = validationConfig(
    await json(join(root, ".local/validation/config.json"), {}),
  );
  if (command === "run") {
    const [lane, ...rest] = args;
    if (rest[0] === "--") rest.shift();
    const lanes = await json(join(root, "scripts/validation/lanes.json"), {});
    if (!Object.hasOwn(lanes, lane))
      throw Error(`Unknown validation lane: ${lane}`);
    return dispatch(
      config,
      () => runNative(lane, rest),
      () => remoteRun(config, root, lane, rest),
    );
  }
  if (command === "images")
    return console.log(
      await image(config, root, undefined, args.includes("--rebuild")),
    );
  if (command === "doctor")
    return console.log({
      config,
      daemon: JSON.parse(output(config, ["info", "--format", "{{json .}} "])),
    });
  if (command === "collect") return collect(config, root, args[0]);
  if (command === "clean") return clean(config, root, args.includes("--apply"));
  throw Error(
    "Use run <lane>, images [--rebuild], doctor, collect <run>, or clean [--apply]",
  );
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    await main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
