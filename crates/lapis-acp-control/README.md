# lapis-acp-control

Application-facing stdio Control V1, V2 and experimental V3 contracts.

Part of [Lapis AI](https://github.com/stevejuma/lapis-ai). Rust 1.99.0.
The controlling application owns trust, credentials, workspace admission,
conversation history and permission policy. An uncertain prompt is never retried.

See the [usage guide](https://github.com/stevejuma/lapis-ai/blob/main/docs/usage.md),
[specification](https://stevejuma.github.io/lapis-ai/) and
[release guide](https://github.com/stevejuma/lapis-ai/blob/main/docs/releasing.md).
The repository is preparing its first Cargo publication; consult releases for
available versions. Source builds use `pnpm dev:cargo -- build -p lapis-acp-control --locked`.

Apache-2.0. See the included `LICENSE.md` and `NOTICE.md` for attribution.
