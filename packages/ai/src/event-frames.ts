import type { ControlFrame, SessionEvent } from "./protocol.js";
import { label } from "./validation.js";

/** One native cursor owns the complete ordered group. Retain the array index
 * alongside that cursor when deriving per-frame application provenance. */
export function sessionEventFrames(event: SessionEvent): ControlFrame[] {
  return [...(event.before ?? []), event.frame, ...(event.after ?? [])];
}

function text(frame: ControlFrame, sessionId: string, derived = true): boolean {
  if (
    !frame ||
    frame.version !== 3 ||
    frame.type !== "activity" ||
    !label(frame.requestId) ||
    (frame.sessionId !== undefined && frame.sessionId !== sessionId) ||
    (derived && Object.hasOwn(frame, "sequence"))
  )
    return false;
  const update = frame.update as Record<string, unknown> | undefined;
  const content = update?.content as Record<string, unknown> | undefined;
  return (
    (update?.sessionUpdate === "agent_message_chunk" ||
      update?.sessionUpdate === "agent_thought_chunk") &&
    content?.type === "text" &&
    typeof content.text === "string"
  );
}
export function eventGroupsValid(
  event: SessionEvent,
  sessionId: string,
): boolean {
  for (const frames of [event.before, event.after]) {
    if (
      frames !== undefined &&
      (!Array.isArray(frames) ||
        frames.length > 16 ||
        !frames.every((frame) => text(frame, sessionId)))
    )
      return false;
  }
  for (const tail of event.after ?? []) {
    const main = event.frame;
    const update = main.update as Record<string, unknown> | undefined;
    const tailUpdate = tail.update as Record<string, unknown>;
    if (
      !text(main, sessionId, false) ||
      ["version", "requestId", "sessionId", "adapterSessionId"].some(
        (key) => main[key] !== tail[key],
      ) ||
      update?.sessionUpdate !== tailUpdate.sessionUpdate ||
      update?.messageId !== tailUpdate.messageId
    )
      return false;
  }
  return true;
}
