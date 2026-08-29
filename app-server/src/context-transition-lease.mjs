import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

import {
  appendLifecycleLedgerEntry,
  verifyLifecycleLedgerEntry,
} from "./context-lifecycle-ledger.mjs";
import { PINNED_PROTOCOL } from "./capabilities.mjs";
import { normalizeCodexLifecycleNotification } from "./codex-lifecycle-notification-source.mjs";

export const CONTEXT_TRANSITION_LEASE_SCHEMA_VERSION = 1;
export const CONTEXT_TRANSITION_LEASE_TYPE = "work-engine.context-transition-lease";
export const CONTEXT_TRANSITION_PREPARATION_SCHEMA_VERSION = 1;
export const CONTEXT_TRANSITION_PREPARATION_TYPE = "work-engine.context-transition-preparation";
export const CONTEXT_WINDOW_IDENTITY_RECEIPT_SCHEMA_VERSION = 1;
export const CONTEXT_WINDOW_IDENTITY_RECEIPT_TYPE = "work-engine.context-window-identity-receipt";
export const CONTEXT_RECONCILIATION_RECEIPT_SCHEMA_VERSION = 1;
export const CONTEXT_RECONCILIATION_RECEIPT_TYPE = "work-engine.context-reconciliation-receipt";

const SHA_REVISION = /^sha256:[a-f0-9]{64}$/;

export class ContextTransitionLeaseError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = "ContextTransitionLeaseError";
    this.code = code;
    this.details = Object.freeze({ ...details });
  }
}

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be non-empty`);
  }
  return value;
}

function shaRevision(value, label) {
  text(value, label);
  if (!SHA_REVISION.test(value)) throw new TypeError(`${label} must be SHA-256 bound`);
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

function digest(value) {
  return createHash("sha256").update(value, "utf8").digest("hex");
}

const RUNTIME_ENVIRONMENT_REFERENCE = /^work-engine\.runtime-role-environment\/v1\/([^/]+)\/([a-f0-9]{64})\/(none|[a-f0-9]{64})\/(.+)$/;

async function verifyRehydrationEnvironment({ publication, role, skills }) {
  record(role, "rehydration role environment");
  if (!Array.isArray(skills)) throw new TypeError("rehydration skills must be an array");
  const environment = record(
    publication.continuationState?.governingEnvironment,
    "checkpoint governing environment",
  );
  const roleContract = record(environment.roleContract, "checkpoint role contract");
  const match = text(roleContract.reference, "checkpoint role contract reference")
    .match(RUNTIME_ENVIRONMENT_REFERENCE);
  if (!match) {
    if (role.compiledSkillSha256 !== null) {
      throw new TypeError("compiled checkpoint role contract is not runtime-environment bound");
    }
    return;
  }
  const [, encodedRole, environmentRevision, satisfactionSha256, encodedContractPath] = match;
  const logicalRoleInstanceId = decodeURIComponent(encodedRole);
  const contractPath = decodeURIComponent(encodedContractPath);
  if (role.logicalRoleInstanceId !== logicalRoleInstanceId
      || role.runtimeEnvironmentRevision !== environmentRevision) {
    throw new TypeError("successor role does not match the checkpointed runtime environment");
  }
  if ((role.compiledSkillSha256 === null) !== (satisfactionSha256 === "none")) {
    throw new TypeError("successor role runtime satisfaction provenance is incompatible");
  }
  const projectedSkills = await Promise.all(skills.map(async (skill, index) => {
    record(skill, `rehydration skill ${index}`);
    const skillPath = text(skill.path, `rehydration skill ${index} path`);
    return { reference: skillPath, sha256: digest(await readFile(skillPath, "utf8")) };
  }));
  const expectedSkills = environment.activatedSkills ?? [];
  if (!Array.isArray(expectedSkills)
      || canonical(projectedSkills.map((value) => canonical(value)).sort())
        !== canonical(expectedSkills.map((value) => canonical(value)).sort())) {
    throw new TypeError("successor activated skills do not match the checkpointed environment");
  }
  const contract = projectedSkills.find(({ reference }) => reference === contractPath);
  if (!contract) throw new TypeError("successor environment omits the checkpointed role contract");
  const content = canonical({
    schema_version: 1,
    logical_role_instance_id: logicalRoleInstanceId,
    runtime_environment_revision: environmentRevision,
    compiled_skill_sha256: role.compiledSkillSha256,
    runtime_satisfaction_sha256: satisfactionSha256,
    developer_instructions: role.developerInstructions,
    role_contract: { path: contractPath, sha256: contract.sha256 },
  });
  if (roleContract.sha256 !== digest(content)) {
    throw new TypeError("successor governing instructions differ from the checkpointed environment");
  }
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function timestamp(value, label) {
  text(value, label);
  if (Number.isNaN(Date.parse(value))) throw new TypeError(`${label} must be an ISO timestamp`);
  return value;
}

function normalizeFence(value) {
  record(value, "transition lease fence");
  if (!Number.isSafeInteger(value.bindingRevision) || value.bindingRevision < 1) {
    throw new TypeError("transition lease bindingRevision must be positive");
  }
  return freeze({
    logicalRoleInstanceId: text(value.logicalRoleInstanceId, "transition lease role"),
    threadId: text(value.threadId, "transition lease thread"),
    predecessorContextWindowId: text(
      value.predecessorContextWindowId,
      "transition lease predecessor context window id",
    ),
    bindingRevision: value.bindingRevision,
    sourceRevision: shaRevision(value.sourceRevision, "transition lease source revision"),
    authorityRevision: shaRevision(value.authorityRevision, "transition lease authority revision"),
    publicationRevision: shaRevision(
      value.publicationRevision,
      "transition lease publication revision",
    ),
    ledgerRevision: shaRevision(value.ledgerRevision, "transition lease ledger revision"),
  });
}

function equalFence(left, right) {
  return Object.keys(left).every((field) => left[field] === right[field]);
}

function equalCanonical(left, right) {
  return canonical(left) === canonical(right);
}

function verifyPublication(publication, fence) {
  record(publication, "transition lease checkpoint publication");
  const { checkpointRevision, ...body } = publication;
  if (publication.schemaVersion !== 1
      || publication.type !== "work-engine.context-checkpoint"
      || revision(body) !== checkpointRevision
      || checkpointRevision !== fence.publicationRevision) {
    throw new TypeError("transition lease checkpoint publication is invalid");
  }
  const subject = record(publication.subject, "transition lease checkpoint subject");
  for (const [subjectField, fenceField] of [
    ["logicalRoleInstanceId", "logicalRoleInstanceId"],
    ["threadId", "threadId"],
    ["bindingRevision", "bindingRevision"],
    ["sourceRevision", "sourceRevision"],
    ["authorityRevision", "authorityRevision"],
  ]) {
    if (subject[subjectField] !== fence[fenceField]) {
      throw new TypeError(`transition lease checkpoint subject mismatches ${subjectField}`);
    }
  }
}

function verifyPublicationLedger(ledgerEntry, previousLedgerEntry, fence) {
  if (!verifyLifecycleLedgerEntry(ledgerEntry, previousLedgerEntry)
      || ledgerEntry.entryRevision !== fence.ledgerRevision
      || ledgerEntry.eventType !== "checkpoint_published"
      || ledgerEntry.status !== "observed"
      || ledgerEntry.subject.logicalRoleInstanceId !== fence.logicalRoleInstanceId
      || ledgerEntry.subject.threadId !== fence.threadId
      || ledgerEntry.subject.bindingRevision !== fence.bindingRevision
      || ledgerEntry.details.checkpointRevision !== fence.publicationRevision) {
    throw new TypeError("transition lease checkpoint ledger evidence is invalid");
  }
}

function subjectFrom(fence) {
  return {
    logicalRoleInstanceId: fence.logicalRoleInstanceId,
    threadId: fence.threadId,
    bindingRevision: fence.bindingRevision,
  };
}

function appendEvent(previous, fence, now, eventType, status, evidenceRefs, details) {
  return appendLifecycleLedgerEntry(previous, {
    eventType,
    status,
    recordedAt: timestamp(now(), "transition lease timestamp"),
    subject: subjectFrom(fence),
    evidenceRefs,
    details,
  });
}

export function compileContextRetirementDirective(lease) {
  verifyContextTransitionLease(lease);
  return [
    "Context lifecycle control directive: context_retirement_ready.",
    `Transition lease: ${lease.leaseRevision}`,
    `Checkpoint: ${lease.subject.checkpointRevision}`,
    `Observed source: ${lease.subject.sourceRevision}`,
    "Context replacement changes runtime memory; it does not decide or complete human intent.",
    "Replacing context after later input could discard unresolved human interaction or let a successor continue as though that interaction were settled.",
    "Perform no domain work and call no tool except new_context.",
    "If this is still the latest input and you have observed no later human input or domain work, invoke new_context now as your only action.",
    "If you have observed later human input or domain work, do not invoke new_context and do not perform domain work.",
  ].join("\n");
}

export function verifyContextTransitionPreparation(value) {
  try {
    record(value, "context transition preparation");
    const { preparationRevision, ...body } = value;
    if (value.schemaVersion !== CONTEXT_TRANSITION_PREPARATION_SCHEMA_VERSION
        || value.type !== CONTEXT_TRANSITION_PREPARATION_TYPE
        || revision(body) !== preparationRevision) return false;
    const subject = record(value.subject, "context transition preparation subject");
    text(subject.logicalRoleInstanceId, "context transition preparation role");
    text(subject.threadId, "context transition preparation thread");
    if (!Number.isSafeInteger(subject.bindingRevision) || subject.bindingRevision < 1) return false;
    timestamp(value.preparedAt, "context transition preparation timestamp");
    return true;
  } catch {
    return false;
  }
}

export function compileContextWindowIdentityDirective(preparation) {
  if (!verifyContextTransitionPreparation(preparation)) {
    throw new TypeError("context-window identity directive requires an integrity-valid preparation");
  }
  return [
    "Context lifecycle control directive: attest_context_window_identity.",
    "Perform no domain work and invoke no tools.",
    "Return exactly one compact JSON object and no Markdown with these fields:",
    `{"schema_version":1,"type":"${CONTEXT_WINDOW_IDENTITY_RECEIPT_TYPE}","preparation_revision":"${preparation.preparationRevision}","logical_role_instance_id":"${preparation.subject.logicalRoleInstanceId}","thread_id":"${preparation.subject.threadId}","first_context_window_id":"<runtime value>","current_context_window_id":"<runtime value>","previous_context_window_id":"<runtime value or null>"}`,
    "Copy the runtime context-window values exactly. Use JSON null only when previous_context_window_id is absent.",
  ].join("\n");
}

const IDENTITY_RECEIPT_FIELDS = [
  "schema_version", "type", "preparation_revision", "logical_role_instance_id",
  "thread_id", "first_context_window_id", "current_context_window_id",
  "previous_context_window_id",
];

export function validateContextWindowIdentityReceipt(outputText, { preparation }) {
  if (!verifyContextTransitionPreparation(preparation)) {
    throw new TypeError("context-window identity validation requires an integrity-valid preparation");
  }
  const reasons = [];
  let receipt;
  try {
    receipt = JSON.parse(text(outputText, "context-window identity receipt"));
    record(receipt, "context-window identity receipt");
  } catch {
    return freeze({ status: "unresolved", reasons: ["malformed_receipt"], receipt: null });
  }
  if (!equalCanonical(Object.keys(receipt).sort(), [...IDENTITY_RECEIPT_FIELDS].sort())) {
    reasons.push("unsupported_receipt_shape");
  }
  for (const [field, expected] of [
    ["schema_version", CONTEXT_WINDOW_IDENTITY_RECEIPT_SCHEMA_VERSION],
    ["type", CONTEXT_WINDOW_IDENTITY_RECEIPT_TYPE],
    ["preparation_revision", preparation.preparationRevision],
    ["logical_role_instance_id", preparation.subject.logicalRoleInstanceId],
    ["thread_id", preparation.subject.threadId],
  ]) {
    if (receipt[field] !== expected) reasons.push(`mismatched_${field}`);
  }
  for (const field of ["first_context_window_id", "current_context_window_id"]) {
    if (typeof receipt[field] !== "string" || receipt[field].trim() === "") {
      reasons.push(`missing_${field}`);
    }
  }
  if (receipt.previous_context_window_id !== null
      && (typeof receipt.previous_context_window_id !== "string"
        || receipt.previous_context_window_id.trim() === "")) {
    reasons.push("invalid_previous_context_window_id");
  }
  return freeze({
    status: reasons.length === 0 ? "accepted" : "unresolved",
    reasons: [...new Set(reasons)],
    receipt: freeze(receipt),
  });
}

export function compileContextReconciliationDirective(lease) {
  verifyContextTransitionLease(lease);
  return [
    "Context lifecycle control directive: reconcile_context_checkpoint.",
    "Perform no domain work and invoke no tools.",
    "Read the exact work-engine.context.checkpoint and work-engine.context.reconciliation-challenge request-context entries.",
    "Reconcile the checkpoint against the governing environment and canonical references that are actually available in this context.",
    "A successful provider transition alone does not establish semantic continuation safety.",
    "If the governing environment, authority, commitments, or uncertainty cannot be established, claiming success could resume under stale, substituted, or incomplete instructions or silently settle unresolved human interaction.",
    "Do not infer missing authority, close an interaction, or represent uncertainty as success.",
    "Return exactly one compact JSON object and no Markdown with these fields:",
    '{"schema_version":1,"type":"work-engine.context-reconciliation-receipt","lease_revision":"<challenge value>","checkpoint_revision":"<challenge value>","logical_role_instance_id":"<challenge value>","thread_id":"<challenge value>","receipt_nonce":"<challenge value>","first_context_window_id":"<runtime value>","current_context_window_id":"<runtime value>","previous_context_window_id":"<runtime value or null>","checkpoint_loaded":true,"governing_environment_applicable":true,"authority_reconciled":true,"open_commitments_preserved":true,"authorized_next_action_valid":true,"uncertainty":[]}',
    "Copy challenge and runtime context-window values exactly. Set a Boolean false and name the reason in uncertainty whenever its claim is not established.",
  ].join("\n");
}

function verifyCheckpointPublication(publication, lease) {
  verifyPublication(publication, {
    logicalRoleInstanceId: lease.subject.logicalRoleInstanceId,
    threadId: lease.subject.threadId,
    bindingRevision: lease.subject.bindingRevision,
    sourceRevision: lease.subject.sourceRevision,
    authorityRevision: lease.subject.authorityRevision,
    publicationRevision: lease.subject.checkpointRevision,
  });
}

function reconciliationChallenge({ lease, receiptNonce }) {
  return freeze({
    schema_version: 1,
    lease_revision: lease.leaseRevision,
    checkpoint_revision: lease.subject.checkpointRevision,
    logical_role_instance_id: lease.subject.logicalRoleInstanceId,
    thread_id: lease.subject.threadId,
    predecessor_context_window_id: lease.subject.predecessorContextWindowId,
    receipt_nonce: text(receiptNonce, "reconciliation receipt nonce"),
  });
}

export function buildContextRehydrationRequest({
  lease,
  publication,
  receiptNonce,
}) {
  if (!verifyContextTransitionLease(lease)) {
    throw new TypeError("context rehydration requires an integrity-valid lease");
  }
  verifyCheckpointPublication(publication, lease);
  const challenge = reconciliationChallenge({ lease, receiptNonce });
  return freeze({
    text: compileContextReconciliationDirective(lease),
    requestContext: {
      "work-engine.context.checkpoint": {
        kind: "lifecycle-checkpoint",
        value: canonical(publication),
      },
      "work-engine.context.reconciliation-challenge": {
        kind: "lifecycle-control-data",
        value: canonical(challenge),
      },
    },
    challenge,
  });
}

const RECEIPT_FIELDS = [
  "schema_version", "type", "lease_revision", "checkpoint_revision",
  "logical_role_instance_id", "thread_id", "receipt_nonce",
  "first_context_window_id", "current_context_window_id",
  "previous_context_window_id", "checkpoint_loaded",
  "governing_environment_applicable", "authority_reconciled",
  "open_commitments_preserved", "authorized_next_action_valid", "uncertainty",
];

export function validateContextReconciliationReceipt(outputText, { challenge }) {
  const reasons = [];
  let receipt;
  try {
    receipt = JSON.parse(text(outputText, "context reconciliation receipt"));
    record(receipt, "context reconciliation receipt");
  } catch {
    return freeze({ status: "unresolved", reasons: ["malformed_receipt"], receipt: null });
  }
  const fields = Object.keys(receipt).sort();
  if (!equalCanonical(fields, [...RECEIPT_FIELDS].sort())) reasons.push("unsupported_receipt_shape");
  for (const [field, expected] of [
    ["schema_version", CONTEXT_RECONCILIATION_RECEIPT_SCHEMA_VERSION],
    ["type", CONTEXT_RECONCILIATION_RECEIPT_TYPE],
    ["lease_revision", challenge.lease_revision],
    ["checkpoint_revision", challenge.checkpoint_revision],
    ["logical_role_instance_id", challenge.logical_role_instance_id],
    ["thread_id", challenge.thread_id],
    ["receipt_nonce", challenge.receipt_nonce],
  ]) {
    if (receipt[field] !== expected) reasons.push(`mismatched_${field}`);
  }
  for (const field of ["first_context_window_id", "current_context_window_id"]) {
    if (typeof receipt[field] !== "string" || receipt[field].trim() === "") {
      reasons.push(`missing_${field}`);
    }
  }
  if (receipt.previous_context_window_id !== challenge.predecessor_context_window_id) {
    reasons.push("mismatched_previous_context_window_id");
  }
  if (receipt.current_context_window_id === challenge.predecessor_context_window_id) {
    reasons.push("context_window_not_replaced");
  }
  for (const field of [
    "checkpoint_loaded", "governing_environment_applicable", "authority_reconciled",
    "open_commitments_preserved", "authorized_next_action_valid",
  ]) {
    if (receipt[field] !== true) reasons.push(`unestablished_${field}`);
  }
  if (!Array.isArray(receipt.uncertainty)
      || receipt.uncertainty.some((value) => typeof value !== "string" || value.trim() === "")) {
    reasons.push("invalid_uncertainty");
  } else if (receipt.uncertainty.length > 0) {
    reasons.push("reported_uncertainty");
  }
  return freeze({
    status: reasons.length === 0 ? "accepted" : "unresolved",
    reasons: [...new Set(reasons)],
    receipt: freeze(receipt),
  });
}

export function verifyContextTransitionLease(value) {
  try {
    record(value, "context transition lease");
    const { leaseRevision, ...body } = value;
    if (value.schemaVersion !== CONTEXT_TRANSITION_LEASE_SCHEMA_VERSION
        || value.type !== CONTEXT_TRANSITION_LEASE_TYPE
        || revision(body) !== leaseRevision) return false;
    const subject = record(value.subject, "context transition lease subject");
    text(subject.logicalRoleInstanceId, "context transition lease role");
    text(subject.threadId, "context transition lease thread");
    text(subject.predecessorContextWindowId, "context transition lease predecessor context window id");
    if (!Number.isSafeInteger(subject.bindingRevision) || subject.bindingRevision < 1) return false;
    for (const field of [
      "sourceRevision", "authorityRevision", "checkpointRevision",
      "publicationLedgerRevision", "readinessLedgerRevision",
    ]) shaRevision(subject[field], `context transition lease ${field}`);
    if (subject.preparedContextRevision != null) {
      shaRevision(
        subject.preparedContextRevision,
        "context transition lease prepared context revision",
      );
    }
    timestamp(value.acquiredAt, "context transition lease acquiredAt");
    return true;
  } catch {
    return false;
  }
}

export class InMemoryContextTransitionLeaseGate {
  constructor({ now = () => new Date().toISOString() } = {}) {
    this.now = now;
    this.states = new Map();
    this.threadRoles = new Map();
    this.tails = new Map();
    this.activePermits = new WeakSet();
  }

  #withRoleLock(logicalRoleInstanceId, operation) {
    const prior = this.tails.get(logicalRoleInstanceId) ?? Promise.resolve();
    const current = prior.then(operation, operation);
    this.tails.set(logicalRoleInstanceId, current.catch(() => {}));
    return current;
  }

  beginPreparation(
    { logicalRoleInstanceId, threadId, bindingRevision },
    { closeAdmission = null } = {},
  ) {
    text(logicalRoleInstanceId, "context transition preparation role");
    text(threadId, "context transition preparation thread");
    if (!Number.isSafeInteger(bindingRevision) || bindingRevision < 1) {
      throw new TypeError("context transition preparation bindingRevision must be positive");
    }
    if (closeAdmission !== null && typeof closeAdmission !== "function") {
      throw new TypeError("context transition admission closer must be a function or null");
    }
    return this.#withRoleLock(logicalRoleInstanceId, async () => {
      if (this.states.has(logicalRoleInstanceId)) {
        throw new ContextTransitionLeaseError(
          "preparation_conflict",
          "a context transition subject already exists for this role",
        );
      }
      const body = {
        schemaVersion: CONTEXT_TRANSITION_PREPARATION_SCHEMA_VERSION,
        type: CONTEXT_TRANSITION_PREPARATION_TYPE,
        subject: { logicalRoleInstanceId, threadId, bindingRevision },
        preparedAt: timestamp(this.now(), "context transition preparation timestamp"),
      };
      const preparation = freeze({ ...body, preparationRevision: revision(body) });
      this.states.set(logicalRoleInstanceId, {
        phase: "preparing",
        preparation,
        identityAttestation: null,
        fence: null,
        lease: null,
        publication: null,
        ledgerEntry: null,
        delivery: null,
        rehydration: null,
        revocationReason: null,
      });
      this.threadRoles.set(threadId, logicalRoleInstanceId);
      try {
        if (closeAdmission) await closeAdmission(preparation);
      } catch (error) {
        this.states.delete(logicalRoleInstanceId);
        this.threadRoles.delete(threadId);
        throw error;
      }
      return freeze({ status: "preparing", preparation });
    });
  }

  acquire({ publication, ledgerEntry, previousLedgerEntry = null, expectedFence }) {
    const fence = normalizeFence(expectedFence);
    return this.#withRoleLock(fence.logicalRoleInstanceId, () => {
      if (this.states.has(fence.logicalRoleInstanceId)) {
        throw new ContextTransitionLeaseError(
          "lease_conflict",
          "a transition lease subject already exists for this role",
        );
      }
      verifyPublication(publication, fence);
      verifyPublicationLedger(ledgerEntry, previousLedgerEntry, fence);
      const readinessEntry = appendEvent(
        ledgerEntry,
        fence,
        this.now,
        "readiness_recorded",
        "accepted",
        [fence.sourceRevision, fence.authorityRevision, fence.publicationRevision],
        { checkpointRevision: fence.publicationRevision },
      );
      const body = {
        schemaVersion: CONTEXT_TRANSITION_LEASE_SCHEMA_VERSION,
        type: CONTEXT_TRANSITION_LEASE_TYPE,
        subject: {
          logicalRoleInstanceId: fence.logicalRoleInstanceId,
          threadId: fence.threadId,
          predecessorContextWindowId: fence.predecessorContextWindowId,
          bindingRevision: fence.bindingRevision,
          sourceRevision: fence.sourceRevision,
          authorityRevision: fence.authorityRevision,
          checkpointRevision: fence.publicationRevision,
          publicationLedgerRevision: fence.ledgerRevision,
          readinessLedgerRevision: readinessEntry.entryRevision,
          preparedContextRevision: null,
        },
        acquiredAt: readinessEntry.recordedAt,
      };
      const lease = freeze({ ...body, leaseRevision: revision(body) });
      const currentFence = freeze({ ...fence, ledgerRevision: readinessEntry.entryRevision });
      this.states.set(fence.logicalRoleInstanceId, {
        phase: "ready",
        fence: currentFence,
        lease,
        publication,
        ledgerEntry: readinessEntry,
        delivery: null,
        rehydration: null,
        revocationReason: null,
      });
      this.threadRoles.set(fence.threadId, fence.logicalRoleInstanceId);
      return freeze({ status: "acquired", lease, ledgerEntry: readinessEntry, currentFence });
    });
  }

  promotePreparation({
    preparation,
    publication,
    ledgerEntry,
    previousLedgerEntry = null,
    expectedFence,
  }) {
    if (!verifyContextTransitionPreparation(preparation)) {
      throw new TypeError("transition promotion requires an integrity-valid preparation");
    }
    const fence = normalizeFence(expectedFence);
    return this.#withRoleLock(preparation.subject.logicalRoleInstanceId, () => {
      const state = this.states.get(preparation.subject.logicalRoleInstanceId);
      if (!state || state.phase !== "identity_attested"
          || state.preparation.preparationRevision !== preparation.preparationRevision) {
        throw new ContextTransitionLeaseError(
          "invalid_preparation_phase",
          "transition promotion requires the active identity-attested preparation",
        );
      }
      for (const field of ["logicalRoleInstanceId", "threadId", "bindingRevision"]) {
        if (fence[field] !== preparation.subject[field]) {
          throw new ContextTransitionLeaseError(
            "preparation_fence_mismatch",
            `transition promotion fence mismatches ${field}`,
          );
        }
      }
      if (fence.predecessorContextWindowId
          !== state.identityAttestation.receipt.current_context_window_id) {
        throw new ContextTransitionLeaseError(
          "predecessor_identity_mismatch",
          "transition promotion predecessor does not match the fenced target-model attestation",
        );
      }
      verifyPublication(publication, fence);
      verifyPublicationLedger(ledgerEntry, previousLedgerEntry, fence);
      const readinessEntry = appendEvent(
        ledgerEntry,
        fence,
        this.now,
        "readiness_recorded",
        "accepted",
        [
          preparation.preparationRevision,
          fence.sourceRevision,
          fence.authorityRevision,
          fence.publicationRevision,
        ],
        {
          checkpointRevision: fence.publicationRevision,
          predecessorContextWindowId: fence.predecessorContextWindowId,
        },
      );
      const body = {
        schemaVersion: CONTEXT_TRANSITION_LEASE_SCHEMA_VERSION,
        type: CONTEXT_TRANSITION_LEASE_TYPE,
        subject: {
          logicalRoleInstanceId: fence.logicalRoleInstanceId,
          threadId: fence.threadId,
          predecessorContextWindowId: fence.predecessorContextWindowId,
          bindingRevision: fence.bindingRevision,
          sourceRevision: fence.sourceRevision,
          authorityRevision: fence.authorityRevision,
          checkpointRevision: fence.publicationRevision,
          publicationLedgerRevision: fence.ledgerRevision,
          readinessLedgerRevision: readinessEntry.entryRevision,
          preparedContextRevision: state.contextSnapshot.contextRevision,
        },
        acquiredAt: readinessEntry.recordedAt,
      };
      const lease = freeze({ ...body, leaseRevision: revision(body) });
      const currentFence = freeze({ ...fence, ledgerRevision: readinessEntry.entryRevision });
      Object.assign(state, {
        phase: "ready",
        fence: currentFence,
        lease,
        publication,
        ledgerEntry: readinessEntry,
      });
      return freeze({ status: "acquired", lease, ledgerEntry: readinessEntry, currentFence });
    });
  }

  snapshot(logicalRoleInstanceId) {
    const state = this.states.get(logicalRoleInstanceId);
    if (!state) return null;
    return freeze({
      phase: state.phase,
      preparation: state.preparation ?? null,
      identityAttestation: state.identityAttestation ?? null,
      contextSnapshot: state.contextSnapshot ?? null,
      pendingContextSnapshot: state.pendingContextSnapshot ?? null,
      fence: state.fence === null ? null : { ...state.fence },
      lease: state.lease,
      publication: state.publication,
      ledgerEntry: state.ledgerEntry,
      delivery: state.delivery,
      rehydration: state.rehydration,
      revocationReason: state.revocationReason,
    });
  }

  #revokeReady(state, reason, evidenceRefs = []) {
    const entry = appendEvent(
      state.ledgerEntry,
      state.fence,
      this.now,
      "readiness_recorded",
      "rejected",
      [state.lease.leaseRevision, ...evidenceRefs],
      { reason },
    );
    state.phase = "revoked";
    state.ledgerEntry = entry;
    state.fence = freeze({ ...state.fence, ledgerRevision: entry.entryRevision });
    state.revocationReason = reason;
  }

  async #deliverWithPermit(logicalRoleInstanceId, deliver) {
    const permit = freeze({ logicalRoleInstanceId });
    this.activePermits.add(permit);
    try {
      return await deliver(permit);
    } finally {
      this.activePermits.delete(permit);
    }
  }

  runTurnAdmission({
    logicalRoleInstanceId,
    text: turnText,
    transitionLease = null,
    transitionPreparation = null,
    skills = [],
    toolBridge = null,
    requestContext = null,
  }, deliver) {
    text(logicalRoleInstanceId, "turn admission role");
    if (typeof deliver !== "function") throw new TypeError("turn admission requires delivery");
    return this.#withRoleLock(logicalRoleInstanceId, async () => {
      const state = this.states.get(logicalRoleInstanceId);
      if (transitionLease && transitionPreparation) {
        throw new ContextTransitionLeaseError(
          "ambiguous_transition_control",
          "a turn cannot use both transition preparation and transition lease authority",
        );
      }
      if (transitionPreparation) {
        if (!state || state.phase !== "preparing"
            || !verifyContextTransitionPreparation(transitionPreparation)
            || transitionPreparation.preparationRevision
              !== state.preparation.preparationRevision) {
          throw new ContextTransitionLeaseError(
            "invalid_transition_preparation",
            "identity attestation requires the active transition preparation",
          );
        }
        if (skills.length !== 0 || toolBridge !== null || requestContext !== null
            || turnText !== compileContextWindowIdentityDirective(state.preparation)) {
          state.phase = "preparation_failed";
          throw new ContextTransitionLeaseError(
            "non_sterile_identity_attestation",
            "identity attestation must contain only the exact lifecycle directive",
          );
        }
        state.phase = "identity_delivering";
        try {
          const delivery = await this.#deliverWithPermit(logicalRoleInstanceId, deliver);
          state.phase = "identity_requested";
          state.delivery = freeze({
            threadId: text(delivery.threadId, "identity attestation delivery thread"),
            turnId: text(delivery.turnId, "identity attestation delivery turn"),
          });
          return delivery;
        } catch (error) {
          state.phase = "preparation_failed";
          throw error;
        }
      }
      if (!transitionLease) {
        if (state?.phase === "ready") this.#revokeReady(state, "competing_domain_turn");
        else if (state && [
          "preparing", "identity_delivering", "identity_requested", "identity_attested",
          "context_delta_observed",
          "delivering", "actuation_requested", "transition_signaled",
          "rehydrating", "rehydration_requested", "unreconciled",
        ].includes(state.phase)) {
          throw new ContextTransitionLeaseError(
            "transition_in_flight",
            "domain turn must remain outside the retiring revision",
            state.lease
              ? { leaseRevision: state.lease.leaseRevision }
              : { preparationRevision: state.preparation.preparationRevision },
          );
        }
        return this.#deliverWithPermit(logicalRoleInstanceId, deliver);
      }
      if (!state || !["ready", "transition_signaled"].includes(state.phase)
          || !verifyContextTransitionLease(transitionLease)
          || transitionLease.leaseRevision !== state.lease.leaseRevision) {
        throw new ContextTransitionLeaseError(
          "invalid_transition_lease",
          "retirement control delivery requires the active transition lease",
        );
      }
      if (state.phase === "transition_signaled") {
        const expected = state.rehydration;
        if (toolBridge !== null
            || turnText !== expected.request.text
            || !equalCanonical(requestContext, expected.request.requestContext)
            || !equalCanonical(skills, expected.skills)) {
          throw new ContextTransitionLeaseError(
            "invalid_rehydration_turn",
            "rehydration delivery must match the exact checkpoint-bound request",
          );
        }
        state.phase = "rehydrating";
        try {
          const delivery = await this.#deliverWithPermit(logicalRoleInstanceId, deliver);
          state.phase = "rehydration_requested";
          state.rehydration = freeze({ ...expected, delivery: {
            threadId: text(delivery.threadId, "rehydration delivery thread"),
            turnId: text(delivery.turnId, "rehydration delivery turn"),
          } });
          return delivery;
        } catch (error) {
          const failureEntry = appendEvent(
            state.ledgerEntry,
            state.fence,
            this.now,
            "failure_recorded",
            "failed",
            [state.lease.leaseRevision, state.ledgerEntry.entryRevision],
            { phase: "rehydration_delivery", message: error instanceof Error ? error.message : "delivery failed" },
          );
          state.phase = "unreconciled";
          state.ledgerEntry = failureEntry;
          state.fence = freeze({ ...state.fence, ledgerRevision: failureEntry.entryRevision });
          throw error;
        }
      }
      if (skills.length !== 0 || toolBridge !== null || requestContext !== null
          || turnText !== compileContextRetirementDirective(state.lease)) {
        this.#revokeReady(state, "non_sterile_retirement_turn");
        throw new ContextTransitionLeaseError(
          "non_sterile_retirement_turn",
          "retirement control delivery must contain only the exact lifecycle directive",
        );
      }
      const actuationEntry = appendEvent(
        state.ledgerEntry,
        state.fence,
        this.now,
        "actuation_requested",
        "attempted",
        [state.lease.leaseRevision, state.fence.publicationRevision],
        { mechanism: "new_context", invocationOwner: "target_model" },
      );
      state.phase = "delivering";
      state.ledgerEntry = actuationEntry;
      state.fence = freeze({ ...state.fence, ledgerRevision: actuationEntry.entryRevision });
      try {
        const delivery = await this.#deliverWithPermit(logicalRoleInstanceId, deliver);
        state.phase = "actuation_requested";
        state.delivery = freeze({
          threadId: text(delivery.threadId, "retirement delivery thread"),
          turnId: text(delivery.turnId, "retirement delivery turn"),
        });
        return delivery;
      } catch (error) {
        const failureEntry = appendEvent(
          state.ledgerEntry,
          state.fence,
          this.now,
          "failure_recorded",
          "failed",
          [state.lease.leaseRevision, state.ledgerEntry.entryRevision],
          { phase: "retirement_control_delivery", message: error instanceof Error ? error.message : "delivery failed" },
        );
        state.phase = "failed";
        state.ledgerEntry = failureEntry;
        state.fence = freeze({ ...state.fence, ledgerRevision: failureEntry.entryRevision });
        throw error;
      }
    });
  }

  recordContextWindowIdentity({ preparation, validation, contextSnapshot = null }) {
    if (!verifyContextTransitionPreparation(preparation)) {
      throw new TypeError("identity recording requires an integrity-valid preparation");
    }
    return this.#withRoleLock(preparation.subject.logicalRoleInstanceId, () => {
      const state = this.states.get(preparation.subject.logicalRoleInstanceId);
      if (!state || state.phase !== "identity_requested"
          || state.preparation.preparationRevision !== preparation.preparationRevision) {
        throw new ContextTransitionLeaseError(
          "invalid_preparation_phase",
          "identity receipt requires the active requested attestation",
        );
      }
      if (validation?.status !== "accepted") {
        state.phase = "preparation_failed";
        state.identityAttestation = freeze({
          status: "unresolved",
          reasons: validation?.reasons ?? ["missing_validation"],
          receipt: validation?.receipt ?? null,
        });
        return freeze({ status: "unresolved", validation: state.identityAttestation });
      }
      if (!contextSnapshot || contextSnapshot.threadId !== preparation.subject.threadId) {
        state.phase = "preparation_failed";
        throw new ContextTransitionLeaseError(
          "invalid_context_snapshot",
          "identity attestation requires a complete snapshot of the fenced thread",
        );
      }
      shaRevision(contextSnapshot.contextRevision, "prepared thread context revision");
      state.phase = "identity_attested";
      state.identityAttestation = freeze({
        status: "accepted",
        reasons: [],
        receipt: validation.receipt,
      });
      state.contextSnapshot = freeze(structuredClone(contextSnapshot));
      return freeze({ status: "attested", validation: state.identityAttestation });
    });
  }

  confirmPreparedContext({ lease, contextSnapshot }) {
    if (!verifyContextTransitionLease(lease)) {
      throw new TypeError("prepared-context confirmation requires an integrity-valid lease");
    }
    return this.#withRoleLock(lease.subject.logicalRoleInstanceId, () => {
      const state = this.states.get(lease.subject.logicalRoleInstanceId);
      if (!state || state.phase !== "ready" || state.lease.leaseRevision !== lease.leaseRevision) {
        throw new ContextTransitionLeaseError(
          "invalid_transition_phase",
          "prepared-context confirmation requires the active ready lease",
        );
      }
      if (lease.subject.preparedContextRevision === null) {
        return freeze({ status: "not_required", contextSnapshot: null });
      }
      if (!contextSnapshot || contextSnapshot.threadId !== lease.subject.threadId) {
        throw new ContextTransitionLeaseError(
          "invalid_context_snapshot",
          "final context snapshot does not match the leased thread",
        );
      }
      shaRevision(contextSnapshot.contextRevision, "final thread context revision");
      if (contextSnapshot.contextRevision !== lease.subject.preparedContextRevision) {
        const deltaEntry = appendEvent(
          state.ledgerEntry,
          state.fence,
          this.now,
          "readiness_recorded",
          "rejected",
          [
            lease.leaseRevision,
            lease.subject.preparedContextRevision,
            contextSnapshot.contextRevision,
          ],
          {
            reason: "context_delta_after_preparation",
            preparedContextRevision: lease.subject.preparedContextRevision,
            currentContextRevision: contextSnapshot.contextRevision,
          },
        );
        state.phase = "context_delta_observed";
        state.pendingContextSnapshot = freeze(structuredClone(contextSnapshot));
        state.ledgerEntry = deltaEntry;
        state.fence = freeze({ ...state.fence, ledgerRevision: deltaEntry.entryRevision });
        state.revocationReason = "context_delta_after_preparation";
        throw new ContextTransitionLeaseError(
          "context_snapshot_changed",
          "thread context changed after checkpoint preparation; compile the captured delta before retirement",
          {
            preparedContextRevision: lease.subject.preparedContextRevision,
            currentContextRevision: contextSnapshot.contextRevision,
          },
        );
      }
      state.contextSnapshotConfirmedAt = timestamp(
        this.now(),
        "prepared-context confirmation timestamp",
      );
      return freeze({ status: "confirmed", contextSnapshot: state.contextSnapshot });
    });
  }

  adoptContextDeltaForRecompilation({ lease }) {
    if (!verifyContextTransitionLease(lease)) {
      throw new TypeError("context-delta adoption requires an integrity-valid lease");
    }
    return this.#withRoleLock(lease.subject.logicalRoleInstanceId, () => {
      const state = this.states.get(lease.subject.logicalRoleInstanceId);
      if (!state || state.phase !== "context_delta_observed"
          || state.lease.leaseRevision !== lease.leaseRevision
          || !state.pendingContextSnapshot) {
        throw new ContextTransitionLeaseError(
          "invalid_transition_phase",
          "context-delta adoption requires the captured post-preparation delta",
        );
      }
      state.phase = "identity_attested";
      state.contextSnapshot = state.pendingContextSnapshot;
      state.pendingContextSnapshot = null;
      state.lease = null;
      state.publication = null;
      state.revocationReason = null;
      return freeze({
        status: "recompile_required",
        preparation: state.preparation,
        identityAttestation: state.identityAttestation,
        contextSnapshot: state.contextSnapshot,
        previousLedgerEntry: state.ledgerEntry,
      });
    });
  }

  prepareRehydration({ lease, observation, receiptNonce, skills = [] }) {
    if (!verifyContextTransitionLease(lease)) {
      throw new TypeError("transition observation requires an integrity-valid lease");
    }
    return this.#withRoleLock(lease.subject.logicalRoleInstanceId, () => {
      const state = this.states.get(lease.subject.logicalRoleInstanceId);
      if (!state || state.phase !== "actuation_requested"
          || state.lease.leaseRevision !== lease.leaseRevision) {
        throw new ContextTransitionLeaseError(
          "invalid_transition_phase",
          "transition evidence requires the active requested actuation",
        );
      }
      const accepted = observation?.schemaVersion === 1
        && observation.observationType === "context_transition_signal"
        && observation.source?.provider === "codex"
        && observation.source?.transport === "app-server"
        && observation.source?.protocolVersion === PINNED_PROTOCOL.codexCliVersion
        && ["item/completed", "thread/compacted"].includes(observation.source?.method)
        && observation.threadId === state.delivery.threadId
        && observation.turnId === state.delivery.turnId
        && observation.details?.signal === "context_compaction"
        && ["completed", "reported"].includes(observation.details?.phase)
        && observation.details?.classification === "unclassified";
      if (!accepted) {
        throw new ContextTransitionLeaseError(
          "unmatched_transition_signal",
          "notification does not match the pinned retirement transition subject",
        );
      }
      const request = buildContextRehydrationRequest({
        lease,
        publication: state.publication,
        receiptNonce,
      });
      const transitionEntry = appendEvent(
        state.ledgerEntry,
        state.fence,
        this.now,
        "transition_observed",
        "unresolved",
        [state.lease.leaseRevision, state.fence.publicationRevision],
        {
          provider: observation.source.provider,
          protocolVersion: observation.source.protocolVersion,
          method: observation.source.method,
          retirementTurnId: observation.turnId,
          classification: "successor_reconciliation_required",
        },
      );
      state.phase = "transition_signaled";
      state.ledgerEntry = transitionEntry;
      state.fence = freeze({ ...state.fence, ledgerRevision: transitionEntry.entryRevision });
      state.rehydration = freeze({ request, skills: freeze(structuredClone(skills)), delivery: null });
      return freeze({ status: "successor_reconciliation_required", ledgerEntry: transitionEntry, request });
    });
  }

  recordReconciliation({ lease, validation }) {
    if (!verifyContextTransitionLease(lease)) {
      throw new TypeError("reconciliation recording requires an integrity-valid lease");
    }
    return this.#withRoleLock(lease.subject.logicalRoleInstanceId, () => {
      const state = this.states.get(lease.subject.logicalRoleInstanceId);
      if (!state || state.phase !== "rehydration_requested"
          || state.lease.leaseRevision !== lease.leaseRevision) {
        throw new ContextTransitionLeaseError(
          "invalid_transition_phase",
          "reconciliation receipt requires the active rehydration turn",
        );
      }
      const accepted = validation?.status === "accepted";
      const entry = appendEvent(
        state.ledgerEntry,
        state.fence,
        this.now,
        "reconciliation_recorded",
        accepted ? "accepted" : "unresolved",
        [state.lease.leaseRevision, state.fence.publicationRevision],
        {
          checkpointRevision: state.lease.subject.checkpointRevision,
          rehydrationTurnId: state.rehydration.delivery.turnId,
          reasons: validation?.reasons ?? ["missing_validation"],
        },
      );
      state.phase = accepted ? "reconciled" : "unreconciled";
      state.ledgerEntry = entry;
      state.fence = freeze({ ...state.fence, ledgerRevision: entry.entryRevision });
      return freeze({ status: accepted ? "reconciled" : "unresolved", ledgerEntry: entry });
    });
  }

  recordTransitionFailure({ lease, phase, error }) {
    if (!verifyContextTransitionLease(lease)) {
      throw new TypeError("transition failure recording requires an integrity-valid lease");
    }
    return this.#withRoleLock(lease.subject.logicalRoleInstanceId, () => {
      const state = this.states.get(lease.subject.logicalRoleInstanceId);
      if (!state || state.lease.leaseRevision !== lease.leaseRevision
          || [
            "ready", "context_delta_observed", "revoked", "failed", "reconciled", "unreconciled",
          ].includes(state.phase)) {
        return freeze({ status: "not_recorded" });
      }
      const entry = appendEvent(
        state.ledgerEntry,
        state.fence,
        this.now,
        "failure_recorded",
        "failed",
        [state.lease.leaseRevision, state.ledgerEntry.entryRevision],
        {
          phase: text(phase, "transition failure phase"),
          message: error instanceof Error ? error.message : "transition orchestration failed",
        },
      );
      state.phase = "unreconciled";
      state.ledgerEntry = entry;
      state.fence = freeze({ ...state.fence, ledgerRevision: entry.entryRevision });
      return freeze({ status: "recorded", ledgerEntry: entry });
    });
  }

  admitToolEffect({ threadId }) {
    text(threadId, "tool effect thread");
    const logicalRoleInstanceId = this.threadRoles.get(threadId);
    if (!logicalRoleInstanceId) return Promise.resolve(freeze({ allowed: true }));
    return this.#withRoleLock(logicalRoleInstanceId, () => {
      const state = this.states.get(logicalRoleInstanceId);
      if (!state || ["preparation_failed", "revoked", "failed", "reconciled"].includes(state.phase)) {
        return freeze({ allowed: true });
      }
      if (state.phase === "ready") this.#revokeReady(state, "competing_tool_effect");
      return freeze({
        allowed: false,
        reason: state.phase === "revoked" ? "transition_lease_revoked" : "transition_in_flight",
        ...(state.lease
          ? { leaseRevision: state.lease.leaseRevision }
          : { preparationRevision: state.preparation.preparationRevision }),
      });
    });
  }

  runBindingAdmission({ logicalRoleInstanceId, admissionPermit = null }, bind) {
    text(logicalRoleInstanceId, "binding admission role");
    if (typeof bind !== "function") throw new TypeError("binding admission requires mutation");
    if (admissionPermit
        && this.activePermits.has(admissionPermit)
        && admissionPermit.logicalRoleInstanceId === logicalRoleInstanceId) {
      return bind();
    }
    return this.#withRoleLock(logicalRoleInstanceId, () => {
      const state = this.states.get(logicalRoleInstanceId);
      if (state?.phase === "ready") this.#revokeReady(state, "runtime_binding_change");
      else if (state && [
        "preparing", "identity_delivering", "identity_requested", "identity_attested",
        "context_delta_observed",
        "delivering", "actuation_requested", "transition_signaled",
        "rehydrating", "rehydration_requested", "unreconciled",
      ].includes(state.phase)) {
        throw new ContextTransitionLeaseError(
          "transition_in_flight",
          "runtime binding cannot change while context transition actuation is in flight",
          state.lease
            ? { leaseRevision: state.lease.leaseRevision }
            : { preparationRevision: state.preparation.preparationRevision },
        );
      }
      return bind();
    });
  }
}

export class ContextTransitionLeaseRuntime {
  constructor({ gate, adapter, inputCustody = null }) {
    if (!gate || typeof gate.acquire !== "function" || typeof gate.runTurnAdmission !== "function") {
      throw new TypeError("context transition runtime requires a transition lease gate");
    }
    if (!adapter || typeof adapter.deliverTurn !== "function") {
      throw new TypeError("context transition runtime requires an App Server adapter");
    }
    if (inputCustody !== null && (typeof inputCustody.closeAdmission !== "function"
        || typeof inputCustody.admission !== "function"
        || typeof inputCustody.releaseAfterReconciliation !== "function")) {
      throw new TypeError("context transition runtime input custody is incomplete");
    }
    this.gate = gate;
    this.adapter = adapter;
    this.inputCustody = inputCustody;
  }

  acquire(input) {
    return this.gate.acquire(input);
  }

  beginPreparation(input) {
    return this.gate.beginPreparation(input, this.inputCustody === null ? {} : {
      closeAdmission: (preparation) => this.inputCustody.closeAdmission({
        logicalRoleInstanceId: preparation.subject.logicalRoleInstanceId,
        threadId: preparation.subject.threadId,
        bindingRevision: preparation.subject.bindingRevision,
        transitionRevision: preparation.preparationRevision,
      }),
    });
  }

  async attestContextWindow({ role, preparation, clientUserMessageId, signal }) {
    if (!verifyContextTransitionPreparation(preparation)) {
      throw new TypeError("context-window attestation requires an integrity-valid preparation");
    }
    const delivery = await this.adapter.deliverTurn({
      role,
      text: compileContextWindowIdentityDirective(preparation),
      clientUserMessageId,
      skills: [],
      toolBridge: null,
      requestContext: null,
      transitionPreparation: preparation,
    });
    const completion = await this.adapter.waitForTurnCompletion({ ...delivery, signal });
    const validation = validateContextWindowIdentityReceipt(completion.outputText, {
      preparation,
    });
    const contextSnapshot = validation.status === "accepted"
      ? await this.adapter.readThreadEffectiveContextSnapshot({ threadId: delivery.threadId })
      : null;
    const attestation = await this.gate.recordContextWindowIdentity({
      preparation,
      validation,
      contextSnapshot,
    });
    return freeze({ delivery, completion, validation, contextSnapshot, attestation });
  }

  promotePreparation(input) {
    return this.gate.promotePreparation(input);
  }

  adoptContextDeltaForRecompilation(input) {
    return this.gate.adoptContextDeltaForRecompilation(input);
  }

  async deliverRetirementControlTurn({ role, lease, clientUserMessageId }) {
    if (!verifyContextTransitionLease(lease)) {
      throw new TypeError("retirement control turn requires an integrity-valid lease");
    }
    if (lease.subject.preparedContextRevision != null) {
      const contextSnapshot = await this.adapter.readThreadEffectiveContextSnapshot({
        threadId: lease.subject.threadId,
      });
      await this.gate.confirmPreparedContext({ lease, contextSnapshot });
    }
    return this.adapter.deliverTurn({
      role,
      text: compileContextRetirementDirective(lease),
      clientUserMessageId,
      skills: [],
      toolBridge: null,
      requestContext: null,
      transitionLease: lease,
    });
  }

  async retireAndReconcile({
    role,
    lease,
    retirementClientUserMessageId,
    rehydrationClientUserMessageId,
    receiptNonce,
    skills = [],
    signal,
  }) {
    if (!verifyContextTransitionLease(lease)) {
      throw new TypeError("retirement and reconciliation requires an integrity-valid lease");
    }
    if (role?.logicalRoleInstanceId !== lease.subject.logicalRoleInstanceId) {
      throw new TypeError("successor role does not match the transition lease subject");
    }
    const state = this.gate.snapshot(lease.subject.logicalRoleInstanceId);
    await verifyRehydrationEnvironment({ publication: state?.publication, role, skills });
    signal?.throwIfAborted();
    let retirementDelivery = null;
    const buffered = [];
    let settleObservation;
    let rejectObservation;
    const observationPromise = new Promise((resolve, reject) => {
      settleObservation = resolve;
      rejectObservation = reject;
    });
    let settled = false;
    const consider = (observation) => {
      if (settled) return;
      if (!retirementDelivery) {
        buffered.push(observation);
        return;
      }
      if (observation.threadId === retirementDelivery.threadId
          && observation.turnId === retirementDelivery.turnId
          && observation.observationType === "context_transition_signal"
          && ["completed", "reported"].includes(observation.details.phase)) {
        settled = true;
        settleObservation(observation);
      }
    };
    const unsubscribe = this.adapter.onNotification((notification) => {
      try {
        const observation = normalizeCodexLifecycleNotification(notification);
        if (observation) consider(observation);
      } catch (error) {
        if (!settled) {
          settled = true;
          rejectObservation(error);
        }
      }
    });
    const onAbort = () => {
      if (settled) return;
      settled = true;
      rejectObservation(signal.reason ?? new Error("transition observation aborted"));
    };
    signal?.addEventListener("abort", onAbort, { once: true });
    try {
      retirementDelivery = await this.deliverRetirementControlTurn({
        role,
        lease,
        clientUserMessageId: retirementClientUserMessageId,
      });
      for (const observation of buffered) consider(observation);
      const [observation] = await Promise.all([
        observationPromise,
        this.adapter.waitForTurnCompletion({ ...retirementDelivery, signal }),
      ]);
      const transition = await this.gate.prepareRehydration({
        lease,
        observation,
        receiptNonce,
        skills,
      });
      const rehydrationDelivery = await this.adapter.deliverTurn({
        role,
        text: transition.request.text,
        clientUserMessageId: rehydrationClientUserMessageId,
        skills,
        toolBridge: null,
        requestContext: transition.request.requestContext,
        transitionLease: lease,
      });
      const completion = await this.adapter.waitForTurnCompletion({
        ...rehydrationDelivery,
        signal,
      });
      const validation = validateContextReconciliationReceipt(
        completion.outputText,
        { challenge: transition.request.challenge },
      );
      const reconciliation = await this.gate.recordReconciliation({ lease, validation });
      let queuedInputRelease = null;
      if (reconciliation.status === "reconciled" && this.inputCustody !== null) {
        const admission = this.inputCustody.admission(lease.subject.logicalRoleInstanceId);
        if (!admission || admission.threadId !== lease.subject.threadId
            || admission.bindingRevision !== lease.subject.bindingRevision) {
          throw new ContextTransitionLeaseError(
            "input_custody_mismatch",
            "reconciled transition does not match its closed input admission",
          );
        }
        queuedInputRelease = await this.inputCustody.releaseAfterReconciliation({
          logicalRoleInstanceId: lease.subject.logicalRoleInstanceId,
          transitionRevision: admission.transitionRevision,
          reconciliationRevision: reconciliation.ledgerEntry.entryRevision,
        }, async (queuedInput) => {
          const delivery = await this.adapter.deliverTurn({
            role,
            text: queuedInput.text,
            clientUserMessageId: queuedInput.clientUserMessageId,
            skills,
            toolBridge: null,
            requestContext: null,
          });
          await this.adapter.waitForTurnCompletion({ ...delivery, signal });
          return freeze({
            logicalRoleInstanceId: delivery.logicalRoleInstanceId,
            threadId: delivery.threadId,
            turnId: delivery.turnId,
            clientUserMessageId: queuedInput.clientUserMessageId,
            replayedDelivery: delivery.replayedDelivery,
          });
        });
      }
      return freeze({
        retirementDelivery,
        transitionObservation: observation,
        rehydrationDelivery,
        completion,
        validation,
        reconciliation,
        queuedInputRelease,
      });
    } catch (error) {
      await this.gate.recordTransitionFailure({
        lease,
        phase: "transition_observation_or_reconciliation",
        error,
      });
      throw error;
    } finally {
      signal?.removeEventListener("abort", onAbort);
      unsubscribe();
    }
  }
}
