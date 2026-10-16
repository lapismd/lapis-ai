# lapis-acp-protocol

Bounded Agent Client Protocol framing, negotiation and session payloads.

Part of [lapis-acp](https://github.com/stevejuma/lapis-acp), an Agent Client Protocol
harness. Rust 1.98.1 or newer is required.

```toml
[dependencies]
lapis-acp-protocol = "0.1.0"
```

See the [API reference](https://docs.rs/lapis-acp-protocol) and the
[canonical specification](https://stevejuma.github.io/lapis-acp/).

Control version 3 is experimental; its live-adapter qualification gaps remain
recorded in the specification ledger. The controlling application owns trust,
credentials, prompt policy and durable state.

Apache-2.0. This project derives from `block/buzz`'s `buzz-acp`; see NOTICE.md.
