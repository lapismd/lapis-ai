import { appendFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

/** Same-repo CI may write Rust dependency caches; forks stay read-only. */
export function cacheWritable({
  event,
  repository,
  headRepository = repository,
}) {
  if (repository !== "lapismd/lapis-ai") return false;
  if (event === "pull_request") return headRepository === repository;
  return ["push", "workflow_dispatch", "workflow_call"].includes(event);
}

if (import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const value = cacheWritable({
    event: process.env.CI_EVENT || process.env.GITHUB_EVENT_NAME,
    repository: process.env.GITHUB_REPOSITORY,
    headRepository:
      process.env.CI_HEAD_REPOSITORY || process.env.GITHUB_REPOSITORY,
  });
  if (process.env.GITHUB_OUTPUT)
    appendFileSync(process.env.GITHUB_OUTPUT, `cache_write=${value}\n`);
  console.log(`cache_write=${value}`);
}
