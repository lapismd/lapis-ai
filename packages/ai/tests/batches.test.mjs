import test from "node:test";
import assert from "node:assert/strict";
import { createAcpClient, sessionEventFrames } from "../dist/index.js";
import {
  deferred,
  fixture,
  frame,
  info,
  snapshot,
  until,
} from "./support/transport.mjs";

const text = (body, request = "turn", message = "answer") =>
  frame("activity", request, {
    update: {
      sessionUpdate: "agent_message_chunk",
      messageId: message,
      content: { type: "text", text: body },
    },
  });
function group() {
  const value = snapshot([text("middle")]);
  value.events[0].before = [text("first", "prior", "prior-answer")];
  value.events[0].after = [text("last")];
  return value;
}
test("all group frames finish consumption before one native cursor is acknowledged", async () => {
  const f = fixture(),
    held = deferred(),
    seen = [];
  const session = await createAcpClient(f.transport).reconnect(info(), {
    async consume(value) {
      for (const event of value.events)
        for (const item of sessionEventFrames(event)) {
          seen.push(item.update.content.text);
          if (seen.length === 1) await held.promise;
        }
    },
  });
  f.emit(group());
  await until(() => seen.length === 1);
  assert.equal(session.sequence, 0);
  assert.equal(
    f.calls.some(([op]) => op === "ack"),
    false,
  );
  held.resolve();
  await until(() => f.calls.some(([op]) => op === "ack"));
  assert.deepEqual(seen, ["first", "middle", "last"]);
  assert.equal(session.sequence, 1);
  assert.equal(f.calls.filter(([op]) => op === "ack").length, 1);
  assert.equal(f.calls.find(([op]) => op === "ack").at(-1), 1);
  await session.close();
});
test("failed group consumption never acknowledges a partly consumed batch", async () => {
  const f = fixture(),
    failures = [],
    seen = [];
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume(value) {
      for (const event of value.events)
        for (const item of sessionEventFrames(event)) {
          seen.push(item.update.content.text);
          if (seen.length === 2) throw Error("consumer failed");
        }
    },
    onFailure: (error) => failures.push(error),
  });
  f.emit(group());
  await until(() => failures.length === 1);
  assert.equal(failures[0].code, "consumer_failed");
  assert.deepEqual(seen, ["first", "middle"]);
  assert.equal(session.sequence, 0);
  assert.equal(
    f.calls.some(([op]) => op === "ack"),
    false,
  );
  await session.close();
});
const invalid = {
  "tail on non-text content": (value) => {
    value.events[0].frame.update.content = { type: "image", data: "fixture" };
  },
  "foreign prefix worker": (value) => {
    value.events[0].before[0].sessionId = "foreign";
  },
  "permission prefix": (value) => {
    value.events[0].before[0].type = "permission";
  },
  "invented provider sequence": (value) => {
    value.events[0].before[0].sequence = 999;
  },
  "non-array prefix": (value) => {
    value.events[0].before = {};
  },
  "too many prefix frames": (value) => {
    value.events[0].before = Array(17).fill(text("x"));
  },
  "foreign startup-tail request": (value) => {
    value.events[0].after[0].requestId = "other";
  },
  "foreign startup-tail message": (value) => {
    value.events[0].after[0].update.messageId = "other";
  },
  "tail on a terminal frame": (value) => {
    value.events[0].frame = frame("record", "turn");
  },
};
for (const [name, corrupt] of Object.entries(invalid))
  test(`${name} fails before consumer or acknowledgement`, async () => {
    const f = fixture(),
      failures = [],
      seen = [];
    const session = await createAcpClient(f.transport).reconnect(info(), {
      consume: (value) => seen.push(value),
      onFailure: (error) => failures.push(error),
    });
    const value = group();
    corrupt(value);
    f.emit(value);
    await until(() => failures.length === 1);
    assert.equal(failures[0].code, "invalid_session_frame");
    assert.equal(seen.length, 0);
    assert.equal(
      f.calls.some(([op]) => op === "ack"),
      false,
    );
    await session.close();
  });
test("legacy events have one frame and need no batch fields", () => {
  const event = { sequence: 1, frame: frame("status", "old") };
  assert.deepEqual(sessionEventFrames(event), [event.frame]);
});
