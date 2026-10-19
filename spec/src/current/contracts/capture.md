# Native session-source acquisition

## Requirements

| ID              | Requirement                                                                                                                                                                                                                                                                                                              |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| LAI-CAPTURE-001 | The source-acquisition crate MUST discover explicitly selected supported locations through bounded read-only metadata walks, retain physical-root/file identity and distinguish absence from unknown IO. It MUST grant no repository, activity, process or publication authority and import no JJ or application models. |
| LAI-CAPTURE-002 | Qualified Codex file inspection MUST require a complete bounded version-matching header and unchanged regular-file identity across reads. Raw headers and directories MUST remain native-only values; the publishing consumer retains normalization, privacy, source cursors and evidence hashes.                        |

`lapis-ai-capture` initially owns bounded Codex header inspection, approved Codex/Cursor filesystem discovery and
the versioned product inventory. Hosts supply explicit roots, home and optional
Codex home values. Resolution never reads environment variables or discovers
configuration implicitly. Source profile registration is not runtime
qualification. Ordinary Git-backed JJ folder automatic capture remains a
consumer qualification gate.

Root resolution admits at most sixteen configured locations. Each scanner
admits eight directory levels, 4096 entries and 256 candidates per step. It skips symlinks and revalidates
physical bindings. Unix device/inode qualification is retained; other platforms
report unavailable rather than inventing identity. Source paths remain native
capabilities without persistence or renderer serialization.

Cursor's project layout skips generated payloads. Consumers may require a
bounded explicit sidecar extension as a discovery hint. Hints are metadata,
never source/activity admission. The neutral crate has no JJ-specific sidecar
name. JJ normalization, source leases, cursor acknowledgements, sanitized spools
and original publication receipts remain in the publishing consumer.

The Codex CLI 0.156.1 profile also accepts the exact native Lapis ACP pair
`source=vscode` and `originator=lapis-acp`. This is the same pinned CLI record
format launched through the owned ACP adapter; it grants no runtime authority.
Other adapter labels and versions remain excluded. In particular, observing a
new Desktop alpha header does not qualify its body parser. Existing physical
identity, complete-header, privacy and consumer-lifecycle gates are unchanged.
