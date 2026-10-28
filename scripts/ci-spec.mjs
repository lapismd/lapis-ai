/**
 * Node-only hosted spec gate: validators, mdBook, and governance that do not
 * need a Rust toolchain, cargo fetch, or compile. Cargo-backed lanes
 * (format:check, boundaries, test:governance) run after fetch in rust-build.
 */
import { spawnSync } from "node:child_process";

function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status) process.exit(result.status);
}

const base = process.env.LAPIS_AI_CHECK_BASE;
const head = process.env.LAPIS_AI_CHECK_HEAD;
if (!base || !head)
  throw new Error("LAPIS_AI_CHECK_BASE and LAPIS_AI_CHECK_HEAD are required");

run("pnpm", ["check:validation-routing"]);
run("pnpm", ["exec", "spec-validator", "validate"]);
run("pnpm", [
  "exec",
  "spec-validator",
  "first",
  "--base",
  base,
  "--head",
  head,
]);
run("pnpm", ["exec", "spec-validator", "build"]);
run("pnpm", ["check:governance"]);
run("pnpm", ["exec", "prettier", "--check", "."]);
run("pnpm", ["check:client"]);
run("pnpm", ["test:client"]);
run("pnpm", ["check:workflow"]);
console.log("Node-only spec gate passed");
