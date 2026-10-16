import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkRelease } from "./release-state.mjs";
import {
  policy,
  validateTag,
  validateReleaseSource,
} from "./release-version.mjs";
export function validateIdentity({
  tag,
  version,
  head,
  tagged,
  bootstrap,
  event,
  initial,
}) {
  validateTag(tag, version);
  if (!/^[a-f0-9]{40}$/.test(head) || head !== tagged)
    throw Error("checkout differs from immutable version tag");
  if (bootstrap && (event !== "workflow_dispatch" || version !== initial))
    throw Error(
      "bootstrap requires explicit dispatch of initial release version",
    );
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const selected = checkRelease();
  validateReleaseSource();
  const git = (...args) =>
    execFileSync("git", args, { encoding: "utf8" }).trim();
  validateIdentity({
    tag: process.env.RELEASE_TAG,
    version: selected,
    head: git("rev-parse", "HEAD"),
    tagged: git("rev-parse", `${process.env.RELEASE_TAG}^{commit}`),
    bootstrap: process.env.RELEASE_BOOTSTRAP === "true",
    event: process.env.RELEASE_EVENT,
    initial: policy().bootstrap_version,
  });
  console.log(`Verified immutable release ${process.env.RELEASE_TAG}`);
}
