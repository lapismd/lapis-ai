# Native ACP session client

Private migration package. Build with `pnpm build:client` at the repository root
and consume only its public `dist` exports. No package publication is performed.

```ts
import { createTauriAcpClient } from "@lapismd/ai/tauri";

const client = createTauriAcpClient();
const targets = await client.targets();
const session = await client.open(
  { requestId: crypto.randomUUID(), targetId: targets[0]!.id },
  {
    consume: async (snapshot) => {
      /* Persist/map raw control-v3 events here. */
    },
  },
);
// Configure an admitted execution mode before sending a prompt.
await session.send(crypto.randomUUID(), { type: "configure", mode: "ask" });
await session.send(crypto.randomUUID(), {
  type: "prompt",
  text: "Authored text",
  context: "Separate context",
  max_duration_ms: 60_000,
});
await session.detach(); // Native worker remains. Explicit close ends it.
```

The native host must register `tauri-plugin-lapis-ai`, implement its admission and
launch policy, and explicitly grant `lapis-ai:acp-session` to intended windows.
The root export accepts an injectable `AcpTransport` and imports no Tauri globals.

`send` resolves with the correlated terminal frame, including native `error`
frames. Read that frame's result. Transport failures throw `AcpError` with a
`rejected` or `unknown` outcome; neither is retried. Never automatically replay
an uncertain prompt or tool effect. A lost open reply is recovered through
`list()` and its original request identity, followed by `reconnect()`.

The awaited `consume` callback owns application effects. Only successful
consumption advances the cursor or allows acknowledgement. Persist the consumed
cursor with the effects for reload recovery. `sequence` reports that cursor.
Detach can leave an unknown in-flight request; reconnect reports retained raw
events to the consumer rather than inventing a new prompt or promise.

Use the exact permission request and provider option IDs. A null option cancels;
unknown choices never become implicit approval. Conversation identities, context,
transcripts, durable history and memory remain application-owned.
