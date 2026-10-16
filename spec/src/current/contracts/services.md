# Native services and packaging

## Requirements

| ID              | Requirement                                                                                                                                                                                                               |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-LOCAL-001   | The maintained Buzz-derived agent MUST retain provider adapters, permission gates, model discovery, metadata summaries and token-source support with complete Apache attribution and source pins.                         |
| LAI-COMPUTE-001 | Compute MUST retain authenticated loopback inference, explicit model preparation/download/import/start/stop, bounded limits and mutually admitted inference-only peers. Sharing defaults off and startup never downloads. |
| LAI-COMPUTE-002 | Lapis compute MUST preserve the jj-store/compute/1 ALPN, invitation representation and request/response semantics. Mixed-version acceptance MUST use isolated pinned legacy helpers.                                      |
| LAI-PACKAGE-001 | Staged local bundles MUST include production executables, licenses and a TOML manifest with target, versions, hashes and native ABI pins. Test executables, secrets and model weights MUST be excluded.                   |
| LAI-PACKAGE-002 | macOS ARM64 and Linux CPU acceptance MUST separately prove cold native inference and an approved ACP tool operation. Unavailable prerequisites MUST leave their gates pending.                                            |

Initial inference remains non-streaming. No cross-application singleton, public
discovery or automatic cloud fallback is introduced. No consumer migration or
remote publication is included. The Cargo and native-runtime pins remain exact.
