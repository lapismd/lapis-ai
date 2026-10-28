import { AcpError } from "./errors.js";
import type { SessionInfo, SessionSnapshot } from "./protocol.js";
import { eventGroupsValid } from "./event-frames.js";
export function cursor(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 0;
}
export function label(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.trim().length > 0 &&
    value.length <= 256 &&
    !/[\u0000-\u001f\u007f-\u009f]/.test(value)
  );
}
export function sessionInfo(info: SessionInfo): void {
  if (
    !info ||
    !label(info.sessionId) ||
    !label(info.requestId) ||
    !label(info.targetId) ||
    info.facts?.version !== 3 ||
    info.facts.type !== "opened" ||
    info.facts.sessionId !== info.sessionId ||
    !label(info.facts.adapterSessionId)
  ) {
    throw new AcpError("invalid_session_info");
  }
}
export function snapshot(
  value: SessionSnapshot,
  after: number,
  sessionId: string,
): void {
  if (
    !value ||
    !cursor(value.sequence) ||
    !cursor(value.acknowledged) ||
    value.acknowledged > after ||
    value.sequence < after ||
    !Array.isArray(value.events) ||
    value.events.length > 4096 ||
    typeof value.closed !== "boolean" ||
    typeof value.cleanupFailed !== "boolean" ||
    !(value.failure === null || label(value.failure)) ||
    !Array.isArray(value.unresolvedRequests) ||
    !value.unresolvedRequests.every(label)
  ) {
    throw new AcpError("event_coverage_gap");
  }
  let next = after;
  for (const event of value.events) {
    const frame = event?.frame;
    if (
      event?.sequence !== ++next ||
      frame?.version !== 3 ||
      !label(frame.type) ||
      !label(frame.requestId) ||
      (frame.sessionId !== undefined && frame.sessionId !== sessionId) ||
      !eventGroupsValid(event, sessionId)
    ) {
      throw new AcpError("invalid_session_frame");
    }
  }
  if (next !== value.sequence) throw new AcpError("event_coverage_gap");
}
