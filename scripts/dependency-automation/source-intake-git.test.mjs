import assert from "node:assert/strict";
import { test } from "node:test";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { execFileSync } from "node:child_process";
import { gunzipSync } from "node:zlib";
import { createHash } from "node:crypto";
import { checkValidatorCanonicalIntake } from "./check-canonical-intake.mjs";
import { sourceIntakeReader } from "./source-intake-reader.mjs";
const bytes = readFileSync(
  new URL("../fixtures/validator-clean-git.json.gz", import.meta.url),
);
assert.equal(
  createHash("sha256").update(bytes).digest("hex"),
  "1f1680bad95ea3bf46d1caf529e3e19bb8bb793c099a7ec7e2399e3c6e73e244",
);
const fixture = JSON.parse(gunzipSync(bytes));
function candidate(row) {
  const root = realpathSync(
    mkdtempSync(join(tmpdir(), "lapis-ai-intake-git-")),
  );
  const put = (path, value) => {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), value);
  };
  for (const [path, value] of Object.entries(fixture.after)) put(path, value);
  put(fixture.note, row.note);
  if (row.extra) put("src/unreviewed.ts", "export const unreviewed = true;\n");
  mkdirSync(join(root, ".git/refs"), { recursive: true });
  put(".git/HEAD", row.head + "\n");
  put(
    ".git/config",
    "[core]\nrepositoryformatversion = 0\nbare = false\nfilemode = true\nfsmonitor = false\nhooksPath = /dev/null\n",
  );
  put(".git/index", Buffer.from(row.index, "base64"));
  for (const [path, value] of Object.entries(fixture.objects)) {
    assert.match(path, /^[a-f0-9]{2}\/[a-f0-9]{38}$/);
    put(".git/objects/" + path, Buffer.from(value, "base64"));
  }
  return root;
}
for (const row of fixture.cases)
  test("clean committed Git intake: " + row.name, () => {
    const root = candidate(row);
    try {
      assert.equal(
        execFileSync("git", ["diff", "--no-ext-diff", "HEAD", "--"], {
          cwd: root,
          encoding: "utf8",
        }),
        "",
        "test-owned committed candidate must have a clean working tree",
      );
      if (row.name === "empty")
        assert.equal(checkValidatorCanonicalIntake(root).version, "0.2.1");
      else
        assert.throws(
          () => checkValidatorCanonicalIntake(root),
          /Unexpected.*(?:maintenance|scope)|Unsafe or oversized/,
        );
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
test("one captured Git parent owns both previous bytes and committed paths", () => {
  const root = candidate(fixture.cases[0]);
  try {
    const read = sourceIntakeReader(root);
    assert.equal(read.sourceSha, fixture.base);
    assert.equal(
      JSON.parse(read("package.json")).devDependencies[
        "@lapismd/spec-validator"
      ],
      "0.2.0",
    );
    assert(read.changedFiles().includes(fixture.note));
    assert(read.changedFiles().includes("pnpm-lock.yaml"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
for (const useJj of [false, true])
  test(
    "explicit baseline controls " + (useJj ? "JJ" : "Git") + " changed paths",
    () => {
      const root = realpathSync(
        mkdtempSync(join(tmpdir(), "lapis-ai-intake-reader-")),
      );
      if (useJj) mkdirSync(join(root, ".jj"));
      const parent = "a".repeat(40),
        calls = [];
      try {
        const read = sourceIntakeReader(
          root,
          (binary, args) => {
            calls.push({ binary, args });
            if (args.includes("log") || args.includes("rev-parse"))
              return parent + "\n";
            return "diff --git a/.changeset/lapis-dependencies-f235813775ce.md b/.changeset/lapis-dependencies-f235813775ce.md\n@@ -0,0 +1 @@\n+---\n";
          },
          useJj,
        );
        assert.deepEqual(read.changedFiles(), [fixture.note]);
        assert.equal(calls.length, 2);
        assert.equal(calls[1].binary, useJj ? "jj" : "git");
        assert(calls[1].args.includes(parent));
        assert(calls[1].args.includes(useJj ? "@" : "HEAD"));
      } finally {
        rmSync(root, { recursive: true, force: true });
      }
    },
  );
test("clean Git root without a parent refuses intake ancestry", () => {
  const root = candidate(fixture.cases[0]);
  try {
    writeFileSync(join(root, ".git/HEAD"), fixture.base + "\n");
    assert.throws(() => sourceIntakeReader(root), /HEAD\^1|ancestry/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
