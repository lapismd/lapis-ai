import assert from "node:assert/strict";
import test from "node:test";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { gunzipSync } from "node:zlib";
import { createHash } from "node:crypto";
import { changesFromVcs } from "@lapismd/spec-validator";
import {
  checkValidatorCanonicalIntake,
  validatorFacts,
  verifyValidatorCanonicalIntake,
} from "./check-canonical-intake.mjs";
const bytes = readFileSync(
  new URL("../fixtures/validator-version-consumption.json.gz", import.meta.url),
);
assert.equal(
  createHash("sha256").update(bytes).digest("hex"),
  "7b93c5b11aa52cf3e709b98c2840b94b539f8339677ada42bf80cae437d86a67",
);
const fixture = JSON.parse(gunzipSync(bytes));
const policyPath = ".github/dependency-automation/canonical-intake.json";
const policyBytes = fixture.after[policyPath];
const { chapters } = JSON.parse(policyBytes);
function data() {
  return {
    files: structuredClone(fixture.after),
    before: structuredClone(fixture.before),
    policyBytes,
    basePolicyBytes: policyBytes,
    documents: fixture.after,
    baseDocuments: fixture.before,
    changedFiles: [fixture.note],
    consumedNotes: [fixture.note],
  };
}
function candidate(t) {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "ai-consumed-note-")));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const [path, value] of Object.entries(fixture.after)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), value);
  }
  return root;
}
for (const head of ["HEAD", "@"]) {
  test(`actual Changesets consumption through ${head} diff keeps unchanged facts`, (t) => {
    const root = candidate(t);
    const reader = () => {
      const read = (path) => {
        assert.equal(typeof fixture.before[path], "string");
        return fixture.before[path];
      };
      read.sourceSha = fixture.base;
      read.changedFiles = () =>
        changesFromVcs(
          { base: fixture.base, head },
          root,
          () => fixture.diff,
        ).map((change) => change.path);
      return read;
    };
    assert.deepEqual(
      validatorFacts(fixture.before, policyBytes),
      validatorFacts(fixture.after, policyBytes),
    );
    assert.equal(checkValidatorCanonicalIntake(root, reader).version, "0.2.1");
  });
}
test("unchanged full dependency tuple permits only explicitly absent consumed notes", () => {
  assert.equal(verifyValidatorCanonicalIntake(data()).version, "0.2.1");
  const d = data();
  delete d.consumedNotes;
  assert.throws(() => verifyValidatorCanonicalIntake(d), /maintenance/);
});
test("deleted note cannot bypass exact bootstrap or real SDK transition", () => {
  const d = data();
  delete d.before;
  assert.throws(() => verifyValidatorCanonicalIntake(d), /maintenance/);
  const historical = JSON.parse(
    gunzipSync(
      readFileSync(
        new URL("../fixtures/validator-intake.json.gz", import.meta.url),
      ),
    ),
  );
  d.before = historical.before;
  assert.throws(() => verifyValidatorCanonicalIntake(d), /maintenance/);
});
test("consumed allowance binds the complete fact tuple rather than version alone", () => {
  const d = data();
  d.before["pnpm-lock.yaml"] = d.before["pnpm-lock.yaml"].replace(
    validatorFacts(d.before, policyBytes).integrity,
    "sha512-" + "A".repeat(86) + "==",
  );
  assert.throws(() => verifyValidatorCanonicalIntake(d), /maintenance/);
});
for (const note of [
  "---\nnull\n---\n",
  "---\n[]\n---\n",
  '---\n"@lapismd/lapis-ai": patch\n---\n',
  "---\n{\n---\n",
  "---\n---\n" + "é".repeat(4096),
]) {
  test(
    "present invalid note rejects before unchanged-fact or bootstrap return: " +
      note.slice(0, 20),
    () => {
      const d = data();
      d.consumedNotes = [];
      d.files[fixture.note] = note;
      assert.throws(
        () => verifyValidatorCanonicalIntake(d),
        /maintenance|YAML/,
      );
      delete d.before;
      assert.throws(
        () => verifyValidatorCanonicalIntake(d),
        /maintenance|YAML/,
      );
    },
  );
}
test("consumed inventory rejects malformed, multiple, duplicate or present entries", () => {
  for (const mutate of [
    (d) => d.changedFiles.push(".changeset/lapis-dependencies-arbitrary.md"),
    (d) => d.changedFiles.push(".changeset/lapis-dependencies-ffffffffffff.md"),
    (d) => d.consumedNotes.push(fixture.note),
    (d) =>
      d.consumedNotes.push(".changeset/lapis-dependencies-ffffffffffff.md"),
    (d) => (d.files[fixture.note] = "---\n---\n\nMaintenance.\n"),
  ]) {
    const d = data();
    mutate(d);
    assert.throws(() => verifyValidatorCanonicalIntake(d), /maintenance/);
  }
});
for (const kind of [
  "directory",
  "symlink",
  "dangling",
  "external",
  "non-directory-parent",
]) {
  test(
    "current loader rejects unsafe consumed-note candidate: " + kind,
    (t) => {
      const root = candidate(t),
        path = join(root, fixture.note);
      mkdirSync(dirname(path), { recursive: true });
      if (kind === "directory") mkdirSync(path);
      else if (kind === "non-directory-parent") {
        rmSync(dirname(path), { recursive: true });
        writeFileSync(dirname(path), "not a directory");
      } else {
        const target =
          kind === "external" ? join(root, "external.md") : path + ".target";
        if (kind !== "dangling") writeFileSync(target, "---\n---\n");
        symlinkSync(target, path);
      }
      const reader = () => {
        const read = (p) => fixture.before[p];
        read.sourceSha = fixture.base;
        read.changedFiles = () => [fixture.note];
        return read;
      };
      assert.throws(() => checkValidatorCanonicalIntake(root, reader));
    },
  );
}
