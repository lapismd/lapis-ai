import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { parse } from "yaml";
const pin = "053ac8be573871ccc5b0232bc91d8ce7f3bff02f";
const workflow = parse(
  readFileSync(
    new URL(
      "../../.github/workflows/dependency-completed.yml",
      import.meta.url,
    ),
    "utf8",
  ),
);
test("completion observes only reviewed AI workflow names", () => {
  assert.deepEqual(workflow.on, {
    workflow_run: {
      workflows: [
        "CI",
        "Publish specification to GitHub Pages",
        "Changesets release orchestration",
      ],
      types: ["completed"],
    },
  });
  assert.equal(workflow.concurrency, undefined);
});
test("completion retains only read metadata and one repository App key", () => {
  assert.deepEqual(workflow.permissions, {
    contents: "read",
    actions: "read",
    "pull-requests": "read",
  });
  assert.deepEqual(Object.keys(workflow.jobs), ["notify"]);
  const job = workflow.jobs.notify;
  assert.equal(
    job.uses,
    "lapismd/dependency-automation/.github/workflows/completion.yml@" + pin,
  );
  assert.deepEqual(job.with, { "automation-ref": pin });
  assert.deepEqual(job.secrets, {
    "app-private-key": "${{ secrets.LAPIS_DEPENDENCY_APP_PRIVATE_KEY }}",
  });
  for (const key of [
    "steps",
    "env",
    "continue-on-error",
    "permissions",
    "strategy",
  ])
    assert.equal(job[key], undefined);
});
test("unsupported merge-group/workflow-call completions are skipped before the callback", () => {
  const expression = workflow.jobs.notify.if;
  assert.equal(
    expression,
    `github.event.workflow_run.conclusion == 'success' && contains(fromJSON('["pull_request","push","workflow_dispatch"]'), github.event.workflow_run.event)`,
  );
  const events = JSON.parse(/fromJSON\('(.*?)'\)/.exec(expression)[1]);
  for (const event of ["merge_group", "workflow_call", "release", "schedule"])
    assert.equal(events.includes(event), false);
  for (const event of ["pull_request", "push", "workflow_dispatch"])
    assert.equal(events.includes(event), true);
});
