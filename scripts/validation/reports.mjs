import { readFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import {
  label,
  workspace,
  stoppedRunState,
  leaseReleaseAllowed,
} from "./model.mjs";
import { output, docker, saveLogs, receiveDirectory } from "./docker.mjs";

const owned = (config, root, kind) =>
  output(config, [
    "ps",
    "-aq",
    "--filter",
    `label=${label}=${kind}`,
    "--filter",
    `label=${label}.workspace=${workspace(root)}`,
  ])
    .split("\n")
    .filter(Boolean);
const status = (config, id) =>
  output(config, ["inspect", id, "--format", "{{.State.Status}} "]);
const runName = (config, id) =>
  output(config, ["inspect", id, "--format", "{{.Name}} "]).replace(/^\//, "");
export async function collect(config, root, run) {
  if (
    !/^lapis-ai-run-[a-f0-9-]{36}$/.test(run) ||
    output(config, [
      "inspect",
      run,
      "--format",
      `{{index .Config.Labels "${label}.workspace"}}`,
    ]) !== workspace(root)
  )
    throw Error("Refusing a validation run owned by another checkout");
  if (!stoppedRunState(status(config, run)))
    throw Error("Only a completed validation run can be collected");
  const directory = join(root, ".local/validation/runs", run);
  await mkdir(directory, { recursive: true });
  await receiveDirectory(config, ["cp", `${run}:/results/.`, "-"], directory);
  await saveLogs(config, run, join(directory, "console.log"));
  const result = JSON.parse(
    await readFile(join(directory, "result.json"), "utf8"),
  );
  if (
    result.run !== run ||
    !["passed", "failed"].includes(result.status) ||
    !result.finished
  )
    throw Error("Incomplete or mismatched validation result");
  if (result.collectionError) throw Error(result.collectionError);
  return result;
}
export async function clean(config, root, apply = false) {
  const ids = owned(config, root, "run");
  const stopped = ids.filter((id) => stoppedRunState(status(config, id)));
  console.log({ stoppedRuns: stopped, apply });
  if (!apply) return;
  const completed = new Set();
  for (const id of stopped) {
    const run = runName(config, id);
    await collect(config, root, run);
    // Recheck immediately; never remove a prepared, paused or active run.
    if (!stoppedRunState(status(config, id)))
      throw Error("Validation run became active during cleanup");
    await docker(config, ["rm", id]);
    completed.add(run);
  }
  const active = owned(config, root, "run").filter(
    (id) => !stoppedRunState(status(config, id)),
  );
  if (!leaseReleaseAllowed(active)) return;
  for (const id of owned(config, root, "lease")) {
    const run = output(config, [
      "inspect",
      id,
      "--format",
      `{{index .Config.Labels "${label}.run"}}`,
    ]);
    let known = completed.has(run);
    if (!known && /^lapis-ai-run-[a-f0-9-]{36}$/.test(run)) {
      try {
        const receipt = JSON.parse(
          await readFile(
            join(root, ".local/validation/runs", run, "result.json"),
            "utf8",
          ),
        );
        known =
          receipt.run === run &&
          ["passed", "failed"].includes(receipt.status) &&
          !!receipt.finished &&
          !receipt.collectionError;
      } catch {
        /* An unproven or interrupted preparation remains leased. */
      }
    }
    if (known) await docker(config, ["rm", id]);
  }
}
