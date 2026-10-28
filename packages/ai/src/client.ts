import { AcpSessionClient } from "./session.js";
import type {
  AcpTransport,
  OpenOptions,
  SessionInfo,
  SessionOptions,
} from "./protocol.js";
import { sessionInfo } from "./validation.js";
export function createAcpClient(transport: AcpTransport) {
  return {
    targets: () => transport.targets(),
    list: async () => {
      const sessions = await transport.list();
      for (const info of sessions) sessionInfo(info);
      return sessions;
    },
    /** Open never prompts. A lost reply is recovered with list, never replay. */
    open: async (options: OpenOptions, session: SessionOptions) => {
      const info = await transport.open(options);
      const client = new AcpSessionClient(transport, info, session);
      await client.attach();
      return client;
    },
    reconnect: async (info: SessionInfo, options: SessionOptions) => {
      const client = new AcpSessionClient(transport, info, options);
      await client.attach();
      return client;
    },
  };
}
