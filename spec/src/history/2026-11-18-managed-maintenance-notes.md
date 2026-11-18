# Bounded managed maintenance notes

Revision: enclosing Jujutsu change. LAI-GOV-100/103/105 and LAI-REL-008 retain
source ownership, private coordinator/client, exact Node22/Rust recipe, all
native/model/platform/publisher gates and the existing exact bootstrap tuple.
M0–M4 are unchanged; M5 remains in progress.

## Measured source boundary

Two regressions failed before the fix: initial-bootstrap canonical verification
returned before rejecting managed product release intent, and an ordinary real
SDK0.2.0→0.2.1 candidate admitted an oversized UTF-8 note. The source now checks
reserved managed notes before bootstrap and same-fact returns. Only one exact
12-hex generated path is allowed; malformed names, missing bytes and unexpected
release metadata reject. Regular-file reads reject symlinks, missing/unsafe
paths, invalid UTF-8 and files larger than 8192 bytes. BOM preambles are retained
for rejection. Authored notes outside that reserved namespace keep their owning
release contract; ordinary SDK-transition scope remains strict.

The initial source/metadata/manifest/lock binding is unchanged. Foreign/future
initialization and missing existing policy/slots still fail. There is no new
policy field, manifest/lock change, runtime migration, exception or output path.
The source note reader never writes or normalizes authored bytes.

## Actual source and current-main evidence

The exact remote main `1e6743d0095f83c52b3329782a927437e63b3107` and actual
adoption `16766a1d3435abd4bea36bc630eddfe50faac3a2` were replayed with a frozen
SDK0.2.1 install, official Node22.23.5/pnpm10.34.5, no Deno and no GitHub
credentials. With no notes, CLI2.31.1 status exits1 for changed private packages;
the actual raw macro independently exits0 with unchanged private0.2.0 metadata.
Actual empty-note bytes make the qualified guard exit0/false intent. The
coordinator's measured empty-note flag is a separate operator deployment choice;
source App/writer admission is not waived by raw macro success.

The updated source guard is also exercised at that exact 1e initial baseline,
with the original exact bootstrap input tuple and actual generated empty note,
and with an existing-policy controlled released SDK0.2.0→0.2.1 candidate.
The controlled candidate rejects stale facts, accepts canonical preparation,
actual note writing and two repeated preparations with identical output, then
rejects runtime/null/oversized note metadata and accepts restored exact bytes.
The coordinator implementation is pinned to
`89e69c1a04c508c7ec8a22952742842d67951b5b`; operational receipts stay untracked.
These are source guard and protocol proofs, not public release/native acceptance.

## Validation

- Exact initial targets:0 passed/2 failed before, then2 passed/0 failed.
- Expanded source suite46/46, including actual note bytes, repeats, metadata,
  namespace/count, regular UTF-8 reads and byte rather than code-unit limits.
- Normal governance71/71 plus nested46/46, routing, release metadata,
  architecture/progress and all11 canonical lanes pass. The complete actual
  Node-only base/head gate passes, including renderer check/build,43 renderer
  tests, formatter and actionlint. Existing native/model/platform lanes remain
  distinct and required in hosted qualification.
- The new reader fixture initially omitted its canonical-path import; its exact
  target then1/1 and the complete46/46 pass after correcting that test import.
- The existing whole private Node22 recipe receipt remains unchanged and
  separate from these note-parser and exact-main measurements.

Hosted source checks, ordinary merge and source/coordinator activation remain
separate. The original development advisory tracking and all security/native/
publisher boundaries stay binding without a source exception.
