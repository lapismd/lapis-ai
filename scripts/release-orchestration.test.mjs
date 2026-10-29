import test from "node:test";
import assert from "node:assert/strict";
import {
  awaitQualifiedCi,
  dispatchVersionChecks,
  githubClient,
  inspectExactSourceCi,
  planTag,
  publishTag,
  REQUIRED_CI_JOBS,
  summarizeCiJobs,
} from "./release-orchestration.mjs";

const state = { tag: "v0.1.1", pending: 0 };
const sha = "a".repeat(40);
test("pending_notes_never_create_or_dispatch_a_release", async () => {
  let calls = 0;
  assert.equal(
    (await publishTag({ ...state, pending: 1 }, sha, () => calls++)).candidate,
    false,
  );
  assert.equal(calls, 0);
});
test("already_released_history_never_retags_newer_main", async () => {
  const calls = [];
  const result = await publishTag(state, sha, async (...args) => {
    calls.push(args);
    return { status: 200, data: { object: { sha: "b".repeat(40) } } };
  });
  assert.equal(result.candidate, false);
  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], "git/ref/tags/v0.1.1");
});
test("new_version_dispatches_the_exact_qualified_immutable_tag", async () => {
  const calls = [];
  const results = [{ status: 404 }, { status: 201 }, { status: 204 }];
  assert.deepEqual(
    await publishTag(state, sha, async (...args) => {
      calls.push(args);
      return results.shift();
    }),
    { dispatched: true, tag: state.tag },
  );
  assert.deepEqual(calls[1], [
    "git/refs",
    "POST",
    { ref: "refs/tags/v0.1.1", sha },
  ]);
  assert.deepEqual(calls[2], [
    "actions/workflows/release.yml/dispatches",
    "POST",
    { ref: state.tag, inputs: { tag: state.tag } },
  ]);
});
test("uncertain_tag_lookup_and_conflicting_races_block_publication", async () => {
  await assert.rejects(
    planTag(state, async () => ({ status: 422 })),
    /tag absence/,
  );
  const responses = [
    { status: 404 },
    { status: 422 },
    { status: 200, data: { object: { sha: "b".repeat(40) } } },
  ];
  await assert.rejects(
    publishTag(state, sha, async () => responses.shift()),
    /refusing replacement/,
  );
  assert.equal(responses.length, 0);
});
test("bot_version_pr_explicitly_dispatches_required_ci_and_pages_build", async () => {
  const calls = [];
  await dispatchVersionChecks(7, async (...args) => {
    calls.push(args);
    return calls.length === 1
      ? {
          status: 200,
          data: {
            base: { ref: "main" },
            head: {
              ref: "changeset-release/main",
              repo: { full_name: "lapismd/lapis-ai" },
            },
          },
        }
      : { status: 204 };
  });
  assert.deepEqual(calls.slice(1), [
    [
      "actions/workflows/ci.yml/dispatches",
      "POST",
      { ref: "changeset-release/main" },
    ],
    [
      "actions/workflows/publish-spec-pages.yml/dispatches",
      "POST",
      { ref: "changeset-release/main" },
    ],
  ]);
});
test("release_orchestration_refuses_forks_and_non_main_context", () => {
  for (const env of [
    { GITHUB_REPOSITORY: "someone/jj-acp", GITHUB_REF: "refs/heads/main" },
    {
      GITHUB_REPOSITORY: "lapismd/lapis-ai",
      GITHUB_REF: "refs/heads/feature",
    },
  ])
    assert.throws(() => githubClient(env), /canonical main/);
});
test("github_errors_are_fail_closed_without_disclosing_credentials", async () => {
  const client = githubClient(
    {
      GITHUB_REPOSITORY: "lapismd/lapis-ai",
      GITHUB_REF: "refs/heads/main",
      GITHUB_TOKEN: "fixture-private-token",
    },
    async () => ({ ok: false, status: 503 }),
  );
  await assert.rejects(client("git/ref/tags/v0.1.1"), {
    message: "GitHub orchestration HTTP 503",
  });
});

function jobs(overrides = {}) {
  return REQUIRED_CI_JOBS.map((name) => ({
    name,
    status: "completed",
    conclusion: "success",
    ...overrides[name],
  }));
}

test("summarize_ci_jobs_requires_every_release_gate", () => {
  assert.equal(summarizeCiJobs(jobs()).state, "success");
  assert.equal(
    summarizeCiJobs(jobs({ Plan: { conclusion: "failure" } })).state,
    "failure",
  );
  assert.equal(
    summarizeCiJobs(jobs({ Spec: { status: "in_progress", conclusion: null } }))
      .state,
    "pending",
  );
  assert.equal(
    summarizeCiJobs(jobs().filter((job) => job.name !== "Canonical checks"))
      .state,
    "incomplete",
  );
});

test("inspect_ignores_workflow_call_duplicates_and_accepts_green_push_ci", async () => {
  const result = await inspectExactSourceCi(sha, async (path) => {
    if (path.startsWith("actions/workflows/ci.yml/runs"))
      return {
        status: 200,
        data: {
          workflow_runs: [
            { id: 1, event: "workflow_call", status: "completed" },
            { id: 2, event: "push", status: "completed" },
          ],
        },
      };
    assert.equal(path, "actions/runs/2/jobs?per_page=100");
    return { status: 200, data: { jobs: jobs() } };
  });
  assert.deepEqual(result, { state: "success", runId: 2 });
});

test("await_ci_reuses_green_exact_source_ci_without_dispatch", async () => {
  let calls = 0;
  const result = await awaitQualifiedCi(
    sha,
    async (path) => {
      calls++;
      if (path.startsWith("actions/workflows/ci.yml/runs"))
        return {
          status: 200,
          data: {
            workflow_runs: [
              { id: 9, event: "workflow_dispatch", status: "completed" },
            ],
          },
        };
      assert.match(path, /^actions\/runs\/9\/jobs/);
      return { status: 200, data: { jobs: jobs() } };
    },
    { sleep: async () => {}, pollMs: 0, deadlineMs: 1_000 },
  );
  assert.deepEqual(result, { qualified: true, runId: 9, dispatched: false });
  assert.equal(calls, 2);
});

test("await_ci_dispatches_once_when_main_tip_has_no_ci", async () => {
  const calls = [];
  let tick = 0;
  const result = await awaitQualifiedCi(
    sha,
    async (path, method = "GET", body) => {
      calls.push([path, method, body]);
      if (path.startsWith("actions/workflows/ci.yml/runs")) {
        tick++;
        return {
          status: 200,
          data: {
            workflow_runs:
              tick < 3
                ? []
                : [{ id: 11, event: "workflow_dispatch", status: "completed" }],
          },
        };
      }
      if (path === "git/ref/heads/main")
        return { status: 200, data: { object: { sha } } };
      if (path === "actions/workflows/ci.yml/dispatches")
        return { status: 204 };
      if (path.startsWith("actions/runs/11/jobs"))
        return { status: 200, data: { jobs: jobs() } };
      throw Error(path);
    },
    { sleep: async () => {}, now: () => tick * 10, pollMs: 1, deadlineMs: 100 },
  );
  assert.deepEqual(result, { qualified: true, runId: 11, dispatched: true });
  assert.ok(
    calls.some(
      ([path, method]) =>
        path === "actions/workflows/ci.yml/dispatches" && method === "POST",
    ),
  );
});

test("await_ci_fails_closed_on_red_ci_and_refuses_off_tip_dispatch", async () => {
  await assert.rejects(
    awaitQualifiedCi(
      sha,
      async (path) => {
        if (path.startsWith("actions/workflows/ci.yml/runs"))
          return {
            status: 200,
            data: {
              workflow_runs: [{ id: 3, event: "push", status: "completed" }],
            },
          };
        return {
          status: 200,
          data: {
            jobs: jobs({
              "Native aarch64-apple-darwin": { conclusion: "failure" },
            }),
          },
        };
      },
      { sleep: async () => {}, pollMs: 0, deadlineMs: 1_000 },
    ),
    /exact-source CI failed/,
  );
  await assert.rejects(
    awaitQualifiedCi(
      sha,
      async (path) => {
        if (path.startsWith("actions/workflows/ci.yml/runs"))
          return { status: 200, data: { workflow_runs: [] } };
        return { status: 200, data: { object: { sha: "b".repeat(40) } } };
      },
      { sleep: async () => {}, pollMs: 0, deadlineMs: 1_000 },
    ),
    /not main tip/,
  );
});
