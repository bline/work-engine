import { SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL } from "./host-effect-runtime.mjs";

const STRATEGIC_CONTINUITY = Object.freeze(["initialized", "retained", "reconstructed"]);
const NON_EMPTY_TEXT_SCHEMA = Object.freeze({ type: "string", pattern: ".*\\S.*" });
const strategicReconciliationToolDescription =
  "Reconcile durable campaign evidence through the host-owned strategic planner. "
  + "Use operation=reconcile with exact input fields instance_id, client_user_message_id, "
  + "strategic_objective, evidence_cutoff, canonical_references, and continuity; the schema "
  + "defines their required nested fields and rejects unknown fields. This advisory boundary "
  + "does not create a campaign or grant checkpoint, publication, pilot, or Git authority.";

function strategicReconciliationRequestInputSchema() {
  const text = () => ({ ...NON_EMPTY_TEXT_SCHEMA });
  return {
    type: "object",
    required: ["instance_id", "client_user_message_id", "strategic_objective",
      "evidence_cutoff", "canonical_references", "continuity"],
    properties: {
      instance_id: text(), client_user_message_id: text(), strategic_objective: text(),
      evidence_cutoff: {
        type: "object", required: ["roadmap_revision", "repository_revision"],
        properties: {
          roadmap_revision: text(), repository_revision: text(),
          campaign_terminals: { type: "array", items: {
            type: "object", required: ["run_id", "slice_number", "status"],
            properties: { run_id: text(), slice_number: { type: "integer", minimum: 0 },
              status: text() }, additionalProperties: false,
          } },
        }, additionalProperties: false,
      },
      canonical_references: { type: "array", minItems: 1, items: {
        type: "object", required: ["owner", "reference", "revision", "freshness_rule"],
        properties: { owner: text(), reference: text(), revision: text(), freshness_rule: text(),
          integrity_sha256: { type: "string", pattern: "^[0-9a-f]{64}$" } },
        additionalProperties: false,
      } },
      continuity: { type: "string", enum: [...STRATEGIC_CONTINUITY] },
    }, additionalProperties: false,
  };
}

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
  "capability.workspace_coordination": Object.freeze(["acquire", "inspect", "release"]),
  "capability.worktree_lifecycle": Object.freeze(["allocate", "cleanup"]),
  "capability.canonical_publication": Object.freeze([
    "prepare", "seal_validation", "promote", "reconcile",
  ]),
  "capability.completion_publication": Object.freeze(["prepare", "complete", "reconcile"]),
  "capability.strategic_reconciliation": Object.freeze(["reconcile"]),
});

const TOOL_NAMES = Object.freeze({
  "capability.preflight": "preflight",
  "capability.lifecycle_control": "lifecycle_control",
  "capability.receipt_finalization": "receipt_finalization",
  "capability.checkpoint_lifecycle": "checkpoint_lifecycle",
  "capability.completion_offer": "completion_offer",
  "capability.resume": "resume",
  "capability.workspace_coordination": "workspace_coordination",
  "capability.worktree_lifecycle": "worktree_lifecycle",
  "capability.canonical_publication": "canonical_publication",
  "capability.completion_publication": "completion_publication",
  "capability.strategic_reconciliation": "strategic_reconciliation",
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
  "capability.workspace_coordination/acquire": [
    new Set(["resource", "holder", "intent_id", "ttl_ms"]), new Set(),
  ],
  "capability.workspace_coordination/inspect": [new Set(["resource"]), new Set()],
  "capability.workspace_coordination/release": [new Set(["lease"]), new Set()],
  "capability.worktree_lifecycle/allocate": [
    new Set(["operation_id", "agent_id", "intent_id", "baseline_commit"]), new Set(["ttl_ms"]),
  ],
  "capability.worktree_lifecycle/cleanup": [new Set(["allocation"]), new Set()],
  "capability.canonical_publication/prepare": [
    new Set(["operation_id", "target_branch", "expected_parent", "checkpoint", "manifest", "authorization", "message"]), new Set(),
  ],
  "capability.canonical_publication/seal_validation": [
    new Set(["operation_id", "preparation_revision", "validation"]), new Set(),
  ],
  "capability.canonical_publication/promote": [
    new Set(["operation_id", "prepared_revision"]), new Set(["publication_ttl_ms"]),
  ],
  "capability.canonical_publication/reconcile": [
    new Set(["operation_id", "prepared_revision"]), new Set(),
  ],
  "capability.completion_publication/prepare": [
    new Set(["offer", "accepted_checkpoint"]), new Set(),
  ],
  "capability.completion_publication/complete": [
    new Set(["offer", "preparation_revision", "validation"]), new Set(),
  ],
  "capability.completion_publication/reconcile": [
    new Set(["offer", "preparation_revision"]), new Set(),
  ],
  "capability.strategic_reconciliation/reconcile": [
    new Set(["instance_id", "client_user_message_id", "strategic_objective",
      "evidence_cutoff", "canonical_references", "continuity"]), new Set(),
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
  if (capability === "capability.workspace_coordination" && operation !== "release") {
    requireRecord(value.resource, "workspace resource");
    exactFields(value.resource, new Set(["type", "id"]), new Set(), "workspace resource");
    requireText(value.resource.type, "workspace resource type");
    requireText(value.resource.id, "workspace resource id");
  }
  if (capability === "capability.workspace_coordination" && operation === "acquire") {
    requireText(value.holder, "workspace resource holder");
    requireText(value.intent_id, "workspace resource intent");
    if (!Number.isSafeInteger(value.ttl_ms) || value.ttl_ms < 1) throw new TypeError("workspace resource ttl_ms must be positive");
  }
  if (capability === "capability.worktree_lifecycle" && operation === "allocate") {
    for (const [field, label] of [["operation_id", "worktree operation"], ["agent_id", "worktree agent"], ["intent_id", "worktree intent"], ["baseline_commit", "worktree baseline"]]) requireText(value[field], label);
    if (value.ttl_ms !== undefined && (!Number.isSafeInteger(value.ttl_ms) || value.ttl_ms < 1)) throw new TypeError("worktree ttl_ms must be positive");
  }
  if (capability === "capability.canonical_publication") {
    requireText(value.operation_id, "publication operation");
    if (operation === "prepare") {
      requireText(value.target_branch, "publication target branch");
      requireText(value.expected_parent, "publication expected parent");
      requireRecord(value.checkpoint, "publication checkpoint");
      if (!Array.isArray(value.manifest)) throw new TypeError("publication manifest must be an array");
      requireRecord(value.authorization, "publication authorization");
      requireRecord(value.message, "publication message");
    } else {
      requireText(value[operation === "seal_validation" ? "preparation_revision" : "prepared_revision"], "publication revision");
      if (operation === "seal_validation") requireRecord(value.validation, "publication validation");
      if (value.publication_ttl_ms !== undefined && (!Number.isSafeInteger(value.publication_ttl_ms) || value.publication_ttl_ms < 1)) throw new TypeError("publication ttl must be positive");
    }
  }
  if (capability === "capability.completion_publication") {
    requireRecord(value.offer, "completion publication offer");
    if (operation === "prepare") requireRecord(value.accepted_checkpoint, "completion publication accepted checkpoint");
    else {
      requireText(value.preparation_revision, "completion publication preparation revision");
      if (operation === "complete") requireRecord(value.validation, "completion publication validation");
    }
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

function inputSchema(capability, operations) {
  return {
    type: "object",
    required: ["operation", "input"],
    properties: {
      operation: { type: "string", enum: operations },
      input: capability === "capability.strategic_reconciliation"
        ? strategicReconciliationRequestInputSchema()
        : { type: "object" },
    },
    additionalProperties: false,
  };
}

export function createSupervisorCampaignCapabilityDefinitions(
  invokeHostEffect,
  { executeStrategicReconciliation = null } = {},
) {
  if (typeof invokeHostEffect !== "function") {
    throw new TypeError("supervisor campaign capability definitions require a host effect client");
  }
  return new Map(Object.entries(CAPABILITIES).map(([capability, operations]) => [capability, {
    namespace: "campaign",
    name: TOOL_NAMES[capability],
    description: capability === "capability.strategic_reconciliation"
      ? strategicReconciliationToolDescription
      : `Invoke the host-owned ${capability} boundary. The host validates exact operation identity, authority, and durable state.`,
    inputSchema: inputSchema(capability, operations),
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
      if (capability === "capability.strategic_reconciliation") {
        if (typeof executeStrategicReconciliation !== "function") {
          throw new Error("strategic reconciliation executor is unavailable in this generation");
        }
        const admitted = validateSupervisorCapabilityOutput(capability, operation,
          await invokeHostEffect({
            protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
            capability, operation, input: { phase: "admit", request: input },
          }));
        const execution = await executeStrategicReconciliation(input);
        return validateSupervisorCapabilityOutput(capability, operation,
          await invokeHostEffect({
            protocol: SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL,
            capability, operation,
            input: {
              phase: "complete", admission: admitted.result.admission,
              output_text: execution.completion.outputText,
            },
          }));
      }
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
