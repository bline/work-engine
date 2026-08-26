function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function positiveInteger(value, label) {
  if (!Number.isSafeInteger(value) || value < 1) {
    throw new TypeError(`${label} must be a positive safe integer`);
  }
  return value;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function identifier(value) {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function subjectFrom(value) {
  const params = value?.params ?? value;
  const threadId = identifier(params?.threadId ?? value?.thread?.id);
  const turnId = identifier(params?.turnId ?? params?.turn?.id ?? value?.turn?.id);
  const item = params?.item ?? null;
  return {
    threadId,
    turnId,
    itemId: identifier(item?.id),
    itemType: identifier(item?.type),
  };
}

function metricsFrom(method, value) {
  if (method !== "thread/tokenUsage/updated") return null;
  const usage = value?.params?.tokenUsage;
  const last = usage?.last;
  const metric = (name) => Number.isSafeInteger(last?.[name]) && last[name] >= 0
    ? last[name]
    : null;
  const contextWindow = Number.isSafeInteger(usage?.modelContextWindow)
      && usage.modelContextWindow >= 0
    ? usage.modelContextWindow
    : null;
  return {
    inputTokens: metric("inputTokens"),
    cachedInputTokens: metric("cachedInputTokens"),
    outputTokens: metric("outputTokens"),
    totalTokens: metric("totalTokens"),
    modelContextWindow: contextWindow,
  };
}

function eventLine(event) {
  const parts = [
    event.observedAt,
    `#${event.sequence}`,
    event.kind,
    event.method ?? "transport",
  ];
  if (event.operationId) parts.push(`op=${event.operationId}`);
  if (event.subject.threadId) parts.push(`thread=${event.subject.threadId}`);
  if (event.subject.turnId) parts.push(`turn=${event.subject.turnId}`);
  if (event.subject.itemType) parts.push(`item=${event.subject.itemType}`);
  if (event.durationMs !== null) parts.push(`duration_ms=${event.durationMs}`);
  if (event.metrics && event.metrics.totalTokens !== null) {
    parts.push(`tokens=${event.metrics.totalTokens}/${event.metrics.modelContextWindow ?? "?"}`);
  }
  if (event.failureType) parts.push(`failure=${event.failureType}`);
  return parts.join(" ");
}

export function formatAppServerProtocolEvent(event) {
  if (!event || typeof event !== "object" || Array.isArray(event)) {
    throw new TypeError("App Server protocol event must be an object");
  }
  return eventLine(event);
}

export class ObservableAppServerTransport {
  constructor({
    transport,
    retentionLimit = 512,
    onEvent = null,
    now = () => new Date().toISOString(),
    monotonicNow = () => performance.now(),
  }) {
    if (!transport || typeof transport.request !== "function"
        || typeof transport.notify !== "function"
        || typeof transport.onServerRequest !== "function"
        || typeof transport.onNotification !== "function") {
      throw new TypeError("observable App Server transport requires a compatible transport");
    }
    positiveInteger(retentionLimit, "protocol event retention limit");
    if (onEvent !== null && typeof onEvent !== "function") {
      throw new TypeError("protocol event observer must be a function or null");
    }
    if (typeof now !== "function" || typeof monotonicNow !== "function") {
      throw new TypeError("observable App Server transport clocks must be functions");
    }
    this.transport = transport;
    this.retentionLimit = retentionLimit;
    this.now = now;
    this.monotonicNow = monotonicNow;
    this.sequence = 0;
    this.operationSequence = 0;
    this.events = [];
    this.observationErrors = [];
    this.subscribers = new Set(onEvent ? [onEvent] : []);
    this.notificationHandlers = new Set();
    this.closedHandlers = new Set();
    this.serverRequestHandler = null;
    transport.onServerRequest((request) => this.#handleServerRequest(request));
    transport.onNotification((notification) => this.#handleNotification(notification));
    transport.onClosed?.((error) => this.#handleClosed(error));
  }

  #record(kind, {
    method = null,
    operationId = null,
    value = null,
    durationMs = null,
    failure = null,
  } = {}) {
    const observedAt = text(this.now(), "protocol event timestamp");
    if (Number.isNaN(Date.parse(observedAt))) {
      throw new TypeError("protocol event timestamp must be ISO-compatible");
    }
    this.sequence += 1;
    const event = freeze({
      schemaVersion: 1,
      sequence: this.sequence,
      observedAt,
      kind,
      method,
      operationId,
      subject: subjectFrom(value),
      metrics: metricsFrom(method, value),
      durationMs,
      failureType: failure instanceof Error ? failure.name : failure === null ? null : typeof failure,
    });
    this.events.push(event);
    while (this.events.length > this.retentionLimit) this.events.shift();
    for (const subscriber of this.subscribers) {
      try {
        subscriber(event);
      } catch (error) {
        this.observationErrors.push(freeze({
          eventSequence: event.sequence,
          failureType: error instanceof Error ? error.name : typeof error,
        }));
        while (this.observationErrors.length > this.retentionLimit) {
          this.observationErrors.shift();
        }
      }
    }
    return event;
  }

  onServerRequest(handler) {
    if (typeof handler !== "function") {
      throw new TypeError("App Server request handler must be a function");
    }
    this.serverRequestHandler = handler;
  }

  onNotification(handler) {
    if (typeof handler !== "function") {
      throw new TypeError("App Server notification handler must be a function");
    }
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  onClosed(handler) {
    if (typeof handler !== "function") {
      throw new TypeError("App Server close handler must be a function");
    }
    this.closedHandlers.add(handler);
    return () => this.closedHandlers.delete(handler);
  }

  subscribe(handler) {
    if (typeof handler !== "function") {
      throw new TypeError("protocol event subscriber must be a function");
    }
    this.subscribers.add(handler);
    return () => this.subscribers.delete(handler);
  }

  snapshot() {
    return freeze({
      latestSequence: this.sequence,
      events: [...this.events],
      observationErrors: [...this.observationErrors],
    });
  }

  async request(method, params) {
    text(method, "App Server request method");
    this.operationSequence += 1;
    const operationId = `client-request:${this.operationSequence}`;
    const started = this.monotonicNow();
    this.#record("request_started", { method, operationId, value: params });
    try {
      const result = await this.transport.request(method, params);
      this.#record("request_completed", {
        method,
        operationId,
        value: { ...params, ...result },
        durationMs: Math.max(0, Math.round(this.monotonicNow() - started)),
      });
      return result;
    } catch (error) {
      this.#record("request_failed", {
        method,
        operationId,
        value: params,
        durationMs: Math.max(0, Math.round(this.monotonicNow() - started)),
        failure: error,
      });
      throw error;
    }
  }

  notify(method, params) {
    text(method, "App Server notification method");
    try {
      const result = this.transport.notify(method, params);
      this.#record("notification_sent", { method, value: params });
      return result;
    } catch (error) {
      this.#record("notification_failed", { method, value: params, failure: error });
      throw error;
    }
  }

  close() {
    return this.transport.close?.();
  }

  async #handleServerRequest(request) {
    const method = identifier(request?.method);
    this.operationSequence += 1;
    const hasProtocolId = (typeof request?.id === "string" && request.id.length > 0)
      || Number.isSafeInteger(request?.id);
    const operationId = hasProtocolId
      ? `server-request:${request.id}`
      : `server-request:${this.operationSequence}`;
    const started = this.monotonicNow();
    this.#record("server_request_started", { method, operationId, value: request });
    if (!this.serverRequestHandler) {
      const error = new Error(`unsupported App Server request: ${method ?? "unknown"}`);
      this.#record("server_request_failed", {
        method,
        operationId,
        value: request,
        durationMs: Math.max(0, Math.round(this.monotonicNow() - started)),
        failure: error,
      });
      throw error;
    }
    try {
      const result = await this.serverRequestHandler(request);
      this.#record("server_request_completed", {
        method,
        operationId,
        value: request,
        durationMs: Math.max(0, Math.round(this.monotonicNow() - started)),
      });
      return result;
    } catch (error) {
      this.#record("server_request_failed", {
        method,
        operationId,
        value: request,
        durationMs: Math.max(0, Math.round(this.monotonicNow() - started)),
        failure: error,
      });
      throw error;
    }
  }

  #handleNotification(notification) {
    const method = identifier(notification?.method);
    this.#record("notification_received", { method, value: notification });
    for (const handler of this.notificationHandlers) handler(notification);
  }

  #handleClosed(error) {
    this.#record("transport_closed", { failure: error });
    for (const handler of this.closedHandlers) handler(error);
  }
}
