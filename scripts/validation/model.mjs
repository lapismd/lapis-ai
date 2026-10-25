import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";

export const label = "dev.lapismd.lapis-ai.validation";
export const hash = (value) => createHash("sha256").update(value).digest("hex");
export const workspace = (root) => hash(root).slice(0, 16);
export const imageKey = (inputs) =>
  hash(
    JSON.stringify(
      Object.fromEntries(
        Object.entries(inputs).sort(([a], [b]) => a.localeCompare(b)),
      ),
    ),
  ).slice(0, 20);
export function validationKeys(inputs) {
  const toolKeys = [
    "scripts/validation/Dockerfile",
    "scripts/validation/install-tools.mjs",
    "packageManager",
    "toolchain",
    "architecture",
  ];
  const imageInputs = Object.fromEntries(
    toolKeys.map((key) => [key, inputs[key]]),
  );
  const cacheInputs = Object.fromEntries(
    Object.entries(inputs).filter(
      ([key]) => !["packageManager", "toolchain"].includes(key),
    ),
  );
  return {
    image: imageKey(imageInputs),
    cache: imageKey(cacheInputs),
    imageInputs,
    cacheInputs,
  };
}
export const stoppedRunState = (state) => ["exited", "dead"].includes(state);
export const leaseReleaseAllowed = (active) =>
  Array.isArray(active) && active.length === 0;
export function validationConfig(local = {}, env = process.env) {
  const config = {
    runtime: "docker",
    cpus: 4,
    memoryGiB: 12,
    shmGiB: 2,
    ...local,
  };
  config.runtime = env.LAPIS_AI_VALIDATION_RUNTIME ?? config.runtime;
  config.context = env.LAPIS_AI_VALIDATION_CONTEXT ?? config.context;
  if (config.context === undefined) delete config.context;
  if (!["docker", "native"].includes(config.runtime))
    throw Error("Invalid validation runtime");
  if (
    config.archives !== undefined ||
    env.LAPIS_AI_VALIDATION_ARCHIVES !== undefined
  )
    throw Error(
      "AI workers accept self-contained tracked source only, not package/model archives",
    );
  if (
    config.transport !== undefined &&
    !["docker", "ssh-cli"].includes(config.transport)
  )
    throw Error("Invalid Docker transport");
  if (
    config.context !== undefined &&
    (typeof config.context !== "string" || !config.context.trim())
  )
    throw Error("Invalid Docker context");
  for (const key of ["cpus", "memoryGiB", "shmGiB"])
    if (!Number.isFinite(config[key]) || config[key] <= 0)
      throw Error(`Invalid ${key}`);
  return config;
}
export function nativeInvocation(lanes, lane, args, root, env = process.env) {
  if (!Object.hasOwn(lanes, lane))
    throw Error(`Unknown validation lane: ${lane}`);
  return {
    command: "/bin/sh",
    args: ["-c", lanes[lane] + ' "$@"', "validation", ...args],
    env: {
      ...env,
      LAPIS_AI_VALIDATION_RUNTIME: "native",
      PATH: join(root, "node_modules/.bin") + ":" + (env.PATH ?? ""),
    },
  };
}
export function auditRouting(scripts, lanes) {
  const native = JSON.parse(
    readFileSync(new URL("./native-scripts.json", import.meta.url), "utf8"),
  );
  const findings = [];
  for (const [name, command] of Object.entries(scripts))
    if (!Object.hasOwn(lanes, name) && native[name] !== command)
      findings.push(
        `Script ${name} needs a Docker lane or an explicitly reviewed native exception`,
      );
  for (const name of Object.keys(lanes))
    if (scripts[name] !== `node scripts/validation.mjs run ${name}`)
      findings.push(
        `Portable script ${name} must route through the fail-closed Docker runner`,
      );
  return findings;
}
export async function dispatch(config, native, docker) {
  return config.runtime === "native" ? native() : docker();
}
export function sourcePathViolation(path) {
  return (
    path
      .split("/")
      .some((part) =>
        /^(?:\.\.?|\.local|\.env(?:\..*)?|node_modules|target|\.cache|\.ssh|secrets?|credentials(?:\..*)?)$/i.test(
          part,
        ),
      ) ||
    /\.(?:pem|key|p12|pfx|gguf|ggml|onnx|safetensors|pt|pth|ckpt|binmodel)$/i.test(
      path,
    ) ||
    path.startsWith("/") ||
    path.includes("\\")
  );
}
