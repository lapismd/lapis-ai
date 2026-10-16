import test from "node:test";
import assert from "node:assert/strict";
import {
  dispatchVersionChecks,
  githubClient,
  planTag,
  publishTag,
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
    { ref: state.tag, inputs: { tag: state.tag, publish_crates: true } },
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
              repo: { full_name: "stevejuma/lapis-ai" },
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
      GITHUB_REPOSITORY: "stevejuma/lapis-ai",
      GITHUB_REF: "refs/heads/feature",
    },
  ])
    assert.throws(() => githubClient(env), /canonical main/);
});
test("github_errors_are_fail_closed_without_disclosing_credentials", async () => {
  const client = githubClient(
    {
      GITHUB_REPOSITORY: "stevejuma/lapis-ai",
      GITHUB_REF: "refs/heads/main",
      GITHUB_TOKEN: "fixture-private-token",
    },
    async () => ({ ok: false, status: 503 }),
  );
  await assert.rejects(client("git/ref/tags/v0.1.1"), {
    message: "GitHub orchestration HTTP 503",
  });
});
