export function identifyAutomation(pr, repository) {
  if (
    !repository ||
    pr?.head?.repo?.full_name !== repository ||
    pr?.base?.repo?.full_name !== repository ||
    pr?.base?.ref !== "main" ||
    pr?.user?.type !== "Bot"
  )
    return "";
  if (
    pr.user.login === "dependabot[bot]" &&
    pr.head.ref?.startsWith("dependabot/")
  )
    return "dependabot";
  if (
    pr.user.login === "github-actions[bot]" &&
    pr.head.ref === "changeset-release/main"
  )
    return "changesets";
  return "";
}

export async function selectAutomation({
  eventName,
  event,
  repository,
  head,
  base,
  parent,
  associated,
}) {
  if (eventName === "pull_request") {
    const pr = event.pull_request;
    return pr?.head?.sha === head ? identifyAutomation(pr, repository) : "";
  }
  if (
    !["push", "workflow_dispatch", "workflow_call"].includes(eventName) ||
    base !== parent
  )
    return "";
  for (const pr of await associated()) {
    // Dispatched PR head or exact merged revision; an old associated PR cannot
    // exempt a later human change on main or a release tag at another revision.
    if (
      pr.head?.sha === head ||
      (pr.merged_at && pr.merge_commit_sha === head)
    ) {
      const kind = identifyAutomation(pr, repository);
      if (kind) return kind;
    }
  }
  return "";
}

export async function associatedPullRequests(
  repository,
  head,
  env = process.env,
  request = fetch,
) {
  const prs = [];
  for (let page = 1; ; page++) {
    const response = await request(
      `${env.GITHUB_API_URL || "https://api.github.com"}/repos/${repository}/commits/${head}/pulls?per_page=100&page=${page}`,
      {
        headers: {
          Authorization: `Bearer ${env.GITHUB_TOKEN}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        },
        signal: AbortSignal.timeout(30000),
      },
    );
    if (!response.ok)
      throw Error(`Automation PR lookup HTTP ${response.status}`);
    const batch = await response.json();
    if (!Array.isArray(batch))
      throw Error("Invalid automation PR lookup response");
    prs.push(...batch);
    if (batch.length < 100) return prs;
  }
}

export const automationExempt = (env = process.env) =>
  ["dependabot", "changesets"].includes(env.LAPIS_AI_CHECK_AUTOMATION);
