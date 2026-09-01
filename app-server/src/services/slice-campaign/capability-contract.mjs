import { SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL } from "./host-effect-runtime.mjs";

const CAPABILITIES = Object.freeze({
  "capability.preflight": Object.freeze(["run"]),
  "capability.lifecycle_control": Object.freeze([
    "admit", "recover", "advance", "bind_review_selection", "terminalize",
  ]),
  "capability.receipt_finalization": Object.freeze(["finalize_named_campaign"]),
  "capability.checkpoint_lifecycle": Object.freeze(["bind_candidate", "accept", "stop"]),
  "capability.completion_offer": Object.freeze([
    "open", "load", "resolve", "reconcile", "expire",
  ]),
  "capability.resume": Object.freeze(["recover_active", "recover_terminal"]),
});

const TOOL_NAMES = Object.freeze({
  "capability.preflight": "preflight",
  "capability.lifecycle_control": "lifecycle_control",
  "capability.receipt_finalization": "receipt_finalization",
  "capability.checkpoint_lifecycle": "checkpoint_lifecycle",
  "capability.completion_offer": "completion_offer",
  "capability.resume": "resume",
});

const INPUT_FIELDS = Object.freeze({
  "capability.preflight/run": [new Set(["campaign_path"]), new Set()],
  "capability.lifecycle_control/admit": [
    new Set(["identity", "workspace", "acceptedBoundary", "baseline"]),
    new Set(["expectedImpact"]),
  ],
  "capability.lifecycle_control/recover": [new Set(["identity"]), new Set()],
  "capability.lifecycle_control/advance": [
    new Set(["identity", "expectedRevision", "phase", "consequence"]), new Set(),
  ],
  "capability.lifecycle_control/bind_review_selection": [
    new Set(["identity", "expectedRevision", "selection"]), new Set(),
  ],
  "capability.lifecycle_control/terminalize": [
    new Set(["identity", "expectedRevision", "outcome", "receipt"]), new Set(),
  ],
  "capability.receipt_finalization/finalize_named_campaign": [
    new Set(["semantic_receipt", "telemetry_ingress", "campaign_preflight", "handoff_receipt"]),
    new Set(["checkpoint_receipt", "completion_commit_receipt"]),
  ],
  "capability.checkpoint_lifecycle/bind_candidate": [
    new Set(["identity", "expectedRevision", "request"]), new Set(),
  ],
  "capability.checkpoint_lifecycle/accept": [
    new Set(["candidate", "review_result", "gate_receipt_digest"]),
    new Set(["expected_accepted"]),
  ],
  "capability.checkpoint_lifecycle/stop": [new Set(["candidate"]), new Set()],
  "capability.completion_offer/open": [
    new Set(["identity", "expected_revision", "request"]), new Set(),
  ],
  "capability.completion_offer/load": [
    new Set(["repository", "run_id", "slice_number"]), new Set(),
  ],
  "capability.completion_offer/resolve": [
    new Set(["offer", "decision"]), new Set(),
  ],
  "capability.completion_offer/reconcile": [new Set(["offer"]), new Set()],
  "capability.completion_offer/expire": [new Set(["offer", "reason"]), new Set()],
  "capability.resume/recover_active": [new Set(["identity"]), new Set()],
  "capability.resume/recover_terminal": [
    new Set(["campaign_preflight", "run_id"]), new Set(),
  ],
});

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function exactFields(value, required, optional, label) {
  const allowed = new Set([...required, ...optional]);
  const unsupported = Object.keys(value).find((field) => !allowed.has(field));
  if (unsupported) throw new TypeError(`${label} contains unsupported field ${unsupported}`);
  const missing = [...required].find((field) => !Object.hasOwn(value, field));
  if (missing) throw new TypeError(`${label} requires field ${missing}`);
  return value;
}

function validateDecision(value) {
  requireRecord(value, "completion offer decision");
  exactFields(value, new Set(["decision", "authority"]), new Set(), "completion offer decision");
  if (!["create", "decline"].includes(value.decision)) {
    throw new TypeError("completion offer decision must be create or decline");
  }
  requireRecord(value.authority, "completion offer decision authority");
  exactFields(
    value.authority,
    new Set(["kind", "reference", "observed_at"]),
    new Set(),
    "completion offer decision authority",
  );
  if (value.authority.kind !== "human") {
    throw new TypeError("completion offer decision authority kind must be human");
  }
  requireText(value.authority.reference, "completion offer decision authority reference");
  requireText(value.authority.observed_at, "completion offer decision authority observed_at");
  return value;
}

export function validateSupervisorCapabilityInput(capability, operation, value) {
  const operations = CAPABILITIES[capability];
  if (!operations || !operations.includes(operation)) {
    throw new TypeError(`unsupported supervisor capability operation ${capability}/${operation}`);
  }
  requireRecord(value, `${capability}/${operation} input`);
  const [required, optional] = INPUT_FIELDS[`${capability}/${operation}`];
  exactFields(value, required, optional, `${capability}/${operation} input`);
  if (capability === "capability.preflight") {
    requireText(value.campaign_path, "campaign path");
  }
  if (capability === "capability.completion_offer" && operation === "resolve") {
    validateDecision(value.decision);
  }
  if (capability === "capability.completion_offer" && operation === "expire") {
    requireText(value.reason, "completion offer expiration reason");
  }
  if (capability === "capability.resume" && operation === "recover_terminal") {
    requireText(value.run_id, "campaign run id");
  }
  return structuredClone(value);
}

export function validateSupervisorCapabilityOutput(capability, operation, value) {
  requireRecord(value, "supervisor capability result");
  exactFields(
    value,
    new Set(["schema_version", "generation_id", "capability", "operation", "result"]),
    new Set(),
    "supervisor capability result",
  );
  if (value.schema_version !== 1 || value.capability !== capability
      || value.operation !== operation) {
    throw new TypeError("supervisor capability result binding is invalid");
  }
  requireText(value.generation_id, "supervisor capability result generation_id");
  if (value.result !== null) requireRecord(value.result, "supervisor capability result payload");
  return structuredClone(value);
}

function inputSchema(operations) {
  return {
    type: "object",
    required: ["operation", "input"],
    properties: {
      operation: { type: "string", enum: operations },
      input: { type: "object" },
    },
    additionalProperties: false,
  };
}

export function createSupervisorCampaignCapabilityDefinitions(invokeHostEffect) {
  if (typeof invokeHostEffect !== "function") {
    throw new TypeError("supervisor campaign capability definitions require a host effect client");
  }
  return new Map(Object.entries(CAPABILITIES).map(([capability, operations]) => [capability, {
    namespace: "campaign",
    name: TOOL_NAMES[capability],
    description: `Invoke the host-owned ${capability} boundary. The host validates exact operation identity, authority, and durable state.`,
    inputSchema: inputSchema(operations),
    async handler(argumentsValue) {
      requireRecord(argumentsValue, `${capability} tool arguments`);
      exactFields(
        argumentsValue,
        new Set(["operation", "input"]),
        new Set(),
        `${capability} tool arguments`,
      );
      const operation = requireText(argumentsValue.operation, `${capability} operation`);
      const input = validateSupervisorCapabilityInput(capability, operation, argumentsValue.input);
      const result = await invokeHostEffect({
        protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
        capability,
        operation,
        input,
      });
      return validateSupervisorCapabilityOutput(capability, operation, result);
    },
  }]));
}

export const supervisorCampaignCapabilityOperations = CAPABILITIES;
