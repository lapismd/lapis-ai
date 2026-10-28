export function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
export function info(id = "worker") {
  return {
    sessionId: id,
    requestId: "open-one",
    targetId: "agent",
    facts: {
      version: 3,
      type: "opened",
      requestId: "native-open",
      sessionId: id,
      adapterSessionId: "provider-one",
    },
  };
}
export function frame(type, requestId, extra = {}) {
  return { version: 3, type, requestId, sessionId: "worker", ...extra };
}
export function snapshot(frames = [], after = 0, extra = {}) {
  return {
    events: frames.map((frame, i) => ({ sequence: after + i + 1, frame })),
    sequence: after + frames.length,
    acknowledged: 0,
    closed: false,
    failure: null,
    unresolvedRequests: [],
    cleanupFailed: false,
    ...extra,
  };
}
export function fixture() {
  const calls = [],
    subscriptions = [];
  const transport = {
    async targets() {
      return [{ id: "agent", label: "Agent" }];
    },
    async list() {
      return [info()];
    },
    async open(options) {
      calls.push(["open", options]);
      return info();
    },
    async submit(...args) {
      calls.push(["submit", ...args]);
    },
    async snapshot() {
      return snapshot();
    },
    async attach(id, after, receive) {
      const token = `subscription-${subscriptions.length}`;
      subscriptions.push({ id, after, receive, token });
      calls.push(["attach", id, after]);
      return token;
    },
    async detach(...args) {
      calls.push(["detach", ...args]);
    },
    async ack(...args) {
      calls.push(["ack", ...args]);
    },
    async close(...args) {
      calls.push(["close", ...args]);
    },
  };
  const emit = (value, index = subscriptions.length - 1, extra = {}) => {
    const sub = subscriptions[index];
    sub.receive({
      sessionId: sub.id,
      subscriptionId: sub.token,
      snapshot: value,
      ...extra,
    });
  };
  return { transport, calls, subscriptions, emit };
}
export async function until(predicate) {
  for (let attempt = 0; attempt < 100; attempt++) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 2));
  }
  throw Error("condition did not become true");
}
