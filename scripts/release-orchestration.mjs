import { appendFileSync } from "node:fs";
import { resolve } from "node:path";
import { releaseState } from "./release-state.mjs";

export function githubClient(env = process.env, fetcher = fetch) {
  if (
    env.GITHUB_REPOSITORY !== "stevejuma/lapis-ai" ||
    env.GITHUB_REF !== "refs/heads/main"
  )
    throw Error("release orchestration requires canonical main");
  return async (path, method = "GET", body) => {
    const response = await fetcher(
      `https://api.github.com/repos/stevejuma/lapis-ai/${path}`,
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
      inputs: { tag: state.tag, publish_crates: true },
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
    pull.data.head?.repo?.full_name !== "stevejuma/lapis-ai"
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

if (
  process.argv[1] &&
  resolve(process.argv[1]) === resolve(new URL(import.meta.url).pathname)
) {
  const request = githubClient();
  const mode = process.argv[2];
  if (mode === "checks")
    await dispatchVersionChecks(process.env.VERSION_PR, request);
  else {
    const state = await releaseState();
    const result =
      mode === "tag"
        ? await publishTag(state, process.env.GITHUB_SHA, request)
        : mode === "plan"
          ? await planTag(state, request)
          : null;
    if (!result) throw Error("expected plan, tag or checks");
    console.log(result);
    if (process.env.GITHUB_OUTPUT && mode === "plan")
      appendFileSync(
        process.env.GITHUB_OUTPUT,
        `candidate=${result.candidate}\n`,
      );
  }
}
