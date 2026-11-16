import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import YAML from "yaml";

const source = path.resolve(".");
const qualificationPath = ".github/version-runtime-qualification.json";
const helper =
  "lapismd/lapis-ci/actions/guard-version-runtime@2f90eb7a035f841f0757d3a2cffc959cbecdfff0";
const digest = (value) => createHash("sha256").update(value).digest("hex");
const canonical = (value) =>
  value && typeof value === "object" && !Array.isArray(value)
    ? Object.fromEntries(
        Object.keys(value)
          .sort()
          .map((key) => [key, canonical(value[key])]),
      )
    : value;

test("qualified runtime guard follows real native bootstrap and precedes App writer authority", () => {
  const workflow = YAML.parse(
    readFileSync(path.join(source, ".github/workflows/changesets.yml"), "utf8"),
  );
  const steps = workflow.jobs.version.steps;
  const guard = steps.find((step) => step.uses === helper);
  assert(guard, "AI must use the exact qualified shared helper");
  assert.deepEqual(guard.with, { "qualification-file": qualificationPath });
  for (const key of ["if", "continue-on-error", "env"])
    assert.equal(guard[key], undefined);
  const bootstrap = steps.find(
    (step) => step.run === "pnpm dev:cargo -- fetch --locked",
  );
  const app = steps.find((step) =>
    step.uses?.startsWith("actions/create-github-app-token@"),
  );
  const writer = steps.find((step) =>
    step.uses?.startsWith("changesets/action@"),
  );
  assert(bootstrap && app && writer);
  assert(steps.indexOf(bootstrap) < steps.indexOf(guard));
  assert(steps.indexOf(guard) < steps.indexOf(app));
  assert(steps.indexOf(app) < steps.indexOf(writer));
  assert.equal(workflow.env.LAPIS_AI_VALIDATION_RUNTIME, "native");
  assert.equal(
    steps.find((step) => step.uses?.startsWith("actions/setup-node@")).with[
      "node-version"
    ],
    22,
  );
  assert.equal(
    steps.find((step) => step.uses?.startsWith("dtolnay/rust-toolchain@")).with
      .toolchain,
    "1.99.0",
  );
  assert.equal(writer.with.version, "pnpm release:version");
  assert.equal(writer.with.createGithubReleases, false);
});

test("AI qualification retains the complete reviewed whole-recipe input binding", () => {
  const qualification = JSON.parse(
    readFileSync(path.join(source, qualificationPath), "utf8"),
  );
  // Audited hosted proof 43814544020, immutable AI source 3e29e16f..., helper 2f90eb7a....
  assert.equal(
    digest(JSON.stringify(canonical(qualification))),
    "ae99c917bcefd0441de564ac204cc2edf5b79bff291d8cf30d0c181d2686a438",
  );
  assert.equal(qualification.nodeMajor, 22);
  assert.equal(qualification.cliVersion, "2.31.1");
  const manifest = JSON.parse(
    readFileSync(path.join(source, "package.json"), "utf8"),
  );
  assert.equal(manifest.packageManager, qualification.packageManager);
  for (const [name, command] of Object.entries(qualification.scripts))
    assert.equal(manifest.scripts[name], command);
  assert.equal(
    digest(readFileSync(path.join(source, ".changeset/config.json"))),
    qualification.configSha256,
  );
  assert.equal(Object.keys(qualification.files).length, 17);
  for (const [file, expected] of Object.entries(qualification.files))
    assert.equal(digest(readFileSync(path.join(source, file))), expected, file);
  assert.equal(
    qualification.scripts["release:version"],
    "node scripts/changeset-version.mjs",
  );
  assert.equal(
    qualification.scripts["release:check"],
    "node scripts/validation.mjs run release:check",
  );
  assert.equal(
    qualification.scripts["dev:cargo"],
    "node scripts/managed-cargo.mjs",
  );
});
