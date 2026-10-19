import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
const acp = /^lapis-acp(?:-|$)/;
const forbidden =
  /^(nostr.*|reqwest|axum|hyper|rustls|rusqlite|sqlx.*|turso|tauri.*)$/;
export const allowed = {
  "lapis-ai-app-tools": [],
  "lapis-ai-capture": [],
  "tauri-plugin-lapis-ai": ["lapis-ai-runtime"],
  "lapis-acp-protocol": [],
  "lapis-acp-agent": ["lapis-acp-protocol"],
  "lapis-acp-runner": ["lapis-acp-protocol", "lapis-acp-agent"],
  "lapis-acp-control": [
    "lapis-acp-protocol",
    "lapis-acp-agent",
    "lapis-acp-runner",
  ],
  "lapis-acp": [
    "lapis-acp-protocol",
    "lapis-acp-agent",
    "lapis-acp-runner",
    "lapis-acp-control",
    "lapis-ai-config",
  ],
  "lapis-acp-testkit": ["lapis-acp-protocol", "lapis-acp-agent"],
  "lapis-agent": ["lapis-ai-config"],
  "lapis-agent-tools": [],
  "lapis-compute": ["lapis-ai-config"],
  "lapis-ai-config": [],
  "lapis-ai-runtime": [
    "lapis-ai-config",
    "lapis-acp-control",
    "lapis-acp-protocol",
  ],
};
export function boundaryErrors(packages, root = process.cwd()) {
  const errors = [];
  const names = new Set(packages.map((p) => p.name));
  for (const name of Object.keys(allowed))
    if (!names.has(name)) errors.push(`Missing package: ${name}`);
  for (const pkg of packages) {
    if (!(pkg.name in allowed)) {
      errors.push(`${pkg.name}: missing boundary declaration`);
      continue;
    }
    for (const dep of pkg.dependencies) {
      if (dep.path && !dep.path.startsWith(root + "/"))
        errors.push(`${pkg.name}: sibling checkout dependency`);
      if (dep.kind === "dev") continue;
      if (
        (acp.test(pkg.name) ||
          pkg.name === "lapis-ai-config" ||
          pkg.name === "lapis-ai-capture" ||
          pkg.name === "lapis-ai-app-tools") &&
        forbidden.test(dep.name)
      )
        errors.push(`${pkg.name}: excluded ${dep.name}`);
      if (names.has(dep.name) && !allowed[pkg.name].includes(dep.name))
        errors.push(`${pkg.name}: invalid edge ${dep.name}`);
      if (
        pkg.name === "lapis-ai-capture" &&
        /^(?:jj|lapis-(?:acp|agent|compute|ai-runtime)|tokio)/.test(dep.name)
      )
        errors.push(
          `${pkg.name}: source acquisition cannot depend on ${dep.name}`,
        );
      if (dep.name.includes("testkit"))
        errors.push(`${pkg.name}: production depends on testkit`);
    }
  }
  return errors;
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const { packages } = JSON.parse(
    execFileSync(
      "cargo",
      ["metadata", "--no-deps", "--format-version", "1", "--locked"],
      { encoding: "utf8" },
    ),
  );
  const errors = boundaryErrors(packages);
  if (errors.length) throw new Error(errors.join("\n"));
  console.log(`Boundaries pass (${packages.length} packages)`);
}
