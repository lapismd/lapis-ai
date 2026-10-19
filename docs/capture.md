# Embedding native source discovery

`lapis-ai-capture` is an independent Rust source library. Select a `SourceProfile`
and supply approved roots, home and optional Codex-home inputs explicitly. No
environment lookup or provider execution occurs.

```rust,ignore
use lapis_ai_capture::{SourceProfile, discovery::*};
use std::path::Path;
let roots = resolve_codex_roots(SourceProfile::CodexCliJsonlV1, &[],
    Some(Path::new("/home/operator")), None)?;
for path in roots {
    if let Some(root) = ApprovedCodexRoot::open(&path)? {
        let mut scan = CodexDirectoryScanner::new(root)?;
        let step = scan.step(128, 8)?;
        // Host admission and format qualification still precede reading.
    }
}
# Ok::<(), lapis_ai_capture::CaptureError>(())
```

Retain the scanner between steps rather than collecting the directory tree.
Candidates expose native paths and a physical file identity; they intentionally
have no serialization or logging representation. Revalidate before and after
reading. Current-binding checks distinguish missing paths from unknown OS
failures, so permission errors cannot justify retiring a source. `revalidate`
is only an availability fence: false may include unknown IO; use
`current_binding` for an absence/rebinding decision.

`cursor` filters Cursor's project transcript layout and skips generated MCP,
canvas and package payloads. `cursor_with_hint_extension` additionally requires
a consumer-chosen sidecar; that hint grants no source or repository authority.
The publishing host owns leases, active-session checks, redaction, durable
cursors and receipts. Profile versions retain the existing product inventory.

Source acquisition is independently reusable beside any agent runtime. It adds
no automatic filesystem capture to an application until that application's
admission and recovery lifecycle is qualified.

For Codex files, `source_file::inspect_codex` opens the regular read-only source,
checks an optional physical identity and returns the exact bounded header bytes,
session ID, directory and file stamp. It requires a complete newline-terminated
header and the selected product version. The result is raw native input, with no
serialization or logging representation. Revalidate the discovery candidate
around inspection, then apply the host privacy boundary before persistence or
renderer delivery. The consumer owns evidence hashes and source cursors.

Codex CLI 0.156.1 files launched by Lapis ACP use the exact `vscode` /
`lapis-acp` header pair and share the CLI profile. This does not admit other
adapter labels or newer Desktop alpha versions. Imported data remains
untrusted source evidence, never execution authority.
