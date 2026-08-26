import { randomUUID } from "node:crypto";

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function withoutArguments(argument, command) {
  if (argument !== null) throw new Error(`:we ${command} does not accept arguments`);
}

function parseAttachment(value) {
  requireText(value, "switchboard attachment");
  const separator = value.indexOf(":");
  if (separator < 1 || separator !== value.lastIndexOf(":") || separator === value.length - 1) {
    throw new Error("attachment must be formatted as role:instance");
  }
  return { roleId: value.slice(0, separator), instanceId: value.slice(separator + 1) };
}

function bindingView(binding) {
  if (!binding) return null;
  return {
    logicalRoleInstanceId: binding.logicalRoleInstanceId,
    provider: binding.provider,
    threadId: binding.threadId,
    protocolVersion: binding.protocolVersion,
    environmentFingerprint: binding.environmentFingerprint,
    bindingRevision: binding.bindingRevision,
    boundAt: binding.boundAt,
  };
}

function observationView(observer) {
  if (!observer) return null;
  const snapshot = observer.snapshot();
  return {
    latestSequence: snapshot.latestSequence,
    retainedEventCount: snapshot.events.length,
    observationErrorCount: snapshot.observationErrors.length,
    latestEvent: snapshot.events.at(-1) ?? null,
  };
}

export class OperatorSwitchboard {
  #tail = Promise.resolve();

  constructor({
    manifest,
    runtime,
    registry,
    observer = null,
    completionWaiter = null,
    initialAttachment = null,
    onAttachmentChange = null,
    messageIdFactory = () => `operator:${randomUUID()}`,
  }) {
    if (!manifest || !Array.isArray(manifest.roleIds)
        || typeof manifest.projectRole !== "function") {
      throw new TypeError("operator switchboard requires a runtime manifest");
    }
    if (!runtime || typeof runtime.deliverTurn !== "function") {
      throw new TypeError("operator switchboard requires a manifest role runtime");
    }
    if (!registry || typeof registry.get !== "function"
        || typeof registry.listBindings !== "function") {
      throw new TypeError("operator switchboard requires a readable role binding registry");
    }
    if (observer !== null && typeof observer.snapshot !== "function") {
      throw new TypeError("operator switchboard observer must provide snapshots");
    }
    if (completionWaiter !== null && typeof completionWaiter !== "function") {
      throw new TypeError("operator switchboard completion waiter must be a function or null");
    }
    if (typeof messageIdFactory !== "function") {
      throw new TypeError("operator switchboard message id factory must be a function");
    }
    if (onAttachmentChange !== null && typeof onAttachmentChange !== "function") {
      throw new TypeError("operator switchboard attachment writer must be a function or null");
    }
    this.manifest = manifest;
    this.runtime = runtime;
    this.registry = registry;
    this.observer = observer;
    this.completionWaiter = completionWaiter;
    this.onAttachmentChange = onAttachmentChange;
    this.messageIdFactory = messageIdFactory;
    if (initialAttachment !== null) {
      const attachment = parseAttachment(
        `${initialAttachment?.roleId ?? ""}:${initialAttachment?.instanceId ?? ""}`,
      );
      this.manifest.projectRole(attachment.roleId, attachment.instanceId);
      this.attachment = freeze(attachment);
    } else {
      this.attachment = null;
    }
    this.lastLifecycle = null;
  }

  handleLine(line, { signal, clientUserMessageId = null } = {}) {
    return this.startLine(line, { signal, clientUserMessageId }).then((started) =>
      started.result ?? started.completion
    );
  }

  startLine(line, { signal, clientUserMessageId = null } = {}) {
    requireText(line, "operator input");
    const operation = this.#tail.then(() =>
      this.#startLine(line, signal, clientUserMessageId)
    );
    this.#tail = operation.then(
      (started) => started.completion?.then(() => {}, () => {}),
      () => {},
    );
    return operation;
  }

  async #startLine(line, signal, suppliedClientUserMessageId) {
    const normalized = line.trim();
    if (normalized.startsWith(":we")) {
      return { result: await this.#handleCommand(normalized), completion: null };
    }
    if (!this.attachment) {
      throw new Error("no role instance is attached; use :we attach role:instance");
    }
    const clientUserMessageId = requireText(
      suppliedClientUserMessageId ?? this.messageIdFactory(),
      "operator client user message id",
    );
    const runtimeResult = await this.runtime.deliverTurn({
      ...this.attachment,
      clientUserMessageId,
      text: line,
      signal,
    });
    const delivery = runtimeResult?.delivery ?? runtimeResult;
    let completionPromise = runtimeResult?.completion ?? null;
    if (!completionPromise) {
      if (!this.completionWaiter) {
        throw new Error("operator switchboard runtime did not provide turn completion");
      }
      completionPromise = this.completionWaiter({
        threadId: delivery.threadId,
        turnId: delivery.turnId,
        replayedDelivery: delivery.replayedDelivery,
        signal,
      });
    }
    const attachment = { ...this.attachment };
    const deliveryView = {
      logicalRoleInstanceId: delivery.logicalRoleInstanceId,
      threadId: delivery.threadId,
      turnId: delivery.turnId,
      createdThread: delivery.createdThread,
      replayedDelivery: delivery.replayedDelivery,
      bindingRevision: delivery.binding?.bindingRevision ?? null,
    };
    const completion = Promise.resolve(completionPromise).then((outcome) => {
      const shadow = runtimeResult?.shadow == null ? null : structuredClone(runtimeResult.shadow);
      this.lastLifecycle = shadow;
      return freeze({
        kind: "message",
        attachment,
        delivery: deliveryView,
        outputText: outcome.outputText ?? null,
        shadow,
      });
    });
    return {
      result: null,
      completion,
      attachment: freeze(attachment),
      delivery: freeze(deliveryView),
    };
  }

  async #handleCommand(line) {
    const match = /^:we\s+([a-z]+)(?:\s+(.+))?$/.exec(line);
    if (!match) throw new Error("invalid switchboard command syntax");
    const [, command, argument = null] = match;
    if (command === "agents") {
      withoutArguments(argument, command);
      return freeze({ kind: "command", command, roles: [...this.manifest.roleIds] });
    }
    if (command === "attach") {
      const attachment = parseAttachment(argument);
      const projection = this.manifest.projectRole(attachment.roleId, attachment.instanceId);
      const binding = bindingView(await this.registry.get(
        projection.role.logicalRoleInstanceId,
      ));
      await this.onAttachmentChange?.(attachment);
      this.attachment = freeze(attachment);
      return freeze({
        kind: "command",
        command,
        attachment: { ...this.attachment },
        binding,
      });
    }
    if (command === "detach") {
      withoutArguments(argument, command);
      const previousAttachment = this.attachment;
      await this.onAttachmentChange?.(null);
      this.attachment = null;
      return freeze({ kind: "command", command, previousAttachment });
    }
    if (command === "status") {
      withoutArguments(argument, command);
      const attachment = this.attachment ? { ...this.attachment } : null;
      const logicalRoleInstanceId = attachment
        ? `${attachment.roleId}:${attachment.instanceId}`
        : null;
      return freeze({
        kind: "command",
        command,
        attachment,
        binding: logicalRoleInstanceId
          ? bindingView(await this.registry.get(logicalRoleInstanceId))
          : null,
        observation: observationView(this.observer),
        lifecycle: this.lastLifecycle,
      });
    }
    if (command === "threads") {
      withoutArguments(argument, command);
      const bindings = await this.registry.listBindings();
      return freeze({
        kind: "command",
        command,
        threads: bindings.map(bindingView),
      });
    }
    throw new Error(`unknown switchboard command :we ${command}`);
  }
}

export function renderOperatorSwitchboardResult(result) {
  if (result?.kind === "message") {
    const output = result.outputText ?? "[turn completed without final agent output]";
    if (!result.shadow) return output;
    const lifecycle = {
      status: result.shadow.status,
      reason: result.shadow.reason ?? null,
      episodeId: result.shadow.episode?.episodeId ?? null,
      disposition: result.shadow.episode?.pressure?.disposition ?? null,
      inference: result.shadow.episode?.inference?.status ?? null,
      checkpoint: result.shadow.episode?.checkpoint?.status ?? null,
      transition: result.shadow.episode?.transition?.status ?? null,
    };
    return `${output}\n[lifecycle] ${JSON.stringify(lifecycle)}`;
  }
  return JSON.stringify(result, null, 2);
}
