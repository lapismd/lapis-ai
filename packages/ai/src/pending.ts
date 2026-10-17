import { AcpError } from "./errors.js";
import type { ControlFrame, SessionRequest } from "./protocol.js";
interface Pending {
  kind: SessionRequest["type"];
  timer: ReturnType<typeof setTimeout>;
  resolve(frame: ControlFrame): void;
  reject(error: AcpError): void;
}
export class Requests {
  private readonly pending = new Map<string, Pending>();
  private readonly seen = new Set<string>();
  constructor(private readonly expired: (error: AcpError) => void) {}
  register(
    id: string,
    request: SessionRequest,
    timeout: number,
  ): Promise<ControlFrame> {
    if (this.seen.has(id)) throw new AcpError("duplicate_request", "rejected");
    if (this.seen.size >= 4000 || this.pending.size >= 32)
      throw new AcpError("request_limit", "rejected");
    this.seen.add(id);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => this.expired(new AcpError("response_unknown")),
        timeout,
      );
      this.pending.set(id, { kind: request.type, timer, resolve, reject });
    });
  }
  reject(id: string, error: AcpError): void {
    const pending = this.take(id);
    pending?.reject(error);
  }
  receive(frame: ControlFrame): void {
    const pending = this.pending.get(frame.requestId);
    if (!pending) return; // Reconnect can deliver work owned by a previous renderer.
    const terminal =
      frame.type === "error" ||
      {
        configure: "configured",
        prompt: "record",
        permission: "ack",
        cancel: "ack",
        status: "status",
      }[pending.kind] === frame.type;
    if (terminal) this.take(frame.requestId)?.resolve(frame);
  }
  fail(error: AcpError): void {
    for (const id of this.pending.keys()) this.reject(id, error);
  }
  private take(id: string): Pending | undefined {
    const value = this.pending.get(id);
    if (value) {
      clearTimeout(value.timer);
      this.pending.delete(id);
    }
    return value;
  }
}
