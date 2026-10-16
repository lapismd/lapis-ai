import { statfsSync, mkdirSync, mkdtempSync } from "node:fs";
import { spawnSync } from "node:child_process";
const available = statfsSync(process.cwd());
const gib = (Number(available.bavail) * Number(available.bsize)) / 1024 ** 3;
if (gib < 30)
  throw new Error(`Build stopped: ${gib.toFixed(1)} GiB free; 30 required`);
const args = process.argv.slice(2);
if (args[0] === "--") args.shift();
const env = { ...process.env };
if (args.includes("test") && !env.JJ_STORE_COMPUTE_TEST_ROOT) {
  mkdirSync(".local/test-runs", { recursive: true });
  env.JJ_STORE_COMPUTE_TEST_ROOT = `${process.cwd()}/${mkdtempSync(".local/test-runs/compute-")}`;
}
const result = spawnSync("cargo", args, { stdio: "inherit", env });
if (result.error) throw result.error;
process.exit(result.status ?? 1);
