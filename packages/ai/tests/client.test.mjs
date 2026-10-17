import test from "node:test";
import assert from "node:assert/strict";
import { AcpError, createAcpClient } from "../dist/index.js";
import {
  deferred,
  fixture,
  frame,
  info,
  snapshot,
  until,
} from "./support/transport.mjs";

test("open is lazy and passes an opaque resume binding without prompting", async () => {
  const f = fixture();
  const client = createAcpClient(f.transport);
  assert.deepEqual(await client.targets(), [{ id: "agent", label: "Agent" }]);
  const session = await client.open(
    { requestId: "open-one", targetId: "agent", resumeBinding: "app-binding" },
    { consume() {} },
  );
  assert.deepEqual(f.calls[0], [
    "open",
    { requestId: "open-one", targetId: "agent", resumeBinding: "app-binding" },
  ]);
  assert.equal(
    f.calls.some(([call]) => call === "submit"),
    false,
  );
  await session.close();
});

test("lost open acknowledgement uses list and reconnect, never another open", async () => {
  const f = fixture();
  f.transport.open = async () => {
    f.calls.push(["open"]);
    throw "submission_unknown";
  };
  const client = createAcpClient(f.transport);
  await assert.rejects(
    client.open({ requestId: "open-one", targetId: "agent" }, { consume() {} }),
    (error) => error === "submission_unknown",
  );
  const existing = (await client.list()).find(
    (session) => session.requestId === "open-one",
  );
  const session = await client.reconnect(existing, { consume() {} });
  assert.equal(f.calls.filter(([call]) => call === "open").length, 1);
  await session.close();
});

test("correlation is installed before submit and output can precede invoke resolution", async () => {
  const f = fixture(),
    invoked = deferred();
  const seen = [];
  f.transport.submit = async (id, requestId) => {
    f.emit(
      snapshot([
        frame("queued", requestId),
        frame("record", requestId, { record: { status: "completed" } }),
      ]),
    );
    await invoked.promise;
  };
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume(value) {
      seen.push(value);
    },
  });
  const response = await session.send("prompt", {
    type: "prompt",
    text: "authored",
    context: "injected",
    max_duration_ms: 1000,
  });
  assert.equal(response.type, "record");
  assert.equal(seen.length, 1);
  invoked.reject("late-lost-ack");
  await new Promise((resolve) => setTimeout(resolve, 0));
  await session.close();
});

test("permission options and cancellation cross transport exactly without fallback", async () => {
  const f = fixture();
  let sequence = 0;
  f.transport.submit = async (id, requestId, request) => {
    f.calls.push(["submit", id, requestId, request]);
    f.emit(snapshot([frame("ack", requestId)], sequence++));
  };
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume() {},
  });
  for (const [index, option] of ["provider:custom-allow", null].entries()) {
    const request = {
      type: "permission",
      permission_id: "permission-live",
      option_id: option,
    };
    await session.send(`choice-${index}`, request);
    assert.deepEqual(
      f.calls.find((call) => call[2] === `choice-${index}`)[3],
      request,
    );
  }
  await session.close();
});

test("unknown submission rejects honestly, preserves data fields and never replays", async () => {
  const f = fixture();
  f.transport.submit = async (...args) => {
    f.calls.push(["submit", ...args]);
    throw Error("secret diagnostics");
  };
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume() {},
  });
  const request = {
    type: "prompt",
    text: "authored",
    context: "injected",
    max_duration_ms: 1000,
  };
  await assert.rejects(session.send("once", request), {
    code: "submission_unknown",
    outcome: "unknown",
  });
  await assert.rejects(session.send("once", request), {
    code: "duplicate_request",
    outcome: "rejected",
  });
  assert.deepEqual(
    f.calls.filter(([call]) => call === "submit"),
    [["submit", "worker", "once", request]],
  );
  await session.close();
});

test("definite native admission refusal remains distinct from an unknown outcome", async () => {
  const f = fixture();
  f.transport.submit = async () => {
    throw "session_busy";
  };
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume() {},
  });
  await assert.rejects(session.send("busy", { type: "status" }), {
    code: "session_busy",
    outcome: "rejected",
  });
  await session.close();
});

test("terminal error frames remain raw results instead of fabricated completion", async () => {
  const f = fixture();
  f.transport.submit = async (id, requestId) =>
    f.emit(snapshot([frame("error", requestId, { code: "agent_rejected" })]));
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume() {},
  });
  assert.equal(
    (await session.send("rejected", { type: "status" })).code,
    "agent_rejected",
  );
  await session.close();
});

test("detach ends renderer waits and reconnect retains consumed cursor without closing worker", async () => {
  const f = fixture(),
    consumed = [];
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume(value) {
      consumed.push(value);
    },
  });
  const pending = assert.rejects(session.send("pending", { type: "status" }), {
    code: "detached",
    outcome: "unknown",
  });
  await until(() => f.calls.some(([call]) => call === "submit"));
  await session.detach();
  await pending;
  assert.equal(
    f.calls.some(([call]) => call === "close"),
    false,
  );
  await session.attach();
  f.emit(snapshot([frame("status", "pending")]));
  await until(() => session.sequence === 1);
  assert.equal(consumed.length, 1);
  f.emit(snapshot([frame("status", "stale")]), 0);
  assert.equal(consumed.length, 1);
  await session.detach();
  await session.attach();
  assert.equal(f.subscriptions[2].after, 1);
  await session.close();
});

test("native failure and unresolved work remain observable independently of event delivery", async () => {
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
  const result = assert.rejects(session.send("lost", { type: "status" }), {
    code: "output_coverage_lost",
  });
  f.emit(
    snapshot([], 0, {
      closed: true,
      failure: "output_coverage_lost",
      unresolvedRequests: ["lost"],
      cleanupFailed: true,
    }),
  );
  await result;
  assert.deepEqual(consumed[0].unresolvedRequests, ["lost"]);
  assert.equal(failures.length, 1);
  await session.close();
});

test("response timeout stops pending waits with no prompt replay", async () => {
  const f = fixture();
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume() {},
    responseTimeoutMs: 5,
  });
  await assert.rejects(session.send("lost", { type: "status" }), {
    code: "response_unknown",
    outcome: "unknown",
  });
  assert.equal(f.calls.filter(([call]) => call === "submit").length, 1);
  await session.close();
});

test("a send detached before dispatch cannot launch deferred work", async () => {
  const f = fixture();
  const session = await createAcpClient(f.transport).reconnect(info(), {
    consume() {},
  });
  const result = assert.rejects(
    session.send("fenced", { type: "status" }),
    AcpError,
  );
  await session.detach();
  await result;
  assert.equal(
    f.calls.some(([call]) => call === "submit"),
    false,
  );
  await session.close();
});
