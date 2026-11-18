import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { test } from "node:test";
import { parse, stringify } from "yaml";
import {
  validatorFacts,
  verifyValidatorCanonicalIntake,
} from "./check-canonical-intake.mjs";
import { sourceIntakeReader } from "./source-intake-reader.mjs";
const fixture = JSON.parse(
  gunzipSync(
    readFileSync(
      new URL("../fixtures/validator-intake.json.gz", import.meta.url),
    ),
  ),
);
const policyBytes = readFileSync(
  new URL(
    "../../.github/dependency-automation/canonical-intake.json",
    import.meta.url,
  ),
  "utf8",
);
const { chapters } = JSON.parse(policyBytes);
const document = (facts) =>
  "# Source\n\n<!-- dependency-intake:validator-development:start -->\n\n```json\n" +
  JSON.stringify(facts, null, 2) +
  "\n```\n\n<!-- dependency-intake:validator-development:end -->\n";
function input() {
  const before = structuredClone(fixture.before),
    files = structuredClone(fixture.after);
  return {
    before,
    files,
    policyBytes,
    basePolicyBytes: policyBytes,
    changedFiles: [
      ...Object.keys(files).filter((p) => p !== "packages/ai/package.json"),
      ...chapters,
    ],
    baseDocuments: Object.fromEntries(
      chapters.map((p) => [p, document(validatorFacts(before, policyBytes))]),
    ),
    documents: Object.fromEntries(
      chapters.map((p) => [p, document(validatorFacts(files, policyBytes))]),
    ),
  };
}
test("actual released validator patch pair is independently verified", () =>
  assert.equal(verifyValidatorCanonicalIntake(input()).version, "0.2.1"));
for (const [name, mutate] of [
  [
    "stale facts",
    (d) => (d.documents[chapters[0]] = d.baseDocuments[chapters[0]]),
  ],
  ["missing facts", (d) => (d.documents[chapters[0]] = "none")],
  [
    "duplicate markers",
    (d) => (d.documents[chapters[0]] += d.documents[chapters[0]]),
  ],
  ["manual chapter edits", (d) => (d.documents[chapters[0]] += "Manual text")],
  ["changed policy", (d) => (d.basePolicyBytes += " ")],
  ["unexpected source", (d) => d.changedFiles.push("src/index.ts")],
  [
    "unrelated lock drift",
    (d) => (d.files["pnpm-lock.yaml"] += "\nunknownSetting: changed\n"),
  ],
  [
    "public contract drift",
    (d) => {
      const m = JSON.parse(d.files["package.json"]);
      m.peerDependencies = { ...(m.peerDependencies ?? {}), react: "^20" };
      d.files["package.json"] = JSON.stringify(m);
    },
  ],
  [
    "forged integrity",
    (d) =>
      (d.files["pnpm-lock.yaml"] = d.files["pnpm-lock.yaml"].replace(
        validatorFacts(d.files, d.policyBytes).integrity,
        "sha512-" + Buffer.alloc(64, 3).toString("base64"),
      )),
  ],
  [
    "unrelated same-version dependency edge",
    (d) => {
      const lock = parse(d.files["pnpm-lock.yaml"]);
      const [key, row] = Object.entries(lock.snapshots).find(
        ([k, r]) =>
          !k.startsWith("@lapismd/spec-validator@") &&
          r.dependencies &&
          Object.keys(r.dependencies).length,
      );
      const key2 = Object.keys(row.dependencies)[0];
      lock.snapshots[key].dependencies[key2] += "(unreviewed@1.0.0)";
      d.files["pnpm-lock.yaml"] = stringify(lock);
    },
  ],
])
  test("rejects " + name + " without mutation", () => {
    const d = input();
    mutate(d);
    const saved = structuredClone(d);
    assert.throws(() => verifyValidatorCanonicalIntake(d));
    assert.deepEqual(d, saved);
  });
for (const hasJj of [true, false])
  test(
    "source reader uses only read-only " + (hasJj ? "JJ" : "Git") + " ancestry",
    () => {
      const calls = [];
      const read = sourceIntakeReader(
        "/source",
        (binary, args) => {
          calls.push([binary, args]);
          return calls.length === 1 ? "a".repeat(40) + "\n" : "source bytes";
        },
        hasJj,
      );
      assert.equal(read("package.json"), "source bytes");
      assert.equal(calls[0][0], hasJj ? "jj" : "git");
      assert.deepEqual(
        calls[1],
        hasJj
          ? [
              "jj",
              [
                "--ignore-working-copy",
                "file",
                "show",
                "-r",
                "a".repeat(40),
                "--",
                "package.json",
              ],
            ]
          : ["git", ["show", "a".repeat(40) + ":package.json"]],
      );
    },
  );
test("source reader fails on unavailable ancestry", () =>
  assert.throws(() => sourceIntakeReader("/source", () => "", false)));

test("accepts one actual empty managed maintenance note without release intent", () => {
  const d = input();
  const path = ".changeset/lapis-dependencies-f235813775ce.md";
  d.files[path] =
    "---\n---\n\nRefresh first-party Lapis dependencies to the published versions recorded in this pull request.\n";
  d.changedFiles.push(path);
  assert.equal(verifyValidatorCanonicalIntake(d).version, "0.2.1");
});
for (const [name, paths, text] of [
  [
    "nonempty release intent",
    [".changeset/lapis-dependencies-f235813775ce.md"],
    '---\n"@lapismd/lapis-ai": patch\n---\n',
  ],
  [
    "explicit null header",
    [".changeset/lapis-dependencies-f235813775ce.md"],
    "---\nnull\n---\n",
  ],
  [
    "malformed frontmatter",
    [".changeset/lapis-dependencies-f235813775ce.md"],
    "not a Changeset",
  ],
  [
    "multiple maintenance notes",
    [
      ".changeset/lapis-dependencies-f235813775ce.md",
      ".changeset/lapis-dependencies-123456789abc.md",
    ],
    "---\n---\n",
  ],
  ["arbitrary maintenance path", [".changeset/manual.md"], "---\n---\n"],
  [
    "missing managed note",
    [".changeset/lapis-dependencies-f235813775ce.md"],
    undefined,
  ],
])
  test("rejects " + name, () => {
    const d = input();
    for (const path of paths) {
      d.changedFiles.push(path);
      if (text !== undefined) d.files[path] = text;
    }
    const saved = structuredClone(d);
    assert.throws(() => verifyValidatorCanonicalIntake(d));
    assert.deepEqual(d, saved);
  });
