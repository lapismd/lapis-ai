import { appendFileSync } from "node:fs";
import { resolve } from "node:path";
import { releaseState } from "./release-state.mjs";

export const REQUIRED_CI_JOBS = [
  "Plan",
  "Spec",
  "Rust build",
  "Native aarch64-unknown-linux-gnu",
  "Native aarch64-apple-darwin",
  "Canonical checks",
];

export function githubClient(env = process.env, fetcher = fetch) {
  if (
    env.GITHUB_REPOSITORY !== "lapismd/lapis-ai" ||
    env.GITHUB_REF !== "refs/heads/main"
  )
    throw Error("release orchestration requires canonical main");
  return async (path, method = "GET", body) => {
    const response = await fetcher(
      `https://api.github.com/repos/lapismd/lapis-ai/${path}`,
      {
        method,
        headers: {
          authorization: `Bearer ${env.GITHUB_TOKEN}`,
          accept: "application/vnd.github+json",
          "content-type": "application/json",
          "x-github-api-version": "2022-11-28",
        },
        ...(body ? { body: JSON.stringify(body) } : {}),
      },
    );
    if (!response.ok && ![404, 422].includes(response.status))
      throw Error(`GitHub orchestration HTTP ${response.status}`);
    return {
      status: response.status,
      data: response.status === 204 ? null : await response.json(),
    };
  };
}

export async function planTag(state, request) {
  if (state.pending) return { candidate: false, reason: "pending changesets" };
  const existing = await request(`git/ref/tags/${state.tag}`);
  if (existing.status === 200)
    return { candidate: false, reason: "version already tagged" };
  if (existing.status !== 404)
    throw Error("cannot establish release tag absence");
  return { candidate: true, tag: state.tag };
}

export async function publishTag(state, sha, request) {
  if (!/^[0-9a-f]{40}$/.test(sha))
    throw Error("invalid qualified source revision");
  const planned = await planTag(state, request);
  if (!planned.candidate) return planned;
  const created = await request("git/refs", "POST", {
    ref: `refs/tags/${state.tag}`,
    sha,
  });
  if (created.status === 422) {
    const raced = await request(`git/ref/tags/${state.tag}`);
    if (raced.status !== 200 || raced.data.object?.sha !== sha)
      throw Error(
        "release tag already names another source; refusing replacement",
      );
  } else if (created.status !== 201) throw Error("release tag creation failed");
  const dispatch = await request(
    "actions/workflows/release.yml/dispatches",
    "POST",
    {
      ref: state.tag,
      inputs: { tag: state.tag },
    },
  );
  if (dispatch.status !== 204)
    throw Error("tagged release dispatch failed; resume existing tag manually");
  return { dispatched: true, tag: state.tag };
}

export async function dispatchVersionChecks(number, request) {
  if (!/^\d+$/.test(String(number)))
    throw Error("invalid version pull request");
  const pull = await request(`pulls/${number}`);
  if (
    pull.status !== 200 ||
    pull.data.base?.ref !== "main" ||
    pull.data.head?.repo?.full_name !== "lapismd/lapis-ai"
  )
    throw Error("version pull request must belong to canonical main");
  for (const workflow of ["ci.yml", "publish-spec-pages.yml"]) {
    const result = await request(
      `actions/workflows/${workflow}/dispatches`,
      "POST",
      { ref: pull.data.head.ref },
    );
    if (result.status !== 204)
      throw Error("version pull request check dispatch failed");
  }
}

function requireSha(sha) {
  if (!/^[0-9a-f]{40}$/.test(sha))
    throw Error("invalid qualified source revision");
}

export function summarizeCiJobs(jobs) {
  const byName = new Map(jobs.map((job) => [job.name, job]));
  const missing = REQUIRED_CI_JOBS.filter((name) => !byName.has(name));
  if (missing.length) return { state: "incomplete", detail: missing.join(",") };
  const failed = REQUIRED_CI_JOBS.filter((name) => {
    const job = byName.get(name);
    const done = job.status === "completed";
    return (
      done &&
      job.conclusion !== "success" &&
      job.conclusion !== "skipped" &&
      job.conclusion !== "neutral"
    );
  });
  if (failed.length) return { state: "failure", detail: failed.join(",") };
  const pending = REQUIRED_CI_JOBS.filter(
    (name) => byName.get(name).status !== "completed",
  );
  if (pending.length) return { state: "pending", detail: pending.join(",") };
  const requiredOk = REQUIRED_CI_JOBS.every(
    (name) => byName.get(name).conclusion === "success",
  );
  return requiredOk
    ? { state: "success" }
    : { state: "incomplete", detail: "required jobs not successful" };
}

export async function inspectExactSourceCi(sha, request) {
  requireSha(sha);
  const listed = await request(
    `actions/workflows/ci.yml/runs?head_sha=${sha}&per_page=20`,
  );
  if (listed.status !== 200) throw Error("cannot list exact-source CI runs");
  const runs = (listed.data.workflow_runs ?? []).filter(
    (run) => run.event !== "workflow_call",
  );
  if (!runs.length) return { state: "missing" };
  let pending = false;
  let failure = null;
  for (const run of runs) {
    if (
      ["queued", "in_progress", "waiting", "requested"].includes(run.status)
    ) {
      pending = true;
      continue;
    }
    if (run.status !== "completed") continue;
    const jobs = await request(`actions/runs/${run.id}/jobs?per_page=100`);
    if (jobs.status !== 200) throw Error("cannot load exact-source CI jobs");
    const summary = summarizeCiJobs(jobs.data.jobs ?? []);
    if (summary.state === "success") return { state: "success", runId: run.id };
    if (summary.state === "pending") pending = true;
    else if (summary.state === "failure" && !failure)
      failure = { runId: run.id, detail: summary.detail };
  }
  if (pending) return { state: "pending" };
  if (failure)
    return { state: "failure", runId: failure.runId, detail: failure.detail };
  return { state: "missing" };
}

export async function awaitQualifiedCi(
  sha,
  request,
  {
    sleep = (ms) => new Promise((resolveSleep) => setTimeout(resolveSleep, ms)),
    now = Date.now,
    deadlineMs = 10_800_000,
    pollMs = 30_000,
  } = {},
) {
  requireSha(sha);
  let dispatched = false;
  const deadline = now() + deadlineMs;
  while (true) {
    const status = await inspectExactSourceCi(sha, request);
    if (status.state === "success")
      return { qualified: true, runId: status.runId, dispatched };
    if (status.state === "failure")
      throw Error(
        `exact-source CI failed (${status.detail ?? "required jobs"})`,
      );
    if (status.state === "missing" && !dispatched) {
      const tip = await request("git/ref/heads/main");
      if (tip.status !== 200 || tip.data.object?.sha !== sha)
        throw Error("candidate SHA is not main tip; refusing CI dispatch");
      const started = await request(
        "actions/workflows/ci.yml/dispatches",
        "POST",
        { ref: "main" },
      );
      if (started.status !== 204)
        throw Error("exact-source CI dispatch failed");
      dispatched = true;
    }
    if (now() >= deadline) throw Error("timed out waiting for exact-source CI");
    await sleep(pollMs);
  }
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === resolve(new URL(import.meta.url).pathname)
) {
  const request = githubClient();
  const mode = process.argv[2];
  if (mode === "checks")
    await dispatchVersionChecks(process.env.VERSION_PR, request);
  else if (mode === "await-ci") {
    let sha = process.env.GITHUB_SHA;
    if (process.env.RELEASE_TAG) {
      const tag = process.env.RELEASE_TAG.replace(/^refs\/tags\//, "");
      const ref = await request(`git/ref/tags/${tag}`);
      if (ref.status !== 200) throw Error(`release tag ${tag} is missing`);
      sha = ref.data.object?.sha;
      if (ref.data.object?.type === "tag") {
        const annotated = await request(`git/tags/${sha}`);
        if (annotated.status !== 200)
          throw Error(`annotated release tag ${tag} is unreadable`);
        sha = annotated.data.object?.sha;
      }
    }
    const result = await awaitQualifiedCi(sha, request);
    console.log(result);
  } else {
    const state = await releaseState();
    const result =
      mode === "tag"
        ? await publishTag(state, process.env.GITHUB_SHA, request)
        : mode === "plan"
          ? await planTag(state, request)
          : null;
    if (!result) throw Error("expected plan, tag, await-ci or checks");
    console.log(result);
    if (process.env.GITHUB_OUTPUT && mode === "plan")
      appendFileSync(
        process.env.GITHUB_OUTPUT,
        `candidate=${result.candidate}\n`,
      );
  }
}
