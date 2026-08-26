import { AsyncLocalStorage } from "node:async_hooks";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";

import { CodexAppServerAdapter } from "./codex-app-server-adapter.mjs";
import { FileRoleBindingRegistry } from "./role-binding-registry.mjs";
import { ManifestRoleRuntime, projectRuntimeManifest } from "./runtime-manifest.mjs";
import { ExactSkillResolver } from "./skill-resolver.mjs";
import {
  OperatorSwitchboard,
  renderOperatorSwitchboardResult,
} from "./operator-switchboard.mjs";

function requireText(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
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

function notificationThreadId(notification) {
  return notification?.params?.threadId ?? notification?.params?.thread?.id ?? null;
}

function exactTextInput(params) {
  if (!Array.isArray(params?.input) || params.input.length !== 1
      || params.input[0]?.type !== "text" || typeof params.input[0].text !== "string") {
    throw new TypeError("switchboard turns currently require exactly one text input");
  }
  return params.input[0].text;
}

function syntheticTurnStart(threadId, startedAtMs) {
  const turnId = randomUUID();
  const responseTurn = {
    id: turnId,
    items: [],
    itemsView: "notLoaded",
    status: "inProgress",
    error: null,
    startedAt: null,
    completedAt: null,
    durationMs: null,
  };
  const startedTurn = {
    ...responseTurn,
    itemsView: "full",
    startedAt: Math.floor(startedAtMs / 1_000),
  };
  return {
    turnId,
    response: { turn: responseTurn },
    notifications: [{
      method: "turn/started",
      params: { threadId, turn: startedTurn },
    }],
  };
}

function syntheticTurnCompletion(
  threadId,
  turnId,
  outputText,
  { startedAtMs, completedAtMs, status = "completed", error = null },
) {
  const item = {
    type: "agentMessage",
    id: randomUUID(),
    text: outputText,
    phase: "final_answer",
    memoryCitation: null,
    delivery: null,
  };
  const completed = {
      id: turnId,
      items: outputText === null ? [] : [item],
      itemsView: "full",
      status,
      error,
      startedAt: Math.floor(startedAtMs / 1_000),
      completedAt: Math.floor(completedAtMs / 1_000),
      durationMs: completedAtMs - startedAtMs,
  };
  return {
    notifications: [{
      method: "turn/completed",
      params: { threadId, turn: completed },
    }],
  };
}

async function readAttachment(filePath) {
  try {
    const value = JSON.parse(await readFile(filePath, "utf8"));
    if (value === null) return null;
    requireText(value?.roleId, "stored attachment role id");
    requireText(value?.instanceId, "stored attachment instance id");
    return { roleId: value.roleId, instanceId: value.instanceId };
  } catch (error) {
    if (error?.code === "ENOENT") return null;
    throw error;
  }
}

async function writeAttachment(filePath, attachment) {
  await mkdir(path.dirname(filePath), { recursive: true });
  const temporary = `${filePath}.${process.pid}.${randomUUID()}.tmp`;
  await writeFile(temporary, `${JSON.stringify(attachment)}\n`, {
    encoding: "utf8",
    mode: 0o600,
  });
  await rename(temporary, filePath);
}

class DispatchEffectTransport {
  constructor(dynamicTools) {
    this.effects = new AsyncLocalStorage();
    this.dynamicTools = dynamicTools;
    this.notificationHandlers = new Set();
    this.serverRequestHandler = null;
    this.roleThreadIds = new Set();
    this.pendingRoleThreadStarts = 0;
  }

  run(effect, operation) {
    return this.effects.run(effect, operation);
  }

  async request(method, params) {
    const effect = this.effects.getStore();
    if (typeof effect !== "function") {
      throw new Error("role App Server request is outside an admitted worker dispatch");
    }
    const starting = method === "thread/start";
    if (starting) this.pendingRoleThreadStarts += 1;
    try {
      let projectedParams = params;
      if (["thread/start", "thread/resume"].includes(method)) {
        const existing = params?.dynamicTools ?? [];
        if (!Array.isArray(existing)) throw new TypeError("dynamicTools must be an array");
        if (existing.some((tool) => tool?.name === "environment")) {
          throw new Error("environment dynamic tool namespace is already declared");
        }
        projectedParams = { ...params, dynamicTools: [...existing, ...this.dynamicTools] };
      }
      const response = await effect({ method, params: projectedParams });
      const threadId = response?.thread?.id;
      if (["thread/start", "thread/resume"].includes(method) && typeof threadId === "string") {
        this.roleThreadIds.add(threadId);
      }
      return response;
    } finally {
      if (starting) this.pendingRoleThreadStarts -= 1;
    }
  }

  notify(method, params) {
    return this.request(method, params).then(() => undefined);
  }

  onNotification(handler) {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  onServerRequest(handler) {
    this.serverRequestHandler = handler;
  }

  onClosed() { return () => {}; }

  observesRoleThread(notification) {
    const threadId = notificationThreadId(notification);
    if (typeof threadId !== "string") return false;
    if (notification?.method === "thread/started" && this.pendingRoleThreadStarts > 0) {
      this.roleThreadIds.add(threadId);
    }
    return this.roleThreadIds.has(threadId);
  }

  emitNotification(notification) {
    for (const handler of this.notificationHandlers) handler(notification);
  }

  handleServerRequest(request) {
    if (!this.serverRequestHandler) throw new Error("role runtime has no server request handler");
    return this.serverRequestHandler(request);
  }
}

export async function createExecutableGenerationRoleEnvironment({
  snapshotRoot,
  configRelativePath = "app-server/generated/executable-role-environment.json",
  bindingsPath,
  attachmentPath,
  configuredProviderFeatures = [],
  dynamicTools = [],
  now = () => Date.now(),
} = {}) {
  const root = path.resolve(requireText(snapshotRoot, "generation snapshot root"));
  const config = JSON.parse(await readFile(path.join(root, configRelativePath), "utf8"));
  if (config?.schemaVersion !== 1 || !config.manifest?.document) {
    throw new TypeError("executable role environment config is invalid");
  }
  const manifestPath = path.join(root, ...config.manifest.relativePath.split("/"));
  const manifest = projectRuntimeManifest(config.manifest.document, {
    baseDirectory: path.dirname(manifestPath),
    identityBaseDirectory: config.manifest.identityBaseDirectory,
    sourcePath: manifestPath,
    sourceSha256: config.manifest.sha256,
  });
  if (!Array.isArray(dynamicTools)) {
    throw new TypeError("executable role environment dynamic tools must be an array");
  }
  const transport = new DispatchEffectTransport(dynamicTools);
  const registry = new FileRoleBindingRegistry(bindingsPath);
  const adapter = new CodexAppServerAdapter({
    transport,
    registry,
    skillResolver: await ExactSkillResolver.create([path.join(root, "skills")]),
    configuredProviderFeatures,
  });
  const runtime = new ManifestRoleRuntime({ adapter, manifest });
  const switchboard = new OperatorSwitchboard({
    manifest,
    runtime,
    registry,
    completionWaiter: (delivery) => adapter.waitForTurnCompletion(delivery),
    initialAttachment: await readAttachment(attachmentPath),
    onAttachmentChange: (attachment) => writeAttachment(attachmentPath, attachment),
  });
  const uiThreadIds = new Set();
  const pendingUiTurns = new Map();
  const completedRoleTurns = new Map();

  const completeUiTurn = async (pending, providerNotification) => {
    let result = null;
    let status = providerNotification.params.turn.status;
    let error = providerNotification.params.turn.error ?? null;
    try {
      result = await pending.completion;
    } catch (completionError) {
      status = status === "interrupted" ? "interrupted" : "failed";
      error ??= {
        message: completionError instanceof Error
          ? completionError.message
          : "attached role turn failed",
      };
    }
    const outputText = result === null ? null : renderOperatorSwitchboardResult(result);
    return syntheticTurnCompletion(
      pending.uiThreadId,
      pending.uiTurnId,
      outputText,
      { startedAtMs: pending.startedAtMs, completedAtMs: now(), status, error },
    );
  };

  return Object.freeze({
    environmentFingerprint() {
      return `sha256:${createHash("sha256").update(canonicalJson({
        manifestSha256: config.manifest.sha256,
        skillFiles: config.skillFiles,
        toolSpecification: dynamicTools,
      })).digest("hex")}`;
    },

    async handleRequest(payload, effect) {
      if (payload?.method === "initialize") {
        const response = await effect(payload);
        adapter.adoptInitialization(response);
        return { disposition: "respond", result: response };
      }
      if (["thread/start", "thread/resume"].includes(payload?.method)) {
        const response = await effect(payload);
        const threadId = response?.thread?.id;
        if (typeof threadId === "string") uiThreadIds.add(threadId);
        return { disposition: "respond", result: response };
      }
      if (payload?.method !== "turn/start" || !uiThreadIds.has(payload.params?.threadId)) {
        return { disposition: "forward", payload };
      }
      const text = exactTextInput(payload.params);
      const startedAtMs = now();
      const started = await transport.run(effect, () => switchboard.startLine(text, {
        clientUserMessageId: payload.params.clientUserMessageId ?? null,
      }));
      const lifecycle = syntheticTurnStart(payload.params.threadId, startedAtMs);
      if (started.result !== null) {
        const completion = syntheticTurnCompletion(
          payload.params.threadId,
          lifecycle.turnId,
          renderOperatorSwitchboardResult(started.result),
          { startedAtMs, completedAtMs: now() },
        );
        return {
          disposition: "respond",
          result: lifecycle.response,
          notifications: [...lifecycle.notifications, ...completion.notifications],
        };
      }
      const pending = {
        uiThreadId: payload.params.threadId,
        uiTurnId: lifecycle.turnId,
        startedAtMs,
        completion: started.completion,
      };
      pendingUiTurns.set(started.delivery.turnId, pending);
      const earlyCompletion = completedRoleTurns.get(started.delivery.turnId);
      if (earlyCompletion) {
        completedRoleTurns.delete(started.delivery.turnId);
        pendingUiTurns.delete(started.delivery.turnId);
        const completion = await completeUiTurn(pending, earlyCompletion);
        lifecycle.notifications.push(...completion.notifications);
      }
      return {
        disposition: "respond",
        result: lifecycle.response,
        notifications: lifecycle.notifications,
      };
    },

    async handleNotification(notification) {
      const roleOwned = transport.observesRoleThread(notification);
      if (roleOwned) transport.emitNotification(notification);
      if (!roleOwned) return { disposition: "forward" };
      if (notification?.method !== "turn/completed") {
        return { disposition: "respond", result: null };
      }
      const roleTurnId = notification.params?.turn?.id;
      const pending = pendingUiTurns.get(roleTurnId);
      if (!pending) {
        completedRoleTurns.set(roleTurnId, structuredClone(notification));
        if (completedRoleTurns.size > 256) {
          completedRoleTurns.delete(completedRoleTurns.keys().next().value);
        }
        return { disposition: "respond", result: null };
      }
      pendingUiTurns.delete(roleTurnId);
      const completion = await completeUiTurn(pending, notification);
      return {
        disposition: "respond",
        result: null,
        notifications: completion.notifications,
      };
    },

    async handleServerRequest(request) {
      if (!transport.roleThreadIds.has(request?.params?.threadId)) {
        return { disposition: "forward" };
      }
      return {
        disposition: "respond",
        result: await transport.handleServerRequest(request),
      };
    },
  });
}
