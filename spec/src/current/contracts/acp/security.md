# Security and trust

`lapis-acp` runs an autonomous coding agent with tool access on a local machine.
These requirements are scoped to the ACP crates. It
implements no authentication, holds no keys, signs nothing, opens no listener
and keeps no durable state. Every one of those is the controlling application's
responsibility, and blurring that line would create a second, weaker copy of a
policy that already exists upstream of this process.

The threat model is a trusted controlling application supplying untrusted task
content and an untrusted agent subprocess producing untrusted output.

## Requirements

| ID          | Requirement                                                                                                                                                                         |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LAI-SEC-001 | `lapis-acp` MUST NOT implement authentication, identity, key custody or author gating; the controlling application is the trust boundary.                                           |
| LAI-SEC-002 | Credentials MUST NOT appear in process arguments and MUST NOT be written to a log, a diagnostic or a terminal record.                                                               |
| LAI-SEC-003 | A task document MUST be treated as executable input, and the runner identifier check MUST prevent accidental execution by a different runner without claiming to verify authorship. |
| LAI-SEC-004 | `lapis-acp` MUST NOT modify global Git configuration, install a credential helper or persist a signing key.                                                                         |
| LAI-SEC-005 | A failed, cancelled or uncertain invocation MUST NOT be retried automatically.                                                                                                      |
| LAI-SEC-006 | Agent output, agent error text and agent-supplied paths MUST be treated as untrusted data and MUST NOT be interpreted as instructions or executed.                                  |
| LAI-SEC-007 | Resource bounds MUST be enforced on untrusted input: document size, read interval, frame size, turn duration and reap interval.                                                     |

## What is deliberately absent

The upstream harness authenticates to a relay with a keypair, gates inbound
events by author, resolves an owner, verifies workflow-message provenance
against a relay signing key, signs Git commits and tags, and installs scoped
credential helpers with an ephemeral keyfile. None of that is ported, stubbed or
feature-flagged. A future need for it belongs in the controlling application,
which already has an identity system, rather than in a runner that would have to
invent one.

The consequence is explicit: `lapis-acp` grants an agent whatever the launch
environment grants. A caller that supplies provider credentials in the
environment has given them to the agent. That is the caller's decision and it is
not reversible from inside this process, which is why the task document cannot
carry credentials and why reserved environment keys cannot be set by a runner
definition.

## Sanitization

Terminal records are sanitized by construction rather than by filtering. The
record schema has no field that carries prompt text, agent final text, raw
adapter error text or environment content. A failure carries a stable code from
a fixed set. This is stronger than redaction, because there is no path by which
an unexpected value reaches the record.

Diagnostics on stderr are operator-facing and may name a failing executable, a
protocol violation or a timeout stage. They MUST NOT include credentials or
prompt content. Child stderr is forwarded for diagnosis and is the agent's own
output; a controlling application that treats it as trusted is making an error
ACP cannot prevent, and LAI-SEC-006 states the expectation.

## Bounds

Untrusted input is bounded before it is parsed rather than during. The document
size cap applies to the whole UTF-8 object including whitespace. The read
interval is separate from the execution deadline, so a stalled pipe cannot
consume the turn budget and a stalled agent cannot consume the read budget. The
frame byte cap applies to every line the agent emits, including the first.
