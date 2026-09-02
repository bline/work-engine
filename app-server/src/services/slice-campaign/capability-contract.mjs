import { SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL } from "./host-effect-runtime.mjs";

const STRATEGIC_CONTINUITY = Object.freeze(["initialized", "retained", "reconstructed"]);
const NON_EMPTY_TEXT_SCHEMA = Object.freeze({ type: "string", pattern: ".*\\S.*" });
const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const strategicReconciliationToolDescription =
  "Reconcile durable campaign evidence through the host-owned strategic planner. "
  + "Use operation=reconcile with exact input fields instance_id, client_user_message_id, "
  + "strategic_objective, evidence_cutoff, canonical_references, and continuity; the schema "
  + "defines their required nested fields and rejects unknown fields. This advisory boundary "
  + "does not create a campaign or grant checkpoint, publication, pilot, or Git authority.";
const operationalCoordinationToolDescription =
  "Read, post, claim, or release advisory cross-session information through the canonical chatboard. "
  + "A chatboard claim grants no mutation or workflow authority and is not a fenced capability.workspace_coordination lease. "
  + "Treating it as a lease can admit conflicting or stale mutations; acquire the authoritative workspace lease and mutation admission separately whenever they are required.";
const nativeReviewToolDescription =
  "Execute, recover, or explicitly retry an exact supervisor-selected review obligation through the stable host-owned native Claude Code closure. "
  + "The host derives reviewer authority, subject delivery, session identity, command, tools, provider route, durable episode, and finding state. "
  + "Retry is admitted only from host-verified definite pre-provider authentication failure: it resumes an existing exact retained session, or reuses the exact deterministic UUID when failure occurred before any process or session existed; ambiguous outcomes remain non-replayable. "
  + "This capability grants no shell, filesystem, credential, model-routing, reviewer-selection, finding-evaluation, review-acceptance, or campaign-acceptance authority.";

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

function operationalCoordinationToolInputSchema() {
  const text = () => ({ ...NON_EMPTY_TEXT_SCHEMA });
  const uuid = () => ({type: "string", pattern: CANONICAL_UUID.source});
  const inputs = {
    read: {since: {type: "integer", minimum: 0}, limit: {type: "integer", minimum: 1, maximum: 500}},
    claim: {resource: text(), author: text(), session_id: uuid(), claim_id: uuid(),
      ttl_seconds: {type: "integer", minimum: 1}, note: text()},
    post: {author: text(), session_id: uuid(), topic: text(), body: text(),
      references: {type: "array", items: text()}, message_id: uuid()},
    release: {resource: text(), session_id: uuid(), claim_id: uuid()},
  };
  return {oneOf: Object.entries(inputs).map(([operation, properties]) => ({
    type: "object", required: ["operation", "input"], properties: {
      operation: {type: "string", enum: [operation]},
      input: {type: "object", required: Object.keys(properties), properties,
        additionalProperties: false},
    }, additionalProperties: false,
  }))};
}

function nativeReviewToolInputSchema() {
  const text = () => ({...NON_EMPTY_TEXT_SCHEMA});
  const identity = {type: "object", required: ["runId", "sliceNumber", "attemptId", "planVersion"],
    properties: {runId: text(), sliceNumber: {type: "integer", minimum: 0}, attemptId: text(), planVersion: text()},
    additionalProperties: false};
  const common = {identity, expected_revision: {type: "string", pattern: "^[0-9a-f]{64}$"},
    obligation_id: text(), operation_id: text()};
  const inputs = {
    execute: common,
    recover: {identity, obligation_id: text()},
    retry: common,
    record_finding_evaluation: {...common, finding_id: text(), consumer_revision: text()},
    execute_remediation: {...common, remediation_subject: {type: "object",
      required: ["commit", "tree", "patchIdentity"], properties: {
        commit: text(), tree: text(), patchIdentity: text(),
      }, additionalProperties: false}},
  };
  return {oneOf: Object.entries(inputs).map(([operation, properties]) => ({
    type: "object", required: ["operation", "input"], properties: {
      operation: {type: "string", enum: [operation]}, input: {type: "object",
        required: Object.keys(properties), properties, additionalProperties: false},
    }, additionalProperties: false,
  }))};
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
  "capability.operational_coordination": Object.freeze(["read", "claim", "post", "release"]),
  "capability.native_review": Object.freeze([
    "execute", "recover", "retry", "record_finding_evaluation", "execute_remediation",
  ]),
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
  "capability.operational_coordination": "operational_coordination",
  "capability.native_review": "native_review",
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
  "capability.operational_coordination/read": [new Set(["since", "limit"]), new Set()],
  "capability.operational_coordination/claim": [
    new Set(["resource", "author", "session_id", "claim_id", "ttl_seconds", "note"]), new Set(),
  ],
  "capability.operational_coordination/post": [
    new Set(["author", "session_id", "topic", "body", "references", "message_id"]), new Set(),
  ],
  "capability.operational_coordination/release": [
    new Set(["resource", "session_id", "claim_id"]), new Set(),
  ],
  "capability.native_review/execute": [
    new Set(["identity", "expected_revision", "obligation_id", "operation_id"]), new Set(),
  ],
  "capability.native_review/recover": [
    new Set(["identity", "obligation_id"]), new Set(),
  ],
  "capability.native_review/retry": [
    new Set(["identity", "expected_revision", "obligation_id", "operation_id"]), new Set(),
  ],
  "capability.native_review/record_finding_evaluation": [
    new Set(["identity", "expected_revision", "obligation_id", "operation_id", "finding_id", "consumer_revision"]), new Set(),
  ],
  "capability.native_review/execute_remediation": [
    new Set(["identity", "expected_revision", "obligation_id", "operation_id", "remediation_subject"]), new Set(),
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

function validateOperationalCoordination(operation, value) {
  if (operation === "read") {
    if (!Number.isSafeInteger(value.since) || value.since < 0) {
      throw new TypeError("operational coordination since must be a nonnegative safe integer");
    }
    if (!Number.isSafeInteger(value.limit) || value.limit < 1 || value.limit > 500) {
      throw new TypeError("operational coordination limit must be a safe integer from 1 to 500");
    }
    return;
  }
  const textFields = operation === "claim"
    ? ["resource", "author", "session_id", "claim_id", "note"]
    : operation === "post"
      ? ["author", "session_id", "topic", "body", "message_id"]
      : ["resource", "session_id", "claim_id"];
  for (const field of textFields) requireText(value[field], `operational coordination ${field}`);
  for (const field of ["session_id", operation === "post" ? "message_id" : "claim_id"]) {
    if (!CANONICAL_UUID.test(value[field])) {
      throw new TypeError(`operational coordination ${field} must use canonical lowercase UUID syntax`);
    }
  }
  if (operation === "claim"
      && (!Number.isSafeInteger(value.ttl_seconds) || value.ttl_seconds < 1)) {
    throw new TypeError("operational coordination ttl_seconds must be a positive safe integer");
  }
  if (operation === "post"
      && (!Array.isArray(value.references)
        || value.references.some((reference) => typeof reference !== "string" || !reference.trim()))) {
    throw new TypeError("operational coordination references must be an array of nonempty strings");
  }
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
  if (capability === "capability.operational_coordination") {
    validateOperationalCoordination(operation, value);
  }
  if (capability === "capability.native_review") {
    requireRecord(value.identity, "native review campaign identity");
    if (operation !== "recover") {
      requireText(value.expected_revision, "native review expected revision");
      requireText(value.operation_id, "native review operation identity");
    }
    requireText(value.obligation_id, "native review obligation identity");
    if (operation === "record_finding_evaluation") {
      requireText(value.finding_id, "native review finding identity");
      requireText(value.consumer_revision, "native review consumer revision");
    }
    if (operation === "execute_remediation") {
      requireRecord(value.remediation_subject, "native review remediation subject");
      exactFields(value.remediation_subject, new Set(["commit", "tree", "patchIdentity"]), new Set(), "native review remediation subject");
      for (const field of ["commit", "tree", "patchIdentity"]) requireText(value.remediation_subject[field], `native review remediation ${field}`);
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
  if (capability === "capability.operational_coordination") {
    return operationalCoordinationToolInputSchema();
  }
  if (capability === "capability.native_review") return nativeReviewToolInputSchema();
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
      : capability === "capability.operational_coordination"
        ? operationalCoordinationToolDescription
        : capability === "capability.native_review"
          ? nativeReviewToolDescription
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
