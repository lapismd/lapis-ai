import test from "node:test";
import assert from "node:assert/strict";
import { createAcpClient } from "../dist/index.js";
import {
  deferred,
  fixture,
  frame,
  info,
  snapshot,
  until,
} from "./support/transport.mjs";

test("channel frames before attach returns are buffered until token admission", async () => {
  const f = fixture(),
    attach = deferred(),
    consumed = [];
  const base = f.transport.attach;
  f.transport.attach = async (...args) => {
    const token = await base(...args);
    f.emit(snapshot([frame("status", "old")]));
    await attach.promise;
    return token;
  };
  const opening = createAcpClient(f.transport).reconnect(info(), {
    consume(value) {
      consumed.push(value);
    },
  });
  await until(() => f.subscriptions.length === 1);
  assert.equal(consumed.length, 0);
  assert.equal(
    f.calls.some(([call]) => call === "ack"),
    false,
  );
  attach.resolve();
  const session = await opening;
  assert.equal(consumed.length, 1);
  assert.equal(session.sequence, 1);
  await session.close();
});

test("consumption must finish before cursor advancement and acknowledgement", async () => {
  const f = fixture(),
    consume = deferred();
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume: () => consume.promise,
  });
  f.emit(snapshot([frame("activity", "old")]));
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(session.sequence, 0);
  assert.equal(
    f.calls.some(([call]) => call === "ack"),
    false,
  );
  consume.resolve();
  await until(() => f.calls.some(([call]) => call === "ack"));
  assert.equal(session.sequence, 1);
  await session.close();
});

test("consumer failure neither acknowledges nor advances the cursor", async () => {
  const f = fixture(),
    failures = [];
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume() {
      throw Error("storage failed");
    },
    onFailure(error) {
      failures.push(error);
    },
  });
  f.emit(snapshot([frame("record", "old")]));
  await until(() => failures.length === 1);
  assert.equal(failures[0].code, "consumer_failed");
  assert.equal(session.sequence, 0);
  assert.equal(
    f.calls.some(([call]) => call === "ack"),
    false,
  );
  assert.equal(
    f.calls.some(([call]) => call === "close"),
    false,
  );
  await session.close();
});

const malformed = {
  "wrong worker": (value) => ({ ...value, sessionId: "other" }),
  "wrong subscription": (value) => ({ ...value, subscriptionId: "obsolete" }),
  "missing journal event": (value) => ({
    ...value,
    snapshot: { ...value.snapshot, events: [] },
  }),
  "journal gap": (value) => ({
    ...value,
    snapshot: { ...value.snapshot, acknowledged: 1 },
  }),
  "wrong control version": (value) => {
    value.snapshot.events[0].frame.version = 2;
    return value;
  },
  "wrong frame worker": (value) => {
    value.snapshot.events[0].frame.sessionId = "other";
    return value;
  },
  "out of order journal": (value) => {
    value.snapshot.events[0].sequence = 2;
    return value;
  },
  "unsafe cursor": (value) => ({
    ...value,
    snapshot: { ...value.snapshot, sequence: Number.MAX_SAFE_INTEGER + 1 },
  }),
};
for (const [name, corrupt] of Object.entries(malformed)) {
  test(`${name} fails without acknowledging or hiding coverage loss`, async () => {
    const f = fixture(),
      failures = [],
      consumed = [];
    const session = await createAcpClient(f.transport).reconnect(info(), {
      consume(value) {
        consumed.push(value);
      },
      onFailure(error) {
        failures.push(error);
      },
    });
    const sub = f.subscriptions[0];
    sub.receive(
      corrupt({
        sessionId: "worker",
        subscriptionId: sub.token,
        snapshot: snapshot([frame("activity", "old")]),
      }),
    );
    await until(() => failures.length === 1);
    assert.equal(consumed.length, 0);
    assert.equal(
      f.calls.some(([call]) => call === "ack"),
      false,
    );
    await session.close();
  });
}

test("bounded early delivery overflow fails while keeping the worker recoverable", async () => {
  const f = fixture(),
    failures = [];
  const base = f.transport.attach;
  f.transport.attach = async (...args) => {
    const token = await base(...args);
    f.emit(snapshot([frame("activity", "old")]));
    f.emit(snapshot([frame("activity", "old")], 1));
    return token;
  };
  await assert.rejects(
    createAcpClient(f.transport).reconnect(info(), {
      consume() {
        assert.fail("overflow is never consumed");
      },
      maxBufferedMessages: 1,
      onFailure(error) {
        failures.push(error);
      },
    }),
    { code: "renderer_coverage_lost" },
  );
  assert.equal(failures.length, 1);
  assert.equal(
    f.calls.some(([call]) => call === "close"),
    false,
  );
  assert.equal(
    f.calls.some(([call]) => call === "detach"),
    true,
  );
});

test("byte overflow is independently bounded before subscription admission", async () => {
  const f = fixture();
  const base = f.transport.attach;
  f.transport.attach = async (...args) => {
    const token = await base(...args);
    f.emit(snapshot([frame("activity", "old", { content: "x".repeat(2048) })]));
    return token;
  };
  await assert.rejects(
    createAcpClient(f.transport).reconnect(info(), {
      consume() {
        assert.fail();
      },
      maxBufferedBytes: 1024,
    }),
    { code: "renderer_coverage_lost" },
  );
});

test("acknowledgement failure reports uncertainty after successful consumption", async () => {
  const f = fixture(),
    failures = [];
  f.transport.ack = async () => {
    throw Error("lost ack");
  };
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume() {},
    onFailure(error) {
      failures.push(error);
    },
  });
  f.emit(snapshot([frame("activity", "old")]));
  await until(() => failures.length === 1);
  assert.equal(session.sequence, 1);
  assert.equal(failures[0].code, "stream_unknown");
  await session.close();
});

test("lost attach acknowledgement stops renderer delivery without closing the native worker", async () => {
  const f = fixture(),
    failures = [];
  const base = f.transport.attach;
  f.transport.attach = async (...args) => {
    await base(...args);
    f.emit(snapshot([frame("activity", "old")]));
    throw Error("reply lost");
  };
  await assert.rejects(
    createAcpClient(f.transport).reconnect(info(), {
      consume() {
        assert.fail("unadmitted delivery");
      },
      onFailure(error) {
        failures.push(error);
      },
    }),
    { code: "attach_unknown" },
  );
  f.emit(snapshot([frame("activity", "old")], 1));
  assert.equal(failures.length, 1);
  assert.equal(
    f.calls.some(([call]) => call === "close" || call === "ack"),
    false,
  );
});
