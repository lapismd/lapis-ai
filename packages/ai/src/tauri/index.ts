import { Channel, invoke } from "@tauri-apps/api/core";
import { createAcpClient } from "../client.js";
import type { AcpTransport, StreamMessage } from "../protocol.js";
/** The host must register tauri-plugin-lapis-ai with an explicit capability. */
export function createTauriAcpTransport(): AcpTransport {
  const call = <T>(command: string, args?: Record<string, unknown>) =>
    invoke<T>(`plugin:lapis-ai|${command}`, args);
  return {
    targets: () => call("targets"),
    list: () => call("list"),
    open: (options) => call("open", { ...options }),
    submit: (sessionId, requestId, request) =>
      call("submit", { sessionId, requestId, request }),
    snapshot: (sessionId, after) => call("snapshot", { sessionId, after }),
    attach: (sessionId, after, receive) =>
      call("attach", {
        sessionId,
        after,
        onFrame: new Channel<StreamMessage>(receive),
      }),
    detach: (sessionId, subscriptionId) =>
      call("detach", { sessionId, subscriptionId }),
    ack: (sessionId, subscriptionId, sequence) =>
      call("ack", { sessionId, subscriptionId, sequence }),
    close: (sessionId) => call("close", { sessionId }),
  };
}
export function createTauriAcpClient() {
  return createAcpClient(createTauriAcpTransport());
}
