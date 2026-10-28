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

test("queued contiguous output acknowledges consumed prefixes without one IPC round trip per frame", async () => {
  const f = fixture();
  const firstAck = deferred();
  const consumed = [];
  const ack = f.transport.ack;
  let count = 0;
  f.transport.ack = async (...args) => {
    count += 1;
    if (count === 1) await firstAck.promise;
    assert.equal(
      args[2],
      consumed.at(-1),
      "Only the consumed prefix can be acknowledged",
    );
    return ack(...args);
  };
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume(value) {
      consumed.push(value.sequence);
    },
  });
  try {
    f.emit(snapshot([frame("activity", "old")], 0));
    await until(() => count === 1);
    for (let after = 1; after <= 100; after++)
      f.emit(snapshot([frame("activity", "old")], after));
    assert.equal(consumed.length, 1);
    firstAck.resolve();
    await until(() => session.sequence === 101);
    await until(() =>
      f.calls.some(
        ([name, , , sequence]) => name === "ack" && sequence === 101,
      ),
    );
    assert.deepEqual(
      consumed,
      Array.from({ length: 101 }, (_, index) => index + 1),
    );
    assert.ok(
      count <= 5,
      `Queued output required ${count} serial acknowledgement round trips`,
    );
  } finally {
    firstAck.resolve();
    await session.close();
  }
});

test("a blocked in-flight prefix still counts against the declared buffer limit", async () => {
  const f = fixture(),
    blocked = deferred(),
    failures = [];
  const session = await createAcpClient(f.transport).reconnect(info(), {
    maxBufferedMessages: 4,
    consume: () => blocked.promise,
    onFailure: (error) => failures.push(error),
  });
  for (let after = 0; after < 4; after++)
    f.emit(snapshot([frame("activity", "old")], after));
  await new Promise((resolve) => setTimeout(resolve, 0));
  f.emit(snapshot([frame("activity", "old")], 4));
  assert.equal(failures[0]?.code, "renderer_coverage_lost");
  assert.equal(
    f.calls.some(([name]) => name === "ack"),
    false,
  );
  blocked.resolve();
  await session.close();
});

test("a bad queued frame cannot acknowledge the otherwise consumed prefix", async () => {
  const f = fixture(),
    firstAck = deferred(),
    failures = [],
    consumed = [];
  const ack = f.transport.ack;
  let count = 0;
  f.transport.ack = async (...args) => {
    if (++count === 1) await firstAck.promise;
    return ack(...args);
  };
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume: (value) => consumed.push(value.sequence),
    onFailure: (error) => failures.push(error),
  });
  f.emit(snapshot([frame("activity", "old")], 0));
  await until(() => count === 1);
  for (let after = 1; after < 4; after++)
    f.emit(snapshot([frame("activity", "old")], after));
  f.emit(snapshot([frame("activity", "old")], 4), 0, {
    subscriptionId: "obsolete",
  });
  firstAck.resolve();
  await until(() => failures.length === 1);
  assert.equal(failures[0].code, "subscription_scope_mismatch");
  assert.deepEqual(consumed, [1, 2, 3, 4]);
  assert.deepEqual(
    f.calls.filter(([name]) => name === "ack").map((call) => call[3]),
    [1],
  );
  await session.close();
});

test("consumer reentry retains one ordered pump", async () => {
  const f = fixture(),
    consumed = [];
  let active = 0;
  const session = await createAcpClient(f.transport).reconnect(info(), {
    async consume(value) {
      assert.equal(++active, 1);
      consumed.push(value.sequence);
      if (value.sequence < 64)
        f.emit(snapshot([frame("activity", "old")], value.sequence));
      await Promise.resolve();
      active--;
    },
  });
  f.emit(snapshot([frame("activity", "old")]));
  await until(() => session.sequence === 64);
  assert.deepEqual(
    consumed,
    Array.from({ length: 64 }, (_, index) => index + 1),
  );
  await session.close();
});
