# lapis-acp-testkit

Deterministic ACP fixtures for integration tests; Python 3 is needed for fixture agents.

Part of [Lapis AI](https://github.com/lapismd/lapis-ai). Rust 1.99.0.
The controlling application owns trust, credentials, workspace admission,
conversation history and permission policy. An uncertain prompt is never retried.

See the [usage guide](https://github.com/lapismd/lapis-ai/blob/main/docs/usage.md),
[specification](https://lapismd.github.io/lapis-ai/) and
[release guide](https://github.com/lapismd/lapis-ai/blob/main/docs/releasing.md).
The repository is preparing its first Cargo publication; consult releases for
available versions. Source builds use `pnpm dev:cargo -- build -p lapis-acp-testkit --locked`.

Apache-2.0. See the included `LICENSE.md` and `NOTICE.md` for attribution.
