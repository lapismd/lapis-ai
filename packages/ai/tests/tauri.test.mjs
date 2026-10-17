import test from "node:test";
import assert from "node:assert/strict";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createTauriAcpTransport } from "../dist/tauri/index.js";
import { frame, snapshot } from "./support/transport.mjs";

test("public Tauri transport preserves wire casing, frames, exact choices and all scoped commands", async (t) => {
  globalThis.window = { crypto: globalThis.crypto };
  t.after(() => {
    clearMocks();
    delete globalThis.window;
  });
  const calls = [],
    received = [];
  mockIPC((command, args) => {
    calls.push([command, args]);
    if (command === "plugin:lapis-ai|attach") {
      args.onFrame.onmessage({
        sessionId: "worker",
        subscriptionId: "token",
        snapshot: snapshot([frame("activity", "prompt")]),
      });
      return "token";
    }
    return command.endsWith("targets") || command.endsWith("list")
      ? []
      : undefined;
  });
  const transport = createTauriAcpTransport();
  await transport.targets();
  await transport.list();
  await transport.open({
    requestId: "open",
    targetId: "target",
    resumeBinding: "app-binding",
  });
  const choice = {
    type: "permission",
    permission_id: "p",
    option_id: "custom:id",
  };
  await transport.submit("worker", "choice", choice);
  await transport.snapshot("worker", 4);
  assert.equal(
    await transport.attach("worker", 4, (value) => received.push(value)),
    "token",
  );
  await transport.ack("worker", "token", 5);
  await transport.detach("worker", "token");
  await transport.close("worker");
  assert.deepEqual(
    calls.map(([command]) => command),
    [
      "targets",
      "list",
      "open",
      "submit",
      "snapshot",
      "attach",
      "ack",
      "detach",
      "close",
    ].map((command) => `plugin:lapis-ai|${command}`),
  );
  assert.deepEqual(calls[2][1], {
    requestId: "open",
    targetId: "target",
    resumeBinding: "app-binding",
  });
  assert.deepEqual(calls[3][1], {
    sessionId: "worker",
    requestId: "choice",
    request: choice,
  });
  assert.equal(received[0].snapshot.events[0].frame.type, "activity");
  assert.deepEqual(calls[6][1], {
    sessionId: "worker",
    subscriptionId: "token",
    sequence: 5,
  });
});
