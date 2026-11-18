import assert from "node:assert/strict";
import { readFileSync, realpathSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import test from "node:test";
import {
  validatorFacts,
  verifyValidatorCanonicalIntake,
} from "./check-canonical-intake.mjs";
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
  ),
  { chapters } = JSON.parse(policyBytes);
const doc = (files) =>
  "# Source\n\n<!-- dependency-intake:validator-development:start -->\n\n```json\n" +
  JSON.stringify(validatorFacts(files, policyBytes), null, 2) +
  "\n```\n\n<!-- dependency-intake:validator-development:end -->\n";
function data() {
  const { before, after: files } = structuredClone(fixture);
  return {
    before,
    files,
    policyBytes,
    basePolicyBytes: policyBytes,
    documents: Object.fromEntries(chapters.map((p) => [p, doc(files)])),
    baseDocuments: Object.fromEntries(chapters.map((p) => [p, doc(before)])),
    changedFiles: [
      "package.json",
      "pnpm-workspace.yaml",
      "pnpm-lock.yaml",
      ...chapters,
    ],
  };
}
const path = ".changeset/lapis-dependencies-012345abcdef.md";
test("initial exact-input bootstrap still rejects managed product release intent", () => {
  const d = data();
  delete d.before;
  d.changedFiles.push(path);
  d.files[path] = '---\n"product": patch\n---\n';
  assert.throws(() => verifyValidatorCanonicalIntake(d));
});
test("ordinary actual SDK patch rejects oversized UTF8 managed body", () => {
  const d = data();
  d.changedFiles.push(path);
  d.files[path] = "---\n---\n" + "é".repeat(4096);
  assert.throws(() => verifyValidatorCanonicalIntake(d));
});

const notes = await import("./maintenance-note.mjs");
const { mkdtempSync, mkdirSync, writeFileSync, symlinkSync, rmSync } =
  await import("node:fs");
const { tmpdir } = await import("node:os");
const { join } = await import("node:path");
const generated =
  "---\n---\n\nRefresh first-party Lapis dependencies to the published versions recorded in this pull request.\n";
test("actual generated empty-note bytes survive changed candidate and repeat", () => {
  const d = data();
  d.changedFiles.push(path);
  d.files[path] = generated;
  verifyValidatorCanonicalIntake(d);
  d.before = structuredClone(d.files);
  d.baseDocuments = structuredClone(d.documents);
  d.changedFiles = [path];
  verifyValidatorCanonicalIntake(d);
  assert.equal(d.files[path], generated);
});
for (const [name, text] of [
  ["null", "---\nnull\n---\n"],
  ["array", "---\n[]\n---\n"],
  ["preamble", "Before\n---\n---\n"],
  ["malformed", "---\n{\n---\n"],
  ["BOM", "\ufeff---\n---\n"],
])
  test("bounded maintenance helper rejects " + name, () =>
    assert.throws(() => notes.verifyMaintenanceNotes([path], { [path]: text })),
  );
test("reserved namespace remains narrow and at most one note", () => {
  assert.throws(() =>
    notes.verifyMaintenanceNotes(
      [".changeset/lapis-dependencies-unknown.md"],
      {},
    ),
  );
  assert.throws(() =>
    notes.verifyMaintenanceNotes(
      [path, ".changeset/lapis-dependencies-ffffffffffff.md"],
      { [path]: generated },
    ),
  );
  assert.throws(() => notes.verifyMaintenanceNotes([path], {}));
});
test("regular UTF8 note reader rejects symlinks, malformed bytes, missing and oversized files", (t) => {
  const root = realpathSync(
    mkdtempSync(join(tmpdir(), "source-managed-note-")),
  );
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const file = join(root, "note.md");
  writeFileSync(file, generated);
  assert.equal(notes.readMaintenanceNote(file), generated);
  const link = join(root, "link.md");
  symlinkSync(file, link);
  assert.throws(() => notes.readMaintenanceNote(link));
  assert.throws(() => notes.readMaintenanceNote(join(root, "missing.md")));
  const directory = join(root, "directory.md");
  mkdirSync(directory);
  assert.throws(() => notes.readMaintenanceNote(directory));
  writeFileSync(
    file,
    Buffer.concat([Buffer.from("---\n---\n"), Buffer.from([255])]),
  );
  assert.throws(() => notes.readMaintenanceNote(file));
  writeFileSync(file, "---\n---\n" + "é".repeat(4096));
  assert.throws(() => notes.readMaintenanceNote(file));
});
test("UTF88192 boundary is enforced without counting code units", () => {
  const bytes = "---\n---\n" + "é".repeat(4092);
  assert.equal(Buffer.byteLength(bytes), 8192);
  assert.deepEqual(notes.verifyMaintenanceNotes([path], { [path]: bytes }), [
    path,
  ]);
  assert.throws(() =>
    notes.verifyMaintenanceNotes([path], { [path]: bytes + "é" }),
  );
});
