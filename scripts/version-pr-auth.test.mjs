import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import YAML from "yaml";

const workflow = YAML.parse(
  readFileSync(
    new URL("../.github/workflows/changesets.yml", import.meta.url),
    "utf8",
  ),
);
const version = workflow.jobs.version;
const writer = version.steps.find((step) =>
  step.uses?.startsWith("changesets/action@"),
);

test("version PRs fail closed on canonical main with a repository-scoped App", () => {
  assert.equal(
    version.if,
    "github.repository == 'lapismd/lapis-ai' && github.ref == 'refs/heads/main'",
  );
  const app = version.steps.find((step) =>
    step.uses?.startsWith("actions/create-github-app-token@"),
  );
  assert.ok(app, "version PR writes require an App token");
  assert.equal(app.id, "app");
  assert.equal(app.if, "steps.runtime.outputs.hasChangesets == 'true'");
  assert.equal(writer.if, app.if);
  const guard = version.steps.find((step) => step.id === "runtime");
  assert.equal(
    guard.uses,
    "lapismd/lapis-ci/actions/guard-version-runtime@dc81ff65ba812bf005fb2a806eb3308bb8b5f71f",
  );
  assert.equal(guard.with["detect-release-intent"], true);
  assert.equal(
    guard.with["qualification-file"],
    ".github/version-runtime-qualification.json",
  );
  assert.ok(version.steps.indexOf(guard) < version.steps.indexOf(app));
  const intake = version.steps.find(
    (step) => step.run === "pnpm check:dependency-intake",
  );
  assert.ok(
    intake && version.steps.indexOf(intake) < version.steps.indexOf(app),
  );
  assert.equal(app["continue-on-error"], undefined);
  assert.deepEqual(app.with, {
    "client-id": "${{ vars.LAPIS_DEPENDENCY_APP_CLIENT_ID }}",
    "private-key": "${{ secrets.LAPIS_DEPENDENCY_APP_PRIVATE_KEY }}",
    owner: "${{ github.repository_owner }}",
    repositories: "${{ github.event.repository.name }}",
    "permission-contents": "write",
    "permission-pull-requests": "write",
  });
  assert.ok(version.steps.indexOf(app) < version.steps.indexOf(writer));
  assert.equal(writer.env.GITHUB_TOKEN, "${{ steps.app.outputs.token }}");
  assert.equal(writer.with.version, "pnpm release:version");
  assert.equal(writer.with.commitMode, "github-api");
  assert.equal(writer.with.createGithubReleases, false);
  assert.equal(writer.with.publish, undefined);
});

test("App credentials do not replace CI dispatch and release orchestration tokens", () => {
  const checkout = version.steps.find((step) =>
    step.uses?.startsWith("actions/checkout@"),
  );
  assert.equal(checkout.with["persist-credentials"], false);
  for (const job of Object.values(workflow.jobs)) {
    for (const step of job.steps ?? []) {
      if (step.run?.includes("scripts/release-orchestration.mjs"))
        assert.equal(step.env.GITHUB_TOKEN, "${{ github.token }}");
    }
  }
  assert.ok(
    version.steps.some(
      (step) => step.run === "node scripts/release-orchestration.mjs checks",
    ),
  );
  assert.equal(workflow.jobs.qualify.needs, "version");
  assert.deepEqual(workflow.jobs.tag.needs, ["version", "qualify"]);
  assert.equal(version.permissions.actions, "write");
  for (const step of version.steps.filter((step) => step.uses))
    assert.match(step.uses, /@[a-f0-9]{40}$/);
});
