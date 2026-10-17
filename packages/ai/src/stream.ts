import { AcpError } from "./errors.js";
import type {
  AcpTransport,
  SessionOptions,
  SessionSnapshot,
  StreamMessage,
} from "./protocol.js";
import { cursor, label, snapshot } from "./validation.js";

/** A single subscription, including bounded delivery before attach resolves. */
export class AcpStream {
  cursor: number;
  failure?: AcpError;
  private token?: string;
  private stopped = false;
  private queue: { message: StreamMessage; bytes: number }[] = [];
  private bytes = 0;
  private running?: Promise<void>;
  private readonly maxMessages: number;
  private readonly maxBytes: number;
  constructor(
    private readonly transport: AcpTransport,
    private readonly sessionId: string,
    private readonly options: SessionOptions,
    after: number,
    private readonly consumed: (snapshot: SessionSnapshot) => void,
    private readonly failed: (error: AcpError) => void,
  ) {
    this.cursor = after;
    this.maxMessages = options.maxBufferedMessages ?? 256;
    this.maxBytes = options.maxBufferedBytes ?? 16 * 1024 * 1024;
    if (
      !cursor(after) ||
      !Number.isInteger(this.maxMessages) ||
      this.maxMessages < 1 ||
      this.maxMessages > 4096 ||
      !Number.isSafeInteger(this.maxBytes) ||
      this.maxBytes < 1024 ||
      this.maxBytes > 128 * 1024 * 1024
    ) {
      throw new AcpError("invalid_client_options", "rejected");
    }
  }
  async attach(): Promise<void> {
    let token: string;
    try {
      token = await this.transport.attach(
        this.sessionId,
        this.cursor,
        (message) => this.receive(message),
      );
    } catch {
      this.fail(new AcpError("attach_unknown"));
      throw this.failure;
    }
    if (!label(token)) this.fail(new AcpError("invalid_subscription"));
    this.token = token;
    if (this.stopped) {
      await this.transport.detach(this.sessionId, token);
      throw this.failure ?? new AcpError("detached");
    }
    this.kick();
    await this.running;
    if (this.failure) throw this.failure;
  }
  private receive(message: StreamMessage): void {
    if (this.stopped) return;
    let bytes: number;
    try {
      bytes = new TextEncoder().encode(JSON.stringify(message)).byteLength;
    } catch {
      this.fail(new AcpError("invalid_session_frame"));
      return;
    }
    if (
      this.queue.length >= this.maxMessages ||
      bytes > this.maxBytes - this.bytes
    ) {
      this.fail(new AcpError("renderer_coverage_lost"));
      return;
    }
    this.queue.push({ message, bytes });
    this.bytes += bytes;
    this.kick();
  }
  private kick(): void {
    if (this.running || !this.token || this.stopped) return;
    this.running = this.pump().finally(() => {
      this.running = undefined;
      if (this.queue.length) this.kick();
    });
  }
  private async pump(): Promise<void> {
    while (!this.stopped && this.queue.length) {
      const item = this.queue.shift()!;
      try {
        const { message } = item;
        if (
          message.sessionId !== this.sessionId ||
          message.subscriptionId !== this.token
        ) {
          throw new AcpError("subscription_scope_mismatch");
        }
        snapshot(message.snapshot, this.cursor, this.sessionId);
        try {
          await this.options.consume(message.snapshot);
        } catch {
          throw new AcpError("consumer_failed");
        }
        this.cursor = message.snapshot.sequence;
        // Detach fences pending acknowledgements while preserving consumed state.
        if (this.stopped) break;
        await this.transport.ack(this.sessionId, this.token!, this.cursor);
        if (!this.stopped) this.consumed(message.snapshot);
      } catch (error) {
        this.fail(
          error instanceof AcpError ? error : new AcpError("stream_unknown"),
        );
      } finally {
        this.bytes -= item.bytes;
      }
    }
  }
  private fail(error: AcpError): void {
    if (this.stopped) return;
    this.failure = error;
    this.stopped = true;
    this.clear();
    this.failed(error);
    if (this.token)
      void this.transport.detach(this.sessionId, this.token).catch(() => {});
  }
  private clear(): void {
    this.bytes -= this.queue.reduce((total, item) => total + item.bytes, 0);
    this.queue = [];
  }
  async detach(): Promise<void> {
    this.stopped = true;
    this.clear();
    await this.running;
    if (this.token) await this.transport.detach(this.sessionId, this.token);
  }
}
