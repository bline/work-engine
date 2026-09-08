import {
  isSupervisorCampaignHostEffectCandidate,
  SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
} from "./services/slice-campaign/host-effect-runtime.mjs";

function text(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function optionalText(value, label) {
  if (value === undefined || value === null) return null;
  return text(value, label);
}

function controlError(code, message, details = {}) {
  const error = new Error(message);
  error.code = code;
  error.details = Object.freeze({ ...details });
  return error;
}

function turnTargetKey({ threadId, turnId }) {
  return `${threadId}\u0000${turnId}`;
}

const SYNTHETIC_APP_SERVER_RESPONSE = "work-engine.synthetic-app-server-response.v1";
const OPERATOR_PROJECTION_REQUEST = "work-engine.operator-projection-request.v1";

function appServerRequestPayload(method, params, operatorThreadIds) {
  const payload = { method, params };
  const threadId = params?.threadId;
  if (typeof threadId !== "string" || !operatorThreadIds.has(threadId)) return payload;
  return {
    ...payload,
    workEngineRequestContext: {
      protocol: OPERATOR_PROJECTION_REQUEST,
      threadId,
    },
  };
}

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

  async #dispatch(
    generation,
    { operation, payload, requestedByTurnId = null },
    forward,
    effect = forward,
  ) {
    if (typeof generation.dispatch !== "function") {
      throw new TypeError("active executable generation cannot dispatch operations");
    }
    const result = decision(await generation.dispatch(
      text(operation, "generation dispatch operation"),
      structuredClone(payload),
      (effectPayload) => effect(generation, structuredClone(effectPayload)),
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
      const activeTurnAdmissions = this.manager.snapshot().admissions.filter(
        (admission) => admission.kind === "turn",
      );
      const reloadTurnId = requestedByTurnId ?? (
        activeTurnAdmissions.length === 1 ? activeTurnAdmissions[0].subjectId : null
      );
      const staged = await this.manager.requestReload({
        requestedByTurnId: text(reloadTurnId, "reload requesting turn id"),
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

  run({ kind, id, operation, payload }, forward, effect = forward) {
    if (typeof forward !== "function") {
      throw new TypeError("generation dispatch requires a stable forward effect");
    }
    return this.manager.runAdmission({ kind, id }, (generation) =>
      this.#dispatch(generation, { operation, payload }, forward, effect)
    );
  }

  runInAdmission({ kind, subjectId, operation, payload }, forward, effect = forward) {
    if (typeof forward !== "function") {
      throw new TypeError("generation dispatch requires a stable forward effect");
    }
    return this.manager.runInAdmission({ kind, subjectId }, (generation) =>
      this.#dispatch(generation, {
        operation,
        payload,
        requestedByTurnId: subjectId,
      }, forward, effect)
    );
  }

  runCallback({ kind, id, operation, payload }, forward, effect = forward) {
    if (typeof forward !== "function") {
      throw new TypeError("generation callback requires a stable forward effect");
    }
    if (this.manager.snapshot().admissions.length > 0) {
      return this.manager.runDuringActiveAdmissions((generation) =>
        this.#dispatch(generation, { operation, payload }, forward, effect)
      );
    }
    return this.run({ kind, id, operation, payload }, forward, effect);
  }

  snapshot() {
    return this.manager.snapshot();
  }
}

export class GenerationBoundAppServerTransport {
  constructor({
    transport, dispatchHost, idFactory = null, supervisorCampaignHostEffectRuntime = null,
    onInitialization = null,
  }) {
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
    if (supervisorCampaignHostEffectRuntime !== null
        && typeof supervisorCampaignHostEffectRuntime?.dispatch !== "function") {
      throw new TypeError("generation-bound transport requires a supervisor campaign host-effect runtime");
    }
    this.supervisorCampaignHostEffectRuntime = supervisorCampaignHostEffectRuntime;
    if (onInitialization !== null && typeof onInitialization !== "function") {
      throw new TypeError("generation-bound transport initialization observer must be a function");
    }
    this.onInitialization = onInitialization;
    this.serverRequestHandler = null;
    this.notificationHandlers = new Set();
    this.lifecycleErrorHandlers = new Set();
    this.turnAdmissions = new Map();
    this.turnThreads = new Map();
    this.turnControls = new Map();
    this.interruptRequests = new Map();
    this.completedBeforeAdmission = new Set();
    // Operator projection identity belongs to the stable transport. Executable
    // generations are replaceable and must not forget an already-open UI
    // thread when a successor activates.
    this.operatorThreadIds = new Set();
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

  operatorControl() {
    return Object.freeze({
      status: () => this.#controlSnapshot(),
      interrupt: (target = {}) => this.#interrupt(target).then(({ response: _response, ...result }) =>
        Object.freeze(result)),
    });
  }

  async #handleServerRequest(request) {
    const explicitTurnId = request?.params?.turnId;
    const requestThreadId = request?.params?.threadId;
    const turnId = typeof explicitTurnId === "string"
      ? (typeof requestThreadId === "string"
          && this.turnThreads.get(explicitTurnId) === requestThreadId
        ? explicitTurnId
        : null)
      : this.#activeTurnForThread(request?.params?.threadId);
    if (typeof turnId === "string" && this.turnAdmissions.has(turnId)) {
      return this.dispatchHost.runInAdmission({
        kind: "turn",
        subjectId: turnId,
        operation: "app_server.server_request",
        payload: request,
      }, (_generation, forwardedRequest) => {
        if (!this.serverRequestHandler) throw new Error("no App Server client request handler");
        return this.serverRequestHandler(forwardedRequest);
      }, (generation, effectPayload) => this.#performGenerationEffect(generation, effectPayload));
    }
    return this.dispatchHost.runCallback({
      kind: "app_server_callback",
      id: text(this.idFactory("server-request"), "generation callback admission id"),
      operation: "app_server.server_request",
      payload: request,
    }, (_generation, forwardedRequest) => {
      if (!this.serverRequestHandler) throw new Error("no App Server client request handler");
      return this.serverRequestHandler(forwardedRequest);
    }, (generation, effectPayload) => this.#performGenerationEffect(generation, effectPayload));
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
          }, () => { forwardNotification = true; }, (generation, effectPayload) =>
            this.#performGenerationEffect(generation, effectPayload));
          reloadCompletion = this.dispatchHost.manager.closeAdmission(admission);
          this.turnAdmissions.delete(turnId);
          this.turnThreads.delete(turnId);
          const control = this.turnControls.get(turnId);
          this.turnControls.delete(turnId);
          if (control) this.interruptRequests.delete(turnTargetKey(control.target));
        } else {
          this.turnThreads.delete(turnId);
          this.turnControls.delete(turnId);
          let projectedTargetCompleted = false;
          for (const [aliasTurnId, control] of this.turnControls.entries()) {
            if (control.target.turnId === turnId
                && control.target.threadId === notification.params?.threadId) {
              projectedTargetCompleted = true;
              this.turnControls.delete(aliasTurnId);
              this.interruptRequests.delete(turnTargetKey(control.target));
            }
          }
          if (!projectedTargetCompleted) {
            this.completedBeforeAdmission.add(turnId);
            if (this.completedBeforeAdmission.size > 256) {
              this.completedBeforeAdmission.delete(
                this.completedBeforeAdmission.values().next().value,
              );
            }
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
      }, () => { forwardNotification = true; }, (generation, effectPayload) =>
        this.#performGenerationEffect(generation, effectPayload));
    }
    if (reloadCompletion) await reloadCompletion;
    if (forwardNotification) {
      for (const handler of this.notificationHandlers) handler(notification);
    }
    const synthetic = syntheticResponse(generationResult);
    if (synthetic) this.#scheduleSyntheticNotifications(synthetic.notifications);
  }

  #activeTurnForThread(threadId) {
    if (typeof threadId !== "string" || threadId.length === 0) return null;
    const matches = [...this.turnThreads.entries()]
      .filter(([, candidateThreadId]) => candidateThreadId === threadId);
    return matches.length === 1 ? matches[0][0] : null;
  }

  #controlSnapshot() {
    const targets = new Map();
    for (const [admissionTurnId, control] of this.turnControls.entries()) {
      if (!this.turnAdmissions.has(admissionTurnId)) continue;
      const key = turnTargetKey(control.target);
      const current = targets.get(key) ?? {
        threadId: control.target.threadId,
        turnId: control.target.turnId,
        aliases: [],
        interruptRequested: this.interruptRequests.has(key),
      };
      if (!current.aliases.some((alias) => alias.threadId === control.alias.threadId
          && alias.turnId === control.alias.turnId)) {
        current.aliases.push({ ...control.alias });
      }
      targets.set(key, current);
    }
    return Object.freeze({
      schemaVersion: 1,
      generationId: this.dispatchHost.snapshot().activeGeneration?.generationId ?? null,
      activeTurns: Object.freeze([...targets.values()].map((target) => Object.freeze({
        ...target,
        aliases: Object.freeze(target.aliases.map((alias) => Object.freeze(alias))),
      }))),
    });
  }

  #resolveInterruptTarget(input) {
    if (!input || typeof input !== "object" || Array.isArray(input)
        || Object.keys(input).some((key) => !["threadId", "turnId"].includes(key))) {
      throw new TypeError("operator interrupt target must be an object with threadId and/or turnId");
    }
    const threadId = optionalText(input.threadId, "operator interrupt thread id");
    const turnId = optionalText(input.turnId, "operator interrupt turn id");
    let candidates = this.#controlSnapshot().activeTurns;
    if (threadId !== null) {
      candidates = candidates.filter((candidate) => candidate.threadId === threadId
        || candidate.aliases.some((alias) => alias.threadId === threadId));
    }
    if (turnId !== null) {
      candidates = candidates.filter((candidate) => candidate.turnId === turnId
        || candidate.aliases.some((alias) => alias.turnId === turnId));
    }
    if (candidates.length === 0) {
      throw controlError("no_active_turn", "no active interruptible turn matches the target", {
        threadId, turnId,
      });
    }
    if (candidates.length !== 1) {
      throw controlError(
        "ambiguous_active_turn",
        "operator interrupt target matches more than one active turn",
        { threadId, turnId, matches: candidates.length },
      );
    }
    return candidates[0];
  }

  async #interrupt(input) {
    const resolved = this.#resolveInterruptTarget(input);
    const target = { threadId: resolved.threadId, turnId: resolved.turnId };
    const key = turnTargetKey(target);
    const existing = this.interruptRequests.get(key);
    if (existing) return existing;
    const completion = Promise.resolve(this.transport.request("turn/interrupt", target)).then(
      (response) => Object.freeze({
        schemaVersion: 1,
        status: "interrupt_requested",
        target: Object.freeze(target),
        aliases: resolved.aliases,
        response,
      }),
      (error) => {
        this.interruptRequests.delete(key);
        throw error;
      },
    );
    this.interruptRequests.set(key, completion);
    return completion;
  }

  async #retainTurn(response, threadId = null, interruptTarget = null) {
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
        if (typeof threadId === "string" && threadId.length > 0) {
          this.turnThreads.set(turnId, threadId);
          const target = interruptTarget ?? { threadId, turnId };
          if (typeof target.threadId === "string" && target.threadId.length > 0
              && typeof target.turnId === "string" && target.turnId.length > 0) {
            this.turnControls.set(turnId, Object.freeze({
              alias: Object.freeze({ threadId, turnId }),
              target: Object.freeze({ threadId: target.threadId, turnId: target.turnId }),
            }));
          }
        }
      }
    } catch (error) {
      this.dispatchHost.manager.closeAdmission(admission);
      throw error;
    }
    return response;
  }

  async #performGenerationEffect(generation, payload, startedTurns = null) {
    if (payload?.protocol === SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL) {
      if (!this.supervisorCampaignHostEffectRuntime) {
        throw new Error("supervisor campaign host-effect runtime is unavailable");
      }
      return this.supervisorCampaignHostEffectRuntime.dispatch({
        generationId: text(generation?.generationId, "effect generation identity"),
        effect: structuredClone(payload),
      });
    }
    if (isSupervisorCampaignHostEffectCandidate(payload)) {
      throw new TypeError("generation supervisor campaign effect is malformed");
    }
    if (!payload || typeof payload.method !== "string") {
      throw new TypeError("generation App Server effect requires a method");
    }
    const response = await this.transport.request(payload.method, payload.params);
    if (payload.method !== "turn/start") return response;
    if (Array.isArray(startedTurns)) {
      if (!["completed", "interrupted", "failed"].includes(response?.turn?.status)) {
        startedTurns.push({ threadId: payload.params?.threadId, turnId: response?.turn?.id });
      }
      return response;
    }
    return this.#retainTurn(response, payload.params?.threadId);
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
    if (method === "turn/interrupt") {
      return this.#interrupt(params ?? {}).then(({ response }) => response);
    }
    return this.#schedule((markSent) => {
      const startedTurns = [];
      const completion = this.dispatchHost.run({
        kind: "app_server_request",
        id: text(this.idFactory("request"), "generation request admission id"),
        operation: "app_server.request",
        payload: appServerRequestPayload(method, params, this.operatorThreadIds),
      }, (_generation, forwarded) => {
        const forwardedCompletion = this.transport.request(forwarded.method, forwarded.params);
        markSent();
        return forwardedCompletion;
      }, (generation, effectPayload) => {
        const effectCompletion = this.#performGenerationEffect(
          generation,
          effectPayload,
          startedTurns,
        );
        markSent();
        return effectCompletion;
      });
      return completion.then(async (response) => {
        const synthetic = syntheticResponse(response);
        const projected = synthetic ? synthetic.response : response;
        if (method === "initialize" && this.onInitialization) {
          await this.onInitialization(Object.freeze({
            params: structuredClone(params ?? {}),
            response: structuredClone(projected),
          }));
        }
        if (["thread/start", "thread/resume"].includes(method)) {
          const threadId = projected?.thread?.id;
          if (typeof threadId === "string" && threadId.length > 0) {
            this.operatorThreadIds.add(threadId);
          }
        }
        const retained = method === "turn/start"
          ? await this.#retainTurn(
              projected,
              params?.threadId,
              synthetic && startedTurns.length === 1 ? startedTurns[0] : null,
            )
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
