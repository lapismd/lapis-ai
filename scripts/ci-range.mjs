import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync } from "node:fs";
import { associatedPullRequests, selectAutomation } from "./ci-automation.mjs";
const git = (args, input) =>
  execFileSync("git", args, { encoding: "utf8", input }).trim();
const head = git(["rev-parse", process.env.CI_HEAD_SHA || "HEAD"]);
let base = process.env.CI_BASE_SHA;
if (base && /^0+$/.test(base)) {
  base = git(["hash-object", "-t", "tree", "--stdin"], "");
} else if (!base) {
  try {
    base = git(["rev-parse", `${head}^`]);
  } catch {
    base = git(["hash-object", "-t", "tree", "--stdin"], "");
  }
} else if (process.env.CI_EVENT === "pull_request") {
  base = git(["merge-base", base, head]);
}
if (!/^[a-f0-9]{40,64}$/.test(base) || !/^[a-f0-9]{40,64}$/.test(head))
  throw Error("invalid CI revision");
let parent;
try {
  parent = git(["rev-parse", `${head}^`]);
} catch {}
const automation = await selectAutomation({
  eventName: process.env.CI_EVENT,
  event: JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, "utf8")),
  repository: process.env.GITHUB_REPOSITORY,
  head,
  base,
  parent,
  associated: () => associatedPullRequests(process.env.GITHUB_REPOSITORY, head),
});
appendFileSync(
  process.env.GITHUB_ENV,
  `LAPIS_AI_CHECK_BASE=${base}\nLAPIS_AI_CHECK_HEAD=${head}\nLAPIS_AI_CHECK_AUTOMATION=${automation}\n`,
);
console.log(`Checking ${base}..${head}`);
if (automation)
  console.log(
    `${automation}: exempt from same-change spec/evidence bookkeeping; all other checks remain required`,
  );
