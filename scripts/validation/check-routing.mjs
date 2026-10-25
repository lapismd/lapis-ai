import { readFileSync } from "node:fs";
import { auditRouting } from "./model.mjs";

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const lanes = JSON.parse(readFileSync("scripts/validation/lanes.json", "utf8"));
const findings = auditRouting(pkg.scripts, lanes);
for (const finding of findings)
  console.error(`package.json:1 [VALIDATION-ROUTING] LAI-GOV-105: ${finding}`);
console.log(
  `Portable validation routing: ${Object.keys(lanes).length} lanes, ${findings.length} findings.`,
);
process.exitCode = findings.length ? 1 : 0;
