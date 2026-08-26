import { createHash } from "node:crypto";

export const CONTEXT_INPUT_SCHEMA_VERSION = 1;
export const CONTEXT_INPUT_TYPE = "work-engine.context-transition-input";

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

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonical(value[key])}`
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function revision(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export function normalizeContextTransitionInput(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError("context transition input must be an object");
  }
  const allowed = new Set([
    "schemaVersion", "type", "logicalRoleInstanceId", "roleId", "instanceId",
    "threadId", "bindingRevision", "clientUserMessageId", "sourceKind", "text",
    "inputRevision",
  ]);
  const unknown = Object.keys(value).filter((field) => !allowed.has(field));
  if (unknown.length > 0) {
    throw new TypeError(`context transition input has unknown fields: ${unknown.sort().join(", ")}`);
  }
  if (value.schemaVersion !== undefined && value.schemaVersion !== CONTEXT_INPUT_SCHEMA_VERSION) {
    throw new TypeError("context transition input schema version is unsupported");
  }
  if (value.type !== undefined && value.type !== CONTEXT_INPUT_TYPE) {
    throw new TypeError("context transition input type is unsupported");
  }
  const body = {
    schemaVersion: CONTEXT_INPUT_SCHEMA_VERSION,
    type: CONTEXT_INPUT_TYPE,
    logicalRoleInstanceId: text(value.logicalRoleInstanceId, "context input role"),
    roleId: text(value.roleId, "context input role id"),
    instanceId: text(value.instanceId, "context input instance id"),
    threadId: text(value.threadId, "context input thread"),
    bindingRevision: positiveInteger(value.bindingRevision, "context input binding revision"),
    clientUserMessageId: text(value.clientUserMessageId, "context input client message id"),
    sourceKind: value.sourceKind === "human" || value.sourceKind === "agent"
      ? value.sourceKind
      : (() => { throw new TypeError("context input source kind must be human or agent"); })(),
    text: text(value.text, "context input text"),
  };
  if (`${body.roleId}:${body.instanceId}` !== body.logicalRoleInstanceId) {
    throw new TypeError("context input role identity is inconsistent");
  }
  const inputRevision = revision(body);
  if (value.inputRevision !== undefined && value.inputRevision !== inputRevision) {
    throw new TypeError("context transition input revision does not match its content");
  }
  return freeze({ ...body, inputRevision });
}

export class ContextInputCustodyController {
  constructor({ store }) {
    if (!store || typeof store.queueContextInput !== "function"
        || typeof store.nextContextInputForRelease !== "function") {
      throw new TypeError("context input custody requires a durable queue store");
    }
    this.store = store;
    this.roleTails = new Map();
  }

  #serialize(logicalRoleInstanceId, operation) {
    const predecessor = this.roleTails.get(logicalRoleInstanceId) ?? Promise.resolve();
    const running = predecessor.then(operation, operation);
    const settled = running.then(() => undefined, () => undefined);
    this.roleTails.set(logicalRoleInstanceId, settled);
    settled.finally(() => {
      if (this.roleTails.get(logicalRoleInstanceId) === settled) {
        this.roleTails.delete(logicalRoleInstanceId);
      }
    });
    return running;
  }

  closeAdmission(fence) {
    const logicalRoleInstanceId = text(
      fence?.logicalRoleInstanceId,
      "context input admission role",
    );
    return this.#serialize(logicalRoleInstanceId, () =>
      this.store.closeContextInputAdmission(fence)
    );
  }

  admission(logicalRoleInstanceId) {
    text(logicalRoleInstanceId, "context input admission role");
    return this.store.contextInputAdmission(logicalRoleInstanceId);
  }

  queueIfClosed(input) {
    const normalized = normalizeContextTransitionInput(input);
    return this.#serialize(normalized.logicalRoleInstanceId, () => {
      const admission = this.store.contextInputAdmission(normalized.logicalRoleInstanceId);
      if (!admission || admission.status === "open") {
        return freeze({ status: "admission_open", input: normalized });
      }
      return this.store.queueContextInput(normalized);
    });
  }

  releaseAfterReconciliation({
    logicalRoleInstanceId,
    transitionRevision,
    reconciliationRevision,
  }, deliver) {
    text(logicalRoleInstanceId, "context input release role");
    text(transitionRevision, "context input release transition revision");
    text(reconciliationRevision, "context input release reconciliation revision");
    if (typeof deliver !== "function") {
      throw new TypeError("context input release requires a delivery function");
    }
    return this.#serialize(logicalRoleInstanceId, async () => {
      const releasing = this.store.beginContextInputRelease({
        logicalRoleInstanceId,
        transitionRevision,
        reconciliationRevision,
      });
      if (releasing.status === "replayed") return releasing;
      const released = [];
      for (;;) {
        const item = this.store.nextContextInputForRelease({
          logicalRoleInstanceId,
          transitionRevision,
        });
        if (item === null) break;
        const delivery = await deliver(item.input);
        const receipt = this.store.completeContextInputRelease({
          queueId: item.queueId,
          inputRevision: item.input.inputRevision,
          delivery,
        });
        released.push(receipt);
      }
      const admission = this.store.reopenContextInputAdmission({
        logicalRoleInstanceId,
        transitionRevision,
        reconciliationRevision,
      });
      return freeze({ status: "released", admission, released });
    });
  }
}
