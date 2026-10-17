import { AcpError, submissionError } from "./errors.js";
import { Requests } from "./pending.js";
import { AcpStream } from "./stream.js";
import type {
  AcpTransport,
  ControlFrame,
  SessionInfo,
  SessionOptions,
  SessionRequest,
  SessionSnapshot,
} from "./protocol.js";
import { label, sessionInfo } from "./validation.js";

export class AcpSessionClient {
  private stream?: AcpStream;
  private attaching = false;
  private closed = false;
  private failure?: AcpError;
  private cursor: number;
  private readonly requests: Requests;
  constructor(
    private readonly transport: AcpTransport,
    readonly info: SessionInfo,
    private readonly options: SessionOptions,
  ) {
    sessionInfo(info);
    this.cursor = options.after ?? 0;
    if (
      options.responseTimeoutMs !== undefined &&
      (!Number.isSafeInteger(options.responseTimeoutMs) ||
        options.responseTimeoutMs < 1 ||
        options.responseTimeoutMs > 86_700_000)
    ) {
      throw new AcpError("invalid_client_options", "rejected");
    }
    this.requests = new Requests((error) => this.fail(error));
  }
  get sequence(): number {
    return this.stream?.cursor ?? this.cursor;
  }
  async attach(): Promise<void> {
    if (this.closed || this.attaching || this.stream)
      throw new AcpError("client_busy", "rejected");
    this.attaching = true;
    this.failure = undefined;
    try {
      const stream = new AcpStream(
        this.transport,
        this.info.sessionId,
        this.options,
        this.cursor,
        (value) => this.consumed(value),
        (error) => this.fail(error),
      );
      this.stream = stream;
      await stream.attach();
    } catch (error) {
      this.fail(
        error instanceof AcpError ? error : new AcpError("attach_unknown"),
      );
      throw error;
    } finally {
      this.attaching = false;
    }
  }
  /** Correlation exists before native submission; this method never retries. */
  async send(
    requestId: string,
    request: SessionRequest,
  ): Promise<ControlFrame> {
    if (this.closed || !this.stream || this.attaching || this.failure)
      throw new AcpError("client_unavailable", "rejected");
    if (!label(requestId) || requestId.startsWith("native-"))
      throw new AcpError("invalid_request_identity", "rejected");
    const timeout =
      this.options.responseTimeoutMs ??
      (request.type === "prompt" ? request.max_duration_ms + 35_000 : 35_000);
    if (!Number.isSafeInteger(timeout) || timeout < 1 || timeout > 86_700_000)
      throw new AcpError("invalid_request", "rejected");
    const result = this.requests.register(requestId, request, timeout);
    // Attach a rejection handler immediately, before an invoke can synchronously
    // fail or a frame can arrive. The returned promise still carries the outcome.
    const owner = this.stream;
    void Promise.resolve()
      .then(() => {
        if (this.closed || this.failure || this.stream !== owner)
          throw new AcpError("detached");
        return this.transport.submit(this.info.sessionId, requestId, request);
      })
      .catch((error: unknown) =>
        this.requests.reject(requestId, submissionError(error)),
      );
    return result;
  }
  private consumed(value: SessionSnapshot): void {
    for (const event of value.events) this.requests.receive(event.frame);
    if (value.closed || value.failure || value.cleanupFailed)
      this.fail(new AcpError(value.failure ?? "session_closed"));
  }
  private fail(error: AcpError): void {
    if (this.failure) return;
    this.failure = error;
    this.requests.fail(error);
    try {
      this.options.onFailure?.(error);
    } catch {
      /* Observers cannot strand requests. */
    }
  }
  async detach(): Promise<void> {
    const stream = this.stream;
    this.fail(new AcpError("detached"));
    try {
      await stream?.detach();
    } finally {
      this.cursor = stream?.cursor ?? this.cursor;
      this.stream = undefined;
    }
  }
  async close(): Promise<void> {
    this.closed = true;
    let detachError: unknown;
    try {
      await this.detach();
    } catch (error) {
      detachError = error;
    }
    await this.transport.close(this.info.sessionId);
    if (detachError) throw detachError;
  }
}
