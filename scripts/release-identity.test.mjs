import test from "node:test";
import assert from "node:assert/strict";
import { validateIdentity } from "./release-identity.mjs";
const identity = {
  tag: "v0.1.1",
  version: "0.1.1",
  initial: "0.1.1",
  head: "a".repeat(40),
  tagged: "a".repeat(40),
  bootstrap: false,
  event: "push",
};
test("release requires the exact version tag and checkout", () => {
  assert.doesNotThrow(() => validateIdentity(identity));
  assert.throws(
    () => validateIdentity({ ...identity, tag: "v0.1.2" }),
    /tag does not match/,
  );
  assert.throws(
    () => validateIdentity({ ...identity, tagged: "b".repeat(40) }),
    /checkout differs/,
  );
});
test("bootstrap is manual and restricted to the first admitted version", () => {
  assert.throws(
    () => validateIdentity({ ...identity, bootstrap: true }),
    /explicit dispatch/,
  );
  assert.doesNotThrow(() =>
    validateIdentity({
      ...identity,
      bootstrap: true,
      event: "workflow_dispatch",
    }),
  );
  assert.throws(
    () =>
      validateIdentity({
        ...identity,
        tag: "v0.1.2",
        version: "0.1.2",
        bootstrap: true,
        event: "workflow_dispatch",
      }),
    /initial release/,
  );
});
