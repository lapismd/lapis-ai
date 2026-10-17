export class AcpError extends Error {
  constructor(
    public readonly code: string,
    public readonly outcome: "rejected" | "unknown" = "unknown",
  ) {
    super(code);
    this.name = "AcpError";
  }
}
// These native refusals occur before a request is dispatched. Other failures
// cannot establish whether a prompt or effect reached the worker.
const refusals = new Set([
  "denied",
  "disabled",
  "unknown_session",
  "invalid_request_identity",
  "invalid_configuration",
  "invalid_prompt",
  "invalid_permission",
  "invalid_cancel_target",
  "request_limit",
  "queue_full",
  "session_busy",
  "execution_mode_required",
  "stale_or_invalid_permission",
  "not_in_flight",
  "session_closed",
  "duplicate_request",
  "session_starting",
]);
export function submissionError(error: unknown): AcpError {
  return typeof error === "string" && refusals.has(error)
    ? new AcpError(error, "rejected")
    : new AcpError("submission_unknown");
}
