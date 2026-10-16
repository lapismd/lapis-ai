import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import {
  associatedPullRequests,
  identifyAutomation,
  selectAutomation,
} from "./ci-automation.mjs";

const repository = "stevejuma/lapis-ai";
const head = "a".repeat(40);
const base = "b".repeat(40);
const pr = (kind = "dependabot") => ({
  user: {
    login: kind === "dependabot" ? "dependabot[bot]" : "github-actions[bot]",
    type: "Bot",
  },
  base: { ref: "main", repo: { full_name: repository } },
  head: {
    ref:
      kind === "dependabot"
        ? "dependabot/cargo/tokio-1.53.3"
        : "changeset-release/main",
    sha: head,
    repo: { full_name: repository },
  },
});
test("identifies_both_bots_without_exempting_ordinary_or_fork_prs", () => {
  for (const kind of ["dependabot", "changesets"]) {
    assert.equal(identifyAutomation(pr(kind), repository), kind);
    for (const altered of [
      { ...pr(kind), user: { login: "stevejuma", type: "User" } },
      { ...pr(kind), head: { ...pr(kind).head, ref: "feature" } },
      {
        ...pr(kind),
        head: { ...pr(kind).head, repo: { full_name: "fork/jj-acp" } },
      },
      { ...pr(kind), base: { ...pr(kind).base, ref: "other" } },
    ])
      assert.equal(identifyAutomation(altered, repository), "");
  }
});
test("pull_request_exemption_requires_the_exact_checked_head", async () => {
  const options = {
    eventName: "pull_request",
    event: { pull_request: pr() },
    repository,
    head,
    base,
    parent: base,
  };
  assert.equal(await selectAutomation(options), "dependabot");
  assert.equal(
    await selectAutomation({ ...options, head: "c".repeat(40) }),
    "",
  );
});
test("dispatch_and_merged_qualification_recognize_associated_bot_prs", async () => {
  for (const kind of ["dependabot", "changesets"]) {
    for (const eventName of ["workflow_dispatch", "push", "workflow_call"]) {
      for (const merged of [false, true]) {
        const candidate = merged
          ? {
              ...pr(kind),
              head: { ...pr(kind).head, sha: base },
              merged_at: "2026-10-14",
              merge_commit_sha: head,
            }
          : pr(kind);
        assert.equal(
          await selectAutomation({
            eventName,
            event: {},
            repository,
            head,
            base,
            parent: base,
            associated: async () => [candidate],
          }),
          kind,
        );
      }
    }
  }
});
test("unrelated_heads_and_multi_change_pushes_keep_bookkeeping", async () => {
  const options = {
    eventName: "push",
    event: {},
    repository,
    head,
    base,
    parent: base,
    associated: async () => [{ ...pr(), head: { ...pr().head, sha: base } }],
  };
  assert.equal(await selectAutomation(options), "");
  assert.equal(
    await selectAutomation({
      ...options,
      parent: "c".repeat(40),
      associated: async () => [pr()],
    }),
    "",
  );
  await assert.rejects(
    selectAutomation({
      ...options,
      associated: async () => {
        throw Error("GitHub lookup failed");
      },
    }),
    /lookup failed/,
  );
});
test("github_lookup_errors_fail_closed_without_disclosing_credentials", async () => {
  await assert.rejects(
    associatedPullRequests(
      repository,
      head,
      { GITHUB_TOKEN: "private-fixture-token" },
      async () => ({ ok: false, status: 403 }),
    ),
    { message: "Automation PR lookup HTTP 403" },
  );
  await assert.rejects(
    associatedPullRequests(repository, head, {}, async () => ({
      ok: true,
      json: async () => ({}),
    })),
    /Invalid automation PR lookup response/,
  );
});
test("automation_skips_only_spec_first_and_preserves_canonical_lanes", () => {
  const script =
    'const {default:c}=await import("./spec-validator.config.mjs"); console.log(JSON.stringify(c));';
  const load = (kind) =>
    JSON.parse(
      execFileSync(process.execPath, ["--input-type=module", "-e", script], {
        encoding: "utf8",
        env: { ...process.env, LAPIS_AI_CHECK_AUTOMATION: kind },
      }),
    );
  const ordinary = load("");
  for (const kind of ["dependabot", "changesets"]) {
    const automated = load(kind);
    assert.equal(automated.validators.specFirst, false);
    assert.deepEqual(automated.check, ordinary.check);
    for (const validator of ["summary", "governance", "verification", "book"])
      assert.deepEqual(
        automated.validators[validator],
        ordinary.validators[validator],
      );
    const result = spawnSync("pnpm", ["spec:first", "--file", "Cargo.lock"], {
      encoding: "utf8",
      env: { ...process.env, LAPIS_AI_CHECK_AUTOMATION: kind },
    });
    assert.equal(result.status, 0, result.stdout + result.stderr);
  }
  assert.deepEqual(load("unknown"), ordinary);
  const result = spawnSync("pnpm", ["spec:first", "--file", "Cargo.lock"], {
    encoding: "utf8",
    env: { ...process.env, LAPIS_AI_CHECK_AUTOMATION: "" },
  });
  assert.equal(result.status, 1, result.stdout + result.stderr);
});
test("ci_resolves_automation_before_checks_and_keeps_product_gates", () => {
  const ci = readFileSync(
    new URL("../.github/workflows/ci.yml", import.meta.url),
    "utf8",
  );
  assert.ok(
    ci.indexOf("node scripts/ci-range.mjs") < ci.indexOf("pnpm spec:check"),
  );
  assert.match(ci, /GITHUB_TOKEN: \$\{\{ github.token \}\}/);
  for (const command of [
    "pnpm check",
    "pnpm test",
    "pnpm build",
    "pnpm check:workflow",
    "pnpm package:check",
    "pnpm dev:cargo -- publish --workspace --exclude lapis-compute --locked --dry-run",
  ])
    assert.ok(ci.includes(command), command);
});
