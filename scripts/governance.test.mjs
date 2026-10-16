import { test } from "node:test";
import assert from "node:assert/strict";
import { count, errors } from "./architecture.mjs";
test("count excludes comments without excluding executable lines", () => {
  assert.equal(
    count("// ignored\n\nlet x = 1;\n/* comment\nend */\nx += 1;"),
    2,
  );
});
test("owned sources satisfy measured non-growing import limits", () =>
  assert.deepEqual(errors(), []));

test("Done milestones require complete dependencies and actual evidence", async () => {
  const { ledgerErrors } = await import("./progress.mjs");
  const rows = Array.from(
    { length: 6 },
    (_, i) =>
      `| M${i} | ${i === 0 ? "Done" : "Planned"} | ${i === 0 ? "none" : `M${i - 1}`} | gate | pending | next |`,
  ).join("\n");
  assert(
    ledgerErrors(rows, () => undefined).some((error) =>
      error.includes("linked evidence"),
    ),
  );
  const complete = rows.replace(
    "gate | pending",
    "gate | [Evidence](../../history/test.md)",
  );
  assert(
    ledgerErrors(complete, () => "M0").some((error) =>
      error.includes("missing evidence field"),
    ),
  );
  const evidence =
    "M0 Revision: Milestone review: ## Commands ## Results ## Failures ## Gaps Next action: Gate result: passed";
  assert.deepEqual(
    ledgerErrors(complete, () => evidence),
    [],
  );
  const outOfOrder = complete
    .replace("| M1 | Planned |", "| M1 | Done |")
    .replace("| M0 | Done |", "| M0 | Planned |");
  assert(
    ledgerErrors(outOfOrder, () => evidence).some((error) =>
      error.includes("incomplete dependency"),
    ),
  );
});

test("boundary checks reject new packages, network in ACP and sibling imports", async () => {
  const { allowed, boundaryErrors } = await import("./boundaries.mjs");
  const packages = Object.keys(allowed).map((name) => ({
    name,
    dependencies: [],
  }));
  assert.deepEqual(boundaryErrors(packages), []);
  packages
    .find((p) => p.name === "lapis-acp-protocol")
    .dependencies.push({ name: "reqwest", kind: null });
  assert(
    boundaryErrors(packages).some((error) =>
      error.includes("excluded reqwest"),
    ),
  );
  packages
    .find((p) => p.name === "lapis-agent")
    .dependencies.push({
      name: "external",
      kind: "dev",
      path: "/sibling/source",
    });
  assert(
    boundaryErrors(packages).some((error) =>
      error.includes("sibling checkout"),
    ),
  );
  packages.push({ name: "unreviewed-crate", dependencies: [] });
  assert(
    boundaryErrors(packages).some((error) =>
      error.includes("missing boundary declaration"),
    ),
  );
});
