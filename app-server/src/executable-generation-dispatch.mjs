function text(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

const SYNTHETIC_APP_SERVER_RESPONSE = "work-engine.synthetic-app-server-response.v1";

function notification(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)
      || typeof value.method !== "string" || value.method.length === 0
      || Object.keys(value).some((key) => !["method", "params"].includes(key))) {
    throw new TypeError("generation response notification is invalid");
  }
  return value;
}

function syntheticResponse(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)
      || value.protocol !== SYNTHETIC_APP_SERVER_RESPONSE
      || !Object.hasOwn(value, "response") || !Array.isArray(value.notifications)
      || Object.keys(value).some((key) => !["notifications", "protocol", "response"].includes(key))) {
    return null;
  }
  value.notifications.forEach(notification);
  return value;
}

function decision(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)
      || !["forward", "respond", "control"].includes(value.disposition)) {
    throw new TypeError("generation dispatch must return forward, respond, or control");
  }
  const keys = Object.keys(value).sort();
  const allowed = value.disposition === "forward"
    ? new Set(["disposition", "payload"])
    : value.disposition === "respond"
      ? new Set(["disposition", "notifications", "result"])
      : new Set(["control", "disposition"]);
  if (keys.some((key) => !allowed.has(key))
      || (value.disposition === "respond" && !keys.includes("result"))
      || (value.disposition === "respond" && "notifications" in value
        && (!Array.isArray(value.notifications)
          || value.notifications.some((item) => {
            try {
              notification(item);
              return false;
            } catch {
              return true;
            }
          })))
      || (value.disposition === "control"
        && !["environment.status", "environment.reload"].includes(value.control))) {
    throw new TypeError("generation dispatch returned unsupported fields");
  }
  return value;
}

function toolResponse(value) {
  return {
    success: true,
    contentItems: [{ type: "inputText", text: JSON.stringify(value) }],
  };
}

export class ExecutableGenerationDispatchHost {
  constructor(manager) {
    if (!manager || typeof manager.runAdmission !== "function") {
      throw new TypeError("generation dispatch host requires an executable generation manager");
    }
    this.manager = manager;
  }

  async #dispatch(generation, { operation, payload, requestedByTurnId = null }, forward) {
    if (typeof generation.dispatch !== "function") {
      throw new TypeError("active executable generation cannot dispatch operations");
    }
    const result = decision(await generation.dispatch(
      text(operation, "generation dispatch operation"),
      structuredClone(payload),
      (effectPayload) => forward(generation, structuredClone(effectPayload)),
    ));
    if (result.disposition === "respond") {
      if (!("notifications" in result)) return structuredClone(result.result);
      return {
        protocol: SYNTHETIC_APP_SERVER_RESPONSE,
        response: structuredClone(result.result),
        notifications: structuredClone(result.notifications),
      };
    }
    if (result.disposition === "control") {
      if (result.control === "environment.status") {
        return toolResponse(this.manager.snapshot());
      }
      const staged = await this.manager.requestReload({
        requestedByTurnId: text(requestedByTurnId, "reload requesting turn id"),
        source: { source: "configured_executable_inventory" },
      });
      return toolResponse({
        status: staged.status,
        reloadId: staged.reloadId,
        activation: staged.activation,
      });
    }
    return forward(generation, result.payload === undefined
      ? structuredClone(payload)
      : structuredClone(result.payload));
  }

  run({ kind, id, operation, payload }, forward) {
    if (typeof forward !== "function") {
      throw new TypeError("generation dispatch requires a stable forward effect");
    }
    return this.manager.runAdmission({ kind, id }, (generation) =>
      this.#dispatch(generation, { operation, payload }, forward)
    );
  }

  runInAdmission({ kind, subjectId, operation, payload }, forward) {
    if (typeof forward !== "function") {
      throw new TypeError("generation dispatch requires a stable forward effect");
    }
    return this.manager.runInAdmission({ kind, subjectId }, (generation) =>
      this.#dispatch(generation, {
        operation,
        payload,
        requestedByTurnId: subjectId,
      }, forward)
    );
  }

  runCallback({ kind, id, operation, payload }, forward) {
    if (typeof forward !== "function") {
      throw new TypeError("generation callback requires a stable forward effect");
    }
    if (this.manager.snapshot().admissions.length > 0) {
      return this.manager.runDuringActiveAdmissions((generation) =>
        this.#dispatch(generation, { operation, payload }, forward)
      );
    }
    return this.run({ kind, id, operation, payload }, forward);
  }

  snapshot() {
    return this.manager.snapshot();
  }
}

export class GenerationBoundAppServerTransport {
  constructor({ transport, dispatchHost, idFactory = null }) {
    if (!transport || typeof transport.request !== "function"
        || typeof transport.notify !== "function") {
      throw new TypeError("generation-bound App Server transport requires a delegate transport");
    }
    if (!dispatchHost || typeof dispatchHost.run !== "function") {
      throw new TypeError("generation-bound App Server transport requires a dispatch host");
    }
    this.transport = transport;
    this.dispatchHost = dispatchHost;
    this.sequence = 0;
    this.dispatchTail = Promise.resolve();
    this.idFactory = idFactory ?? ((kind) => `${kind}-${++this.sequence}`);
    this.serverRequestHandler = null;
    this.notificationHandlers = new Set();
    this.lifecycleErrorHandlers = new Set();
    this.turnAdmissions = new Map();
    this.completedBeforeAdmission = new Set();
    transport.onServerRequest((request) => this.#handleServerRequest(request));
    transport.onNotification((notification) => {
      this.#handleNotification(notification).catch((error) => {
        for (const handler of this.lifecycleErrorHandlers) handler(error);
      });
    });
  }

  onServerRequest(handler) {
    this.serverRequestHandler = handler;
  }
  onNotification(handler) {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }
  onLifecycleError(handler) {
    this.lifecycleErrorHandlers.add(handler);
    return () => this.lifecycleErrorHandlers.delete(handler);
  }
  onClosed(handler) { return this.transport.onClosed?.(handler); }

  async #handleServerRequest(request) {
    const turnId = request?.params?.turnId;
    if (typeof turnId === "string" && this.turnAdmissions.has(turnId)) {
      return this.dispatchHost.runInAdmission({
        kind: "turn",
        subjectId: turnId,
        operation: "app_server.server_request",
        payload: request,
      }, (_generation, forwardedRequest) => {
        if (!this.serverRequestHandler) throw new Error("no App Server client request handler");
        return this.serverRequestHandler(forwardedRequest);
      });
    }
    return this.dispatchHost.runCallback({
      kind: "app_server_callback",
      id: text(this.idFactory("server-request"), "generation callback admission id"),
      operation: "app_server.server_request",
      payload: request,
    }, (_generation, forwardedRequest) => {
      if (!this.serverRequestHandler) throw new Error("no App Server client request handler");
      return this.serverRequestHandler(forwardedRequest);
    });
  }

  async #handleNotification(notification) {
    let reloadCompletion = null;
    let forwardNotification = false;
    let generationResult = null;
    if (notification?.method === "turn/completed") {
      const turnId = notification.params?.turn?.id;
      if (typeof turnId === "string") {
        const admission = this.turnAdmissions.get(turnId);
        if (admission) {
          generationResult = await this.dispatchHost.runInAdmission({
            kind: "turn",
            subjectId: turnId,
            operation: "app_server.backend_notification",
            payload: notification,
          }, () => { forwardNotification = true; });
          this.turnAdmissions.delete(turnId);
          reloadCompletion = this.dispatchHost.manager.closeAdmission(admission);
        } else {
          this.completedBeforeAdmission.add(turnId);
          if (this.completedBeforeAdmission.size > 256) {
            this.completedBeforeAdmission.delete(this.completedBeforeAdmission.values().next().value);
          }
        }
      }
    }
    if (!reloadCompletion && !forwardNotification) {
      generationResult = await this.dispatchHost.runCallback({
        kind: "app_server_callback",
        id: text(this.idFactory("notification"), "generation callback admission id"),
        operation: "app_server.backend_notification",
        payload: notification,
      }, () => { forwardNotification = true; });
    }
    if (reloadCompletion) await reloadCompletion;
    if (forwardNotification) {
      for (const handler of this.notificationHandlers) handler(notification);
    }
    const synthetic = syntheticResponse(generationResult);
    if (synthetic) this.#scheduleSyntheticNotifications(synthetic.notifications);
  }

  async #retainTurn(response) {
    const turnId = response?.turn?.id;
    if (typeof turnId !== "string" || turnId.length === 0) {
      throw new TypeError("turn/start response requires a turn id");
    }
    if (["completed", "interrupted", "failed"].includes(response.turn.status)) return response;
    const admission = this.dispatchHost.manager.openAdmission({
      kind: "turn",
      id: `provider-turn:${turnId}`,
      subjectId: turnId,
    });
    try {
      await this.dispatchHost.manager.markTurnAdmissionExercised(admission);
      if (this.completedBeforeAdmission.delete(turnId)) {
        this.dispatchHost.manager.closeAdmission(admission);
      } else {
        this.turnAdmissions.set(turnId, admission);
      }
    } catch (error) {
      this.dispatchHost.manager.closeAdmission(admission);
      throw error;
    }
    return response;
  }

  #scheduleSyntheticNotifications(notifications) {
    setImmediate(async () => {
      try {
        for (const item of notifications) await this.#handleNotification(item);
      } catch (error) {
        for (const handler of this.lifecycleErrorHandlers) handler(error);
      }
    });
  }

  #schedule(operation) {
    const slot = this.dispatchTail.then(() => {
      let sent = false;
      let markSent;
      const sentPromise = new Promise((resolve) => { markSent = resolve; });
      const release = () => {
        if (sent) return;
        sent = true;
        markSent();
      };
      let completion;
      try {
        completion = Promise.resolve(operation(release));
      } catch (error) {
        completion = Promise.reject(error);
      }
      completion.then(release, release);
      return { completion, sent: sentPromise };
    });
    this.dispatchTail = slot.then(({ sent }) => sent, () => {});
    return slot.then(({ completion }) => completion);
  }

  request(method, params) {
    return this.#schedule((markSent) => {
      const completion = this.dispatchHost.run({
        kind: "app_server_request",
        id: text(this.idFactory("request"), "generation request admission id"),
        operation: "app_server.request",
        payload: { method, params },
      }, (_generation, forwarded) => {
        const forwardedCompletion = this.transport.request(forwarded.method, forwarded.params);
        markSent();
        return forwardedCompletion;
      });
      return completion.then(async (response) => {
        const synthetic = syntheticResponse(response);
        const projected = synthetic ? synthetic.response : response;
        const retained = method === "turn/start"
          ? await this.#retainTurn(projected)
          : projected;
        if (synthetic) this.#scheduleSyntheticNotifications(synthetic.notifications);
        return retained;
      });
    });
  }

  notify(method, params) {
    return this.#schedule((markSent) => this.dispatchHost.run({
      kind: "app_server_notification",
      id: text(this.idFactory("notification"), "generation notification admission id"),
      operation: "app_server.notification",
      payload: { method, params },
    }, (_generation, forwarded) => {
      const completion = this.transport.notify(forwarded.method, forwarded.params);
      markSent();
      return completion;
    }));
  }

  close() { return this.transport.close?.(); }
}
