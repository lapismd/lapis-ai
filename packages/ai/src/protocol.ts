/** Data only: executable, environment, directory and credentials remain native. */
export type SessionRequest =
  | {
      type: "configure";
      model?: string | null;
      effort?: string | null;
      mode?: string | null;
    }
  | {
      type: "prompt";
      text: string;
      context?: string | null;
      max_duration_ms: number;
    }
  | { type: "permission"; permission_id: string; option_id: string | null }
  | { type: "cancel"; target: string }
  | { type: "status" };

/** Raw control-v3 data. Application mapping must preserve provider payloads. */
export interface ControlFrame {
  version: 3;
  type: string;
  requestId: string;
  sessionId?: string;
  [key: string]: unknown;
}
export interface SessionEvent {
  sequence: number;
  frame: ControlFrame;
  /** Deferred sanitized text, owned by the same acknowledgement cursor. */
  before?: ControlFrame[];
  /** A facts-triggered flush joined before startup becomes observable. */
  after?: ControlFrame[];
}
export interface SessionSnapshot {
  events: SessionEvent[];
  sequence: number;
  acknowledged: number;
  closed: boolean;
  failure: string | null;
  unresolvedRequests: string[];
  cleanupFailed: boolean;
}
export interface StreamMessage {
  subscriptionId: string;
  sessionId: string;
  snapshot: SessionSnapshot;
}
export interface Target {
  id: string;
  label: string;
}
export interface SessionInfo {
  sessionId: string;
  requestId: string;
  targetId: string;
  facts: ControlFrame;
}
export interface OpenOptions {
  requestId: string;
  targetId: string;
  /** Opaque consumer binding admitted by native policy, never a provider ID. */
  resumeBinding?: string;
}
export interface AcpTransport {
  targets(): Promise<Target[]>;
  list(): Promise<SessionInfo[]>;
  open(options: OpenOptions): Promise<SessionInfo>;
  submit(
    sessionId: string,
    requestId: string,
    request: SessionRequest,
  ): Promise<void>;
  snapshot(sessionId: string, after: number): Promise<SessionSnapshot>;
  attach(
    sessionId: string,
    after: number,
    receive: (message: StreamMessage) => void,
  ): Promise<string>;
  detach(sessionId: string, subscriptionId: string): Promise<void>;
  ack(
    sessionId: string,
    subscriptionId: string,
    sequence: number,
  ): Promise<void>;
  close(sessionId: string): Promise<void>;
}
export interface SessionOptions {
  /** Last cursor whose effects the consumer retained. No cursor is invented. */
  after?: number;
  consume(snapshot: SessionSnapshot): void | Promise<void>;
  onFailure?(error: Error): void;
  maxBufferedMessages?: number;
  maxBufferedBytes?: number;
  /** Native request deadline plus transport allowance, primarily for hosts. */
  responseTimeoutMs?: number;
}
