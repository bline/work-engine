import {
  CapabilityError,
  FOUNDATION_CAPABILITIES,
  PINNED_PROTOCOL,
  PINNED_PROVIDER_RUNTIME,
  assertCompatibleServer,
  negotiateCapabilities,
  negotiateProviderCapabilities,
} from "./capabilities.mjs";
import { createHash } from "node:crypto";
import { open } from "node:fs/promises";
import path from "node:path";
import { compileRequestContextInput } from "./request-context-input.mjs";

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
}

const RETAINED_TURN_COMPLETION_LIMIT = 256;
export const CODEX_THREAD_CONTEXT_SNAPSHOT_SCHEMA_VERSION = 1;
export const CODEX_THREAD_CONTEXT_SNAPSHOT_TYPE = "work-engine.codex-thread-context-snapshot";
export const CODEX_EFFECTIVE_CONTEXT_SNAPSHOT_SCHEMA_VERSION = 1;
export const CODEX_EFFECTIVE_CONTEXT_SNAPSHOT_TYPE = "work-engine.codex-effective-context-snapshot";
const DEFAULT_CODEX_ROLLOUT_SNAPSHOT_MAX_BYTES = 64 * 1024 * 1024;

export class AppServerTurnError extends Error {
  constructor(message, { threadId = null, turnId = null, status = null } = {}) {
    super(message);
    this.name = "AppServerTurnError";
    this.threadId = threadId;
    this.turnId = turnId;
    this.status = status;
  }
}

function turnKey(threadId, turnId) {
  requireText(threadId, "turn completion thread id");
  requireText(turnId, "turn completion turn id");
  return JSON.stringify([threadId, turnId]);
}

function finalOutputFrom(turn) {
  if (!Array.isArray(turn.items)) {
    throw new AppServerTurnError("turn/completed payload does not contain turn items", {
      turnId: turn.id,
      status: turn.status,
    });
  }
  const messages = turn.items.filter((item) => item?.type === "agentMessage");
  const explicitFinals = messages.filter((item) => item.phase === "final_answer");
  const compatibleFinals = messages.filter((item) => item.phase == null);
  const candidates = explicitFinals.length > 0 ? explicitFinals : compatibleFinals;
  const finalMessage = candidates.at(-1) ?? null;
  return finalMessage && typeof finalMessage.text === "string"
    ? finalMessage.text
    : null;
}

function completedTurnOutcome(notification) {
  const threadId = notification?.params?.threadId;
  const turn = notification?.params?.turn;
  const turnId = turn?.id;
  const key = turnKey(threadId, turnId);
  if (!turn || typeof turn !== "object" || Array.isArray(turn)) {
    throw new AppServerTurnError("turn/completed payload does not contain a turn", {
      threadId,
      turnId,
    });
  }
  if (turn.status !== "completed") {
    const detail = typeof turn.error?.message === "string"
      ? `: ${turn.error.message}`
      : "";
    return {
      key,
      error: new AppServerTurnError(`App Server turn ended as ${turn.status}${detail}`, {
        threadId,
        turnId,
        status: turn.status ?? null,
      }),
    };
  }
  return {
    key,
    value: Object.freeze({
      threadId,
      turnId,
      status: turn.status,
      outputText: finalOutputFrom(turn),
      turn,
    }),
  };
}

function threadIdFrom(response, operation) {
  const threadId = response?.thread?.id;
  requireText(threadId, `${operation} response thread id`);
  return threadId;
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(value[key])}`,
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function deepFreeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

export function normalizeCodexThreadContextSnapshot(response, expectedThreadId) {
  requireText(expectedThreadId, "thread context snapshot expected thread id");
  const thread = response?.thread;
  if (!thread || typeof thread !== "object" || Array.isArray(thread)) {
    throw new TypeError("thread/read response does not contain a thread");
  }
  if (thread.id !== expectedThreadId) {
    throw new TypeError("thread/read response does not match the requested thread");
  }
  if (!Array.isArray(thread.turns)) {
    throw new TypeError("thread/read response does not contain hydrated turns");
  }
  const turns = thread.turns.map((turn, index) => {
    if (!turn || typeof turn !== "object" || Array.isArray(turn)) {
      throw new TypeError(`thread context turn ${index} must be an object`);
    }
    requireText(turn.id, `thread context turn ${index} id`);
    requireText(turn.status, `thread context turn ${index} status`);
    if (turn.itemsView !== undefined && turn.itemsView !== "full") {
      throw new TypeError(`thread context turn ${index} is not fully hydrated`);
    }
    if (!Array.isArray(turn.items)) {
      throw new TypeError(`thread context turn ${index} does not contain items`);
    }
    return {
      id: turn.id,
      status: turn.status,
      error: turn.error ?? null,
      items: structuredClone(turn.items),
    };
  });
  const body = {
    schemaVersion: CODEX_THREAD_CONTEXT_SNAPSHOT_SCHEMA_VERSION,
    type: CODEX_THREAD_CONTEXT_SNAPSHOT_TYPE,
    threadId: expectedThreadId,
    turns,
  };
  return deepFreeze({
    ...body,
    contextRevision: `sha256:${createHash("sha256").update(canonicalJson(body)).digest("hex")}`,
  });
}

function rolloutItem(record, index) {
  if (!record || typeof record !== "object" || Array.isArray(record)) {
    throw new TypeError(`Codex rollout record ${index} must be an object`);
  }
  requireText(record.type, `Codex rollout record ${index} type`);
  if (!record.payload || typeof record.payload !== "object" || Array.isArray(record.payload)) {
    throw new TypeError(`Codex rollout record ${index} payload must be an object`);
  }
  return { type: record.type, payload: structuredClone(record.payload) };
}

function compatibleTurnId(activeTurnId, itemTurnId) {
  return activeTurnId == null || itemTurnId == null || activeTurnId === itemTurnId;
}

function selectLatestCodexModelContext(items, historyMode) {
  if (historyMode !== "paginated") {
    return { mode: "complete_replay", items };
  }
  let sawCompaction = false;
  let sawCompletedTurnContext = false;
  let mustScanToStart = false;
  let active = { turnId: null, hasUserTurn: false, hasTurnContext: false };
  const newestFirst = [];
  const finalizeActive = () => {
    if (active.hasUserTurn && active.hasTurnContext) sawCompletedTurnContext = true;
    active = { turnId: null, hasUserTurn: false, hasTurnContext: false };
  };

  for (let index = items.length - 1; index >= 1; index -= 1) {
    const item = items[index];
    newestFirst.push(item);
    if (!mustScanToStart) {
      const payload = item.payload;
      if (item.type === "compacted") {
        if (!Array.isArray(payload.replacement_history)
            || !Number.isInteger(payload.window_number)) {
          mustScanToStart = true;
        } else {
          sawCompaction = true;
        }
      } else if (item.type === "turn_context") {
        const turnId = typeof payload.turn_id === "string" ? payload.turn_id : null;
        if (active.turnId == null) active.turnId = turnId;
        if (compatibleTurnId(active.turnId, turnId)) active.hasTurnContext = true;
      } else if (item.type === "event_msg") {
        const eventType = payload.type;
        const turnId = typeof payload.turn_id === "string" ? payload.turn_id : null;
        if (eventType === "thread_rolled_back") {
          mustScanToStart = true;
        } else if (eventType === "item_completed") {
          if (active.turnId == null) active.turnId = turnId;
          if (compatibleTurnId(active.turnId, turnId)
              && payload.item?.type === "user_message") active.hasUserTurn = true;
        } else if (eventType === "task_complete" || eventType === "turn_aborted") {
          if (active.turnId == null) active.turnId = turnId;
        } else if (eventType === "task_started") {
          if (compatibleTurnId(active.turnId, turnId)) finalizeActive();
        } else if (eventType === "user_message") {
          active.hasUserTurn = true;
        }
      } else if (item.type === "inter_agent_communication") {
        active.hasUserTurn = true;
      }
    }
    if (!mustScanToStart && sawCompaction && sawCompletedTurnContext) {
      return {
        mode: "bounded_compaction_suffix",
        items: [items[0], ...newestFirst.reverse()],
      };
    }
  }
  return { mode: "complete_replay", items };
}

function reconstructCodexEffectiveContext(items, sessionMeta) {
  let history = [];
  let referenceTurnContext = null;
  let worldStateReplay = [];
  let window = sessionMeta.payload.context_window ?? null;
  for (const item of items.slice(1)) {
    if (item.type === "response_item") {
      history.push(structuredClone(item.payload));
    } else if (item.type === "inter_agent_communication") {
      history.push({ type: "inter_agent_communication", ...structuredClone(item.payload) });
    } else if (item.type === "compacted") {
      if (!Array.isArray(item.payload.replacement_history)) {
        throw new TypeError(
          "Codex compaction without replacement_history cannot produce an exact effective snapshot",
        );
      }
      history = structuredClone(item.payload.replacement_history);
      referenceTurnContext = null;
      worldStateReplay = [];
      window = {
        number: item.payload.window_number ?? null,
        firstId: item.payload.first_window_id ?? null,
        previousId: item.payload.previous_window_id ?? null,
        id: item.payload.window_id ?? null,
      };
    } else if (item.type === "turn_context") {
      referenceTurnContext = structuredClone(item.payload);
    } else if (item.type === "world_state") {
      worldStateReplay.push(structuredClone(item.payload));
    } else if (item.type === "event_msg" && item.payload.type === "thread_rolled_back") {
      throw new TypeError("Codex rollback requires provider-owned context reconstruction");
    }
  }
  return {
    baseInstructions: structuredClone(sessionMeta.payload.base_instructions),
    history,
    referenceTurnContext,
    worldStateReplay,
    window,
  };
}

export function normalizeCodexEffectiveContextSnapshot({
  records,
  sourceRevision,
  sourceSizeBytes,
}, expectedThreadId) {
  requireText(expectedThreadId, "effective context snapshot expected thread id");
  requireText(sourceRevision, "effective context snapshot source revision");
  if (!/^sha256:[a-f0-9]{64}$/.test(sourceRevision)) {
    throw new TypeError("effective context snapshot source revision must be SHA-256");
  }
  if (!Number.isSafeInteger(sourceSizeBytes) || sourceSizeBytes < 1) {
    throw new TypeError("effective context snapshot source size must be a positive integer");
  }
  if (!Array.isArray(records) || records.length === 0) {
    throw new TypeError("effective context snapshot requires rollout records");
  }
  const items = records.map(rolloutItem);
  const sessionMeta = items[0];
  if (sessionMeta.type !== "session_meta") {
    throw new TypeError("Codex rollout must begin with session_meta");
  }
  if (sessionMeta.payload.id !== expectedThreadId) {
    throw new TypeError("Codex rollout session does not match the requested thread");
  }
  requireText(sessionMeta.payload.base_instructions?.text, "Codex rollout base instructions");
  const historyMode = sessionMeta.payload.history_mode ?? "full";
  requireText(historyMode, "Codex rollout history mode");
  const selected = selectLatestCodexModelContext(items, historyMode);
  const effectiveContext = reconstructCodexEffectiveContext(selected.items, sessionMeta);
  const identity = {
    schemaVersion: CODEX_EFFECTIVE_CONTEXT_SNAPSHOT_SCHEMA_VERSION,
    type: CODEX_EFFECTIVE_CONTEXT_SNAPSHOT_TYPE,
    threadId: expectedThreadId,
    historyMode,
    selectionMode: selected.mode,
    effectiveContext,
  };
  const body = {
    ...identity,
    sourceRevision,
    sourceSizeBytes,
    replayItems: selected.items,
  };
  return deepFreeze({
    ...body,
    contextRevision: `sha256:${createHash("sha256").update(canonicalJson(identity)).digest("hex")}`,
  });
}

export async function readCodexRolloutSnapshot(
  rolloutPath,
  { maxBytes = DEFAULT_CODEX_ROLLOUT_SNAPSHOT_MAX_BYTES } = {},
) {
  requireText(rolloutPath, "Codex rollout path");
  if (!path.isAbsolute(rolloutPath) || !rolloutPath.endsWith(".jsonl")) {
    throw new TypeError("Codex rollout snapshot requires an absolute plain JSONL path");
  }
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 1) {
    throw new TypeError("Codex rollout snapshot maxBytes must be a positive integer");
  }
  const handle = await open(rolloutPath, "r");
  try {
    const before = await handle.stat();
    if (!before.isFile()) throw new TypeError("Codex rollout path is not a regular file");
    if (before.size < 1 || before.size > maxBytes) {
      throw new RangeError("Codex rollout exceeds the configured snapshot byte limit");
    }
    const content = await handle.readFile({ encoding: "utf8" });
    const after = await handle.stat();
    if (before.dev !== after.dev || before.ino !== after.ino || before.size !== after.size
        || before.mtimeMs !== after.mtimeMs || Buffer.byteLength(content, "utf8") !== after.size) {
      throw new Error("Codex rollout changed while the context snapshot was being read");
    }
    const records = content.split("\n").filter((line) => line.trim() !== "").map((line, index) => {
      try {
        return JSON.parse(line);
      } catch (error) {
        throw new TypeError(`Codex rollout line ${index + 1} is not valid JSON`, { cause: error });
      }
    });
    return {
      records,
      sourceRevision: `sha256:${createHash("sha256").update(content).digest("hex")}`,
      sourceSizeBytes: after.size,
    };
  } finally {
    await handle.close();
  }
}

function environmentFingerprint({ dynamicTools, providerCapabilities, role, threadOptions }) {
  return createHash("sha256").update(canonicalJson({
    developerInstructions: role.developerInstructions ?? null,
    dynamicTools,
    providerCapabilities,
    runtimeEnvironmentRevision: role.runtimeEnvironmentRevision ?? null,
    threadOptions,
  })).digest("hex");
}

function checkedThreadOptions(role) {
  const options = role.threadOptions ?? {};
  for (const reserved of ["developerInstructions", "dynamicTools", "ephemeral"]) {
    if (reserved in options) {
      throw new Error(`thread option ${reserved} is owned by the runtime adapter`);
    }
  }
  return options;
}

export class CodexAppServerAdapter {
  constructor({
    transport,
    registry,
    skillResolver,
    protocol = PINNED_PROTOCOL,
    providerRuntimeProfile = PINNED_PROVIDER_RUNTIME,
    configuredProviderFeatures = [],
    transitionGate = null,
    roleToolBridgeResolver = null,
    rolloutSnapshotReader = readCodexRolloutSnapshot,
  }) {
    if (transitionGate
        && (typeof transitionGate.runTurnAdmission !== "function"
          || typeof transitionGate.admitToolEffect !== "function")) {
      throw new TypeError("transition gate must provide turn and tool-effect admission");
    }
    if (typeof rolloutSnapshotReader !== "function") {
      throw new TypeError("rollout snapshot reader must be a function");
    }
    if (roleToolBridgeResolver !== null && typeof roleToolBridgeResolver !== "function") {
      throw new TypeError("role tool bridge resolver must be a function");
    }
    this.transport = transport;
    this.registry = registry;
    if (transitionGate) this.registry.setTransitionGate?.(transitionGate);
    this.skillResolver = skillResolver;
    this.protocol = protocol;
    this.providerRuntimeProfile = providerRuntimeProfile;
    this.configuredProviderFeatures = configuredProviderFeatures;
    this.transitionGate = transitionGate;
    this.rolloutSnapshotReader = rolloutSnapshotReader;
    this.roleToolBridgeResolver = roleToolBridgeResolver;
    this.negotiated = null;
    this.threadTools = new Map();
    this.turnCompletionOutcomes = new Map();
    this.turnCompletionWaiters = new Map();
    transport.onServerRequest((request) => this.#handleServerRequest(request));
    transport.onNotification((notification) => this.#handleNotification(notification));
    transport.onClosed?.((error) => this.#rejectTurnWaiters(error));
  }

  async initialize({
    clientInfo = { name: "work-engine", title: "Work Engine", version: "0.1.0" },
    requiredCapabilities = FOUNDATION_CAPABILITIES,
    requiredProviderCapabilities = [],
  } = {}) {
    if (this.negotiated) {
      for (const name of requiredCapabilities) this.#requireCapability(name);
      for (const name of requiredProviderCapabilities) this.requireProviderCapability(name);
      return this.negotiated;
    }
    const appServer = negotiateCapabilities(requiredCapabilities, this.protocol);
    // Reject impossible provider requirements before crossing the transport
    // boundary, even though the accepted response is adopted below.
    negotiateProviderCapabilities({
      required: requiredProviderCapabilities,
      configuredFeatures: this.configuredProviderFeatures,
      profile: this.providerRuntimeProfile,
      protocol: this.protocol,
    });
    const response = await this.transport.request("initialize", {
      clientInfo,
      capabilities: appServer.initializeCapabilities,
    });
    const negotiated = this.adoptInitialization(response, {
      requiredCapabilities,
      requiredProviderCapabilities,
    });
    this.transport.notify("initialized");
    return negotiated;
  }

  adoptInitialization(response, {
    requiredCapabilities = FOUNDATION_CAPABILITIES,
    requiredProviderCapabilities = [],
  } = {}) {
    if (this.negotiated) {
      for (const name of requiredCapabilities) this.#requireCapability(name);
      for (const name of requiredProviderCapabilities) this.requireProviderCapability(name);
      return this.negotiated;
    }
    const appServer = negotiateCapabilities(requiredCapabilities, this.protocol);
    const provider = negotiateProviderCapabilities({
      required: requiredProviderCapabilities,
      configuredFeatures: this.configuredProviderFeatures,
      profile: this.providerRuntimeProfile,
      protocol: this.protocol,
    });
    assertCompatibleServer(response, this.protocol);
    this.negotiated = Object.freeze({ ...appServer, provider });
    return this.negotiated;
  }

  requireProviderCapability(name) {
    const capability = this.negotiated?.provider?.selected?.[name];
    if (!capability) {
      throw new CapabilityError(`provider capability was not negotiated: ${name}`);
    }
    return capability;
  }

  onNotification(handler) {
    return this.transport.onNotification(handler);
  }

  async readThreadContextSnapshot({ threadId }) {
    if (!this.negotiated) throw new Error("App Server adapter is not initialized");
    requireText(threadId, "thread context snapshot thread id");
    this.#requireCapability("thread_read");
    this.#requireCapability("thread_turns_list");
    const metadata = await this.transport.request("thread/read", {
      threadId,
      includeTurns: false,
    });
    const turns = [];
    const cursors = new Set();
    let cursor = null;
    do {
      const page = await this.transport.request("thread/turns/list", {
        threadId,
        cursor,
        sortDirection: "asc",
        itemsView: "full",
      });
      if (!Array.isArray(page?.data)) {
        throw new TypeError("thread/turns/list response does not contain data");
      }
      turns.push(...page.data);
      cursor = page.nextCursor ?? null;
      if (cursor !== null) {
        requireText(cursor, "thread turns pagination cursor");
        if (cursors.has(cursor)) throw new TypeError("thread turns pagination cursor repeated");
        cursors.add(cursor);
      }
    } while (cursor !== null);
    return normalizeCodexThreadContextSnapshot({
      thread: { ...metadata.thread, turns },
    }, threadId);
  }

  async readThreadEffectiveContextSnapshot({ threadId }) {
    if (!this.negotiated) throw new Error("App Server adapter is not initialized");
    requireText(threadId, "effective context snapshot thread id");
    this.#requireCapability("thread_read");
    const response = await this.transport.request("thread/read", {
      threadId,
      includeTurns: false,
    });
    if (response?.thread?.id !== threadId) {
      throw new TypeError("thread/read response does not match the requested thread");
    }
    requireText(response.thread.path, "thread/read rollout path");
    const source = await this.rolloutSnapshotReader(response.thread.path, { threadId });
    return normalizeCodexEffectiveContextSnapshot(source, threadId);
  }

  #retainTurnOutcome(key, outcome) {
    this.turnCompletionOutcomes.delete(key);
    this.turnCompletionOutcomes.set(key, outcome);
    while (this.turnCompletionOutcomes.size > RETAINED_TURN_COMPLETION_LIMIT) {
      this.turnCompletionOutcomes.delete(this.turnCompletionOutcomes.keys().next().value);
    }
  }

  #settleTurnWaiters(key, outcome) {
    const waiters = this.turnCompletionWaiters.get(key);
    if (!waiters) return;
    this.turnCompletionWaiters.delete(key);
    for (const waiter of waiters) {
      waiter.cleanup();
      if (outcome.error) waiter.reject(outcome.error);
      else waiter.resolve(outcome.value);
    }
  }

  #rejectTurnWaiters(error) {
    const failure = error instanceof Error
      ? error
      : new AppServerTurnError("App Server transport closed before turn completion");
    for (const [key, waiters] of this.turnCompletionWaiters) {
      this.turnCompletionWaiters.delete(key);
      for (const waiter of waiters) {
        waiter.cleanup();
        waiter.reject(failure);
      }
    }
  }

  #handleNotification(notification) {
    if (notification?.method !== "turn/completed") return;
    let outcome;
    try {
      outcome = completedTurnOutcome(notification);
    } catch (error) {
      this.#rejectTurnWaiters(error);
      return;
    }
    this.#retainTurnOutcome(outcome.key, outcome);
    this.#settleTurnWaiters(outcome.key, outcome);
  }

  waitForTurnCompletion({ threadId, turnId, replayedDelivery = false, signal } = {}) {
    const key = turnKey(threadId, turnId);
    const retained = this.turnCompletionOutcomes.get(key);
    if (retained) {
      return retained.error ? Promise.reject(retained.error) : Promise.resolve(retained.value);
    }
    const activeWaiters = this.turnCompletionWaiters.get(key);
    if (replayedDelivery && !activeWaiters) {
      return Promise.reject(new AppServerTurnError(
        "completed turn output is unavailable for the replayed delivery; reconcile it from App Server thread state",
        { threadId, turnId },
      ));
    }
    if (signal?.aborted) {
      return Promise.reject(signal.reason ?? new Error("turn completion wait aborted"));
    }
    return new Promise((resolve, reject) => {
      const waiters = activeWaiters ?? new Set();
      const waiter = {
        resolve,
        reject,
        cleanup: () => signal?.removeEventListener("abort", onAbort),
      };
      const onAbort = () => {
        waiters.delete(waiter);
        if (waiters.size === 0) this.turnCompletionWaiters.delete(key);
        reject(signal.reason ?? new Error("turn completion wait aborted"));
      };
      waiters.add(waiter);
      this.turnCompletionWaiters.set(key, waiters);
      signal?.addEventListener("abort", onAbort, { once: true });
    });
  }

  async runEphemeralTurn({
    developerInstructions,
    text,
    clientUserMessageId,
    threadOptions = {},
    signal,
  }) {
    if (!this.negotiated) throw new Error("App Server adapter is not initialized");
    requireText(developerInstructions, "ephemeral turn developer instructions");
    requireText(text, "ephemeral turn text");
    requireText(clientUserMessageId, "ephemeral turn client user message id");
    this.#requireCapability("thread_start");
    this.#requireCapability("turn_start");
    this.#requireCapability("client_message_id");
    const options = checkedThreadOptions({ threadOptions });
    const tokenUsageByTurn = new Map();
    let threadId = null;
    const detach = this.onNotification((notification) => {
      if (notification?.method !== "thread/tokenUsage/updated"
          || notification.params?.threadId !== threadId
          || typeof notification.params?.turnId !== "string") return;
      tokenUsageByTurn.set(notification.params.turnId, notification.params.tokenUsage ?? null);
    });
    try {
      const threadResponse = await this.transport.request("thread/start", {
        ...options,
        developerInstructions,
        ephemeral: true,
      });
      threadId = threadIdFrom(threadResponse, "ephemeral thread/start");
      const turnResponse = await this.transport.request("turn/start", {
        threadId,
        clientUserMessageId,
        input: [{ type: "text", text, text_elements: [] }],
      });
      const turnId = turnResponse?.turn?.id;
      requireText(turnId, "ephemeral turn/start response turn id");
      const completion = await this.waitForTurnCompletion({ threadId, turnId, signal });
      return Object.freeze({
        threadId,
        turnId,
        completion,
        tokenUsage: tokenUsageByTurn.get(turnId) ?? null,
      });
    } finally {
      detach();
    }
  }

  #requireCapability(name) {
    if (!this.negotiated?.selected[name]) {
      throw new Error(`App Server capability was not negotiated: ${name}`);
    }
  }

  async #handleServerRequest(request) {
    if (request.method !== "item/tool/call") {
      throw new Error(`unsupported App Server request: ${request.method}`);
    }
    const bridge = this.threadTools.get(request.params?.threadId);
    if (!bridge) {
      return {
        success: false,
        contentItems: [{
          type: "inputText",
          text: "No dynamic-tool bridge is registered for this runtime thread",
        }],
      };
    }
    if (this.transitionGate) {
      const admission = await this.transitionGate.admitToolEffect({
        threadId: request.params?.threadId,
        request,
      });
      if (!admission?.allowed) {
        return {
          success: false,
          contentItems: [{
            type: "inputText",
            text: "Dynamic tool effects are unavailable while a context transition is fenced",
          }],
        };
      }
    }
    return bridge.dispatch(request.params);
  }

  async deliverTurn({
    role,
    text,
    clientUserMessageId,
    skills = [],
    toolBridge = null,
    requestContext = null,
    transitionLease = null,
    transitionPreparation = null,
  }) {
    requireText(role?.logicalRoleInstanceId, "logical role instance id");
    const resolvedToolBridge = toolBridge
      ?? this.roleToolBridgeResolver?.(role.capabilities ?? [], role)
      ?? null;
    if (transitionLease || transitionPreparation) {
      this.requireProviderCapability("model_context_replacement");
    }
    if (!this.transitionGate) {
      if (transitionLease || transitionPreparation) {
        throw new Error("transition control delivery requires an adapter transition gate");
      }
      return this.#deliverTurn({
        role, text, clientUserMessageId, skills, toolBridge: resolvedToolBridge, requestContext,
      });
    }
    return this.transitionGate.runTurnAdmission({
      logicalRoleInstanceId: role.logicalRoleInstanceId,
      text,
      transitionLease,
      transitionPreparation,
      skills,
      toolBridge: resolvedToolBridge,
      requestContext,
    }, (admissionPermit) => this.#deliverTurn({
      role, text, clientUserMessageId, skills, toolBridge: resolvedToolBridge, requestContext,
      boundControlTurn: Boolean(transitionLease || transitionPreparation),
      transitionAdmissionPermit: admissionPermit,
    }));
  }

  async #deliverTurn({
    role,
    text,
    clientUserMessageId,
    skills = [],
    toolBridge = null,
    requestContext = null,
    boundControlTurn = false,
    transitionAdmissionPermit = null,
  }) {
    if (!this.negotiated) throw new Error("App Server adapter is not initialized");
    requireText(role?.logicalRoleInstanceId, "logical role instance id");
    requireText(text, "turn text");
    requireText(clientUserMessageId, "client user message id");
    this.#requireCapability("turn_start");
    this.#requireCapability("client_message_id");
    if (skills.length > 0) this.#requireCapability("exact_skill_input");
    const dynamicTools = toolBridge?.specs() ?? [];
    if (dynamicTools.length > 0) this.#requireCapability("thread_scoped_dynamic_tools");
    const threadOptions = checkedThreadOptions(role);
    let binding = await this.registry.get(role.logicalRoleInstanceId);
    if (boundControlTurn && !binding) {
      throw new Error("retirement control turn requires an existing runtime binding");
    }
    const environmentDynamicTools = boundControlTurn
      ? (this.threadTools.get(binding.threadId)?.specs() ?? [])
      : dynamicTools;
    const bindingFingerprint = environmentFingerprint({
      dynamicTools: environmentDynamicTools,
      providerCapabilities: this.negotiated.provider,
      role,
      threadOptions,
    });

    const skillItems = await this.skillResolver.resolve(skills);
    const requestContextItem = compileRequestContextInput(requestContext);
    const requestFingerprint = createHash("sha256").update(canonicalJson({
      logicalRoleInstanceId: role.logicalRoleInstanceId,
      text,
      skillItems,
      requestContextInput: requestContextItem?.text ?? null,
      developerInstructions: role.developerInstructions ?? null,
      runtimeEnvironmentRevision: role.runtimeEnvironmentRevision ?? null,
      threadOptions,
      environmentFingerprint: bindingFingerprint,
    })).digest("hex");
    let createdThread = false;
    let threadId;

    const priorDelivery = await this.registry.getDelivery(
      role.logicalRoleInstanceId,
      clientUserMessageId,
    );
    if (priorDelivery && priorDelivery.requestFingerprint !== requestFingerprint) {
      throw new Error("client user message id was reused for different turn content");
    }
    if (priorDelivery?.status === "completed") {
      return {
        logicalRoleInstanceId: role.logicalRoleInstanceId,
        threadId: priorDelivery.threadId,
        turnId: priorDelivery.turnId,
        createdThread: false,
        replayedDelivery: true,
        binding,
      };
    }
    if (priorDelivery) {
      throw new Error(
        "turn delivery is pending and must be reconciled before retrying",
      );
    }

    if (binding) {
      this.#requireCapability("thread_resume");
      if (binding.environmentFingerprint !== bindingFingerprint) {
        throw new Error(
          "thread-scoped runtime environment changed; replace the runtime binding before resuming",
        );
      }
      const response = await this.transport.request("thread/resume", {
        threadId: binding.threadId,
        ...threadOptions,
        ...(role.developerInstructions
          ? { developerInstructions: role.developerInstructions }
          : {}),
      });
      threadId = threadIdFrom(response, "thread/resume");
      if (threadId !== binding.threadId) {
        throw new Error("resumed App Server thread does not match durable role binding");
      }
    } else {
      this.#requireCapability("thread_start");
      const response = await this.transport.request("thread/start", {
        ...threadOptions,
        ...(role.developerInstructions
          ? { developerInstructions: role.developerInstructions }
          : {}),
        ephemeral: false,
        ...(dynamicTools.length > 0 ? { dynamicTools } : {}),
      });
      threadId = threadIdFrom(response, "thread/start");
      binding = await this.registry.bind({
        logicalRoleInstanceId: role.logicalRoleInstanceId,
        threadId,
        protocolVersion: this.negotiated.protocolVersion,
        environmentFingerprint: bindingFingerprint,
        expectedBindingRevision: null,
      }, { transitionAdmissionPermit });
      createdThread = true;
    }

    if (toolBridge) this.threadTools.set(threadId, toolBridge);
    const delivery = await this.registry.beginDelivery({
      logicalRoleInstanceId: role.logicalRoleInstanceId,
      clientUserMessageId,
      threadId,
      requestFingerprint,
    });
    if (!delivery.created) {
      if (delivery.delivery.requestFingerprint !== requestFingerprint) {
        throw new Error("client user message id was concurrently reused for different content");
      }
      if (delivery.delivery.status === "completed") {
        return {
          logicalRoleInstanceId: role.logicalRoleInstanceId,
          threadId: delivery.delivery.threadId,
          turnId: delivery.delivery.turnId,
          createdThread: false,
          replayedDelivery: true,
          binding,
        };
      }
      throw new Error("turn delivery was concurrently claimed and requires reconciliation");
    }
    const response = await this.transport.request("turn/start", {
      threadId,
      clientUserMessageId,
      input: [
        ...skillItems,
        ...(requestContextItem ? [requestContextItem] : []),
        { type: "text", text, text_elements: [] },
      ],
    });
    requireText(response?.turn?.id, "turn/start response turn id");
    await this.registry.completeDelivery({
      logicalRoleInstanceId: role.logicalRoleInstanceId,
      clientUserMessageId,
      threadId,
      turnId: response.turn.id,
    });
    return {
      logicalRoleInstanceId: role.logicalRoleInstanceId,
      threadId,
      turnId: response.turn.id,
      createdThread,
      replayedDelivery: false,
      binding,
    };
  }
}
