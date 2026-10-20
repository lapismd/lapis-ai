---
"@lapismd/lapis-ai": minor
---

Add typed HTTP MCP declarations for native host-owned Cursor tools. Existing
stdio builders and wire fields are retained; session/runner tool vectors now
accept McpServerConfig with explicit From conversion. Trusted launch files accept
bounded literal-loopback HTTP endpoints and redacted headers, with environment
references for authored authentication values. Parsing performs no network IO.
