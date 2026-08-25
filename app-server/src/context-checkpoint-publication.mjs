import { createHash } from "node:crypto";

import {
  appendLifecycleLedgerEntry,
  verifyLifecycleLedgerEntry,
} from "./context-lifecycle-ledger.mjs";
import { verifySemanticContextVerification } from "./semantic-context-inference.mjs";

export const CONTEXT_CHECKPOINT_SCHEMA_VERSION = 1;
export const CONTEXT_CHECKPOINT_TYPE = "work-engine.context-checkpoint";

const SHA_REVISION = /^sha256:[a-f0-9]{64}$/;

export class ContextCheckpointPublicationError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "ContextCheckpointPublicationError";
    this.code = code;
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

function rejectUnknown(value, allowed, label) {
  const unknown = Object.keys(value).filter((key) => !allowed.includes(key)).sort();
  if (unknown.length > 0) {
    throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
  }
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

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function revision(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function normalizeReference(value, label) {
  record(value, label);
  rejectUnknown(value, ["reference", "sha256"], label);
  const sha256 = text(value.sha256, `${label}.sha256`);
  if (!/^[a-f0-9]{64}$/.test(sha256)) throw new TypeError(`${label}.sha256 is invalid`);
  return { reference: text(value.reference, `${label}.reference`), sha256 };
}

function referenceKey(value) {
  return `${value.reference}\u0000${value.sha256}`;
}

function authorityReferences(candidate) {
  const references = [
    candidate.objective.authorityRef,
    candidate.authorizedNextAction.authorityRef,
    ...candidate.authorityDependencies.canonicalRecords,
    ...candidate.humanInteractions
      .map((interaction) => interaction.durableConsequenceRef)
      .filter(Boolean),
    ...candidate.humanInteractions.map((interaction) => interaction.sourceRef),
  ].map((value, index) => normalizeReference(value, `authority reference ${index}`));
  const unique = new Map(references.map((value) => [referenceKey(value), value]));
  return [...unique.values()].sort((left, right) => {
    const leftKey = referenceKey(left);
    const rightKey = referenceKey(right);
    return leftKey < rightKey ? -1 : leftKey > rightKey ? 1 : 0;
  });
}

function normalizeAuthorityValidation(value, expectedReferences) {
  record(value, "authority revalidation result");
  rejectUnknown(
    value,
    ["status", "authorityRevision", "checkedReferences", "evidenceRefs"],
    "authority revalidation result",
  );
  if (!["current", "invalid", "uncertain"].includes(value.status)) {
    throw new TypeError("authority revalidation status is unsupported");
  }
  const checkedReferences = (value.checkedReferences ?? []).map((reference, index) =>
    normalizeReference(reference, `authority checkedReferences[${index}]`)
  );
  const expected = expectedReferences.map(referenceKey);
  const actual = checkedReferences.map(referenceKey);
  if (new Set(actual).size !== actual.length
      || expected.length !== actual.length
      || expected.some((key, index) => key !== actual[index])) {
    throw new TypeError("authority revalidation must cover the exact authority reference set");
  }
  const evidenceRefs = (value.evidenceRefs ?? []).map((entry, index) =>
    shaRevision(entry, `authority evidenceRefs[${index}]`)
  );
  if (evidenceRefs.length === 0 || new Set(evidenceRefs).size !== evidenceRefs.length) {
    throw new TypeError("authority revalidation requires unique evidence references");
  }
  return freeze({
    status: value.status,
    authorityRevision: shaRevision(value.authorityRevision, "authority revision"),
    checkedReferences,
    evidenceRefs,
  });
}

function normalizeFence(value) {
  record(value, "checkpoint lifecycle fence");
  rejectUnknown(value, [
    "logicalRoleInstanceId", "threadId", "bindingRevision", "sourceRevision",
    "authorityRevision", "publicationRevision", "ledgerRevision",
  ], "checkpoint lifecycle fence");
  if (!Number.isSafeInteger(value.bindingRevision) || value.bindingRevision < 1) {
    throw new TypeError("checkpoint lifecycle fence bindingRevision must be positive");
  }
  for (const field of ["publicationRevision", "ledgerRevision"]) {
    if (value[field] !== null) shaRevision(value[field], `checkpoint lifecycle fence ${field}`);
  }
  return freeze({
    logicalRoleInstanceId: text(value.logicalRoleInstanceId, "checkpoint lifecycle role"),
    threadId: text(value.threadId, "checkpoint lifecycle thread"),
    bindingRevision: value.bindingRevision,
    sourceRevision: shaRevision(value.sourceRevision, "checkpoint lifecycle source revision"),
    authorityRevision: shaRevision(value.authorityRevision, "checkpoint lifecycle authority revision"),
    publicationRevision: value.publicationRevision,
    ledgerRevision: value.ledgerRevision,
  });
}

export function validateContextCheckpointFence(value) {
  return normalizeFence(value);
}

export function verifyContextCheckpointPublication(value) {
  try {
    record(value, "checkpoint publication");
    const { checkpointRevision, ...checkpoint } = value;
    return value.schemaVersion === CONTEXT_CHECKPOINT_SCHEMA_VERSION
      && value.type === CONTEXT_CHECKPOINT_TYPE
      && revision(checkpoint) === checkpointRevision;
  } catch {
    return false;
  }
}

export function validateContextCheckpointPublicationAttempt({
  expectedFence,
  publication,
  ledgerEntry,
  previousLedgerEntry = null,
}) {
  const expected = normalizeFence(expectedFence);
  if (!verifyContextCheckpointPublication(publication)) {
    throw new TypeError("checkpoint publication integrity is invalid");
  }
  const { checkpointRevision } = publication;
  const subject = record(publication.subject, "checkpoint publication subject");
  for (const field of [
    "logicalRoleInstanceId", "threadId", "bindingRevision", "sourceRevision", "authorityRevision",
  ]) {
    if (subject[field] !== expected[field]) {
      throw new TypeError(`checkpoint publication subject does not match expected ${field}`);
    }
  }
  if (publication.predecessorCheckpointRevision !== expected.publicationRevision) {
    throw new TypeError("checkpoint publication predecessor does not match its fence");
  }
  if ((previousLedgerEntry?.entryRevision ?? null) !== expected.ledgerRevision
      || !verifyLifecycleLedgerEntry(ledgerEntry, previousLedgerEntry)) {
    throw new TypeError("checkpoint publication ledger predecessor is invalid");
  }
  if (previousLedgerEntry
      && (previousLedgerEntry.subject.logicalRoleInstanceId !== expected.logicalRoleInstanceId
        || previousLedgerEntry.subject.threadId !== expected.threadId
        || previousLedgerEntry.subject.bindingRevision !== expected.bindingRevision)) {
    throw new TypeError("checkpoint publication ledger predecessor belongs to another subject");
  }
  if (ledgerEntry.subject.logicalRoleInstanceId !== expected.logicalRoleInstanceId
      || ledgerEntry.subject.threadId !== expected.threadId
      || ledgerEntry.subject.bindingRevision !== expected.bindingRevision
      || ledgerEntry.eventType !== "checkpoint_published"
      || ledgerEntry.status !== "observed"
      || ledgerEntry.details.checkpointRevision !== checkpointRevision) {
    throw new TypeError("checkpoint publication ledger evidence does not match the checkpoint");
  }
  return freeze({ expected, publication, ledgerEntry, previousLedgerEntry });
}

function rejected(reason, details = {}) {
  return freeze({ status: "rejected", reason, ...details });
}

export class InMemoryContextCheckpointPublicationStore {
  constructor(initialFences = []) {
    this.states = new Map();
    for (const value of initialFences) {
      const fence = normalizeFence(value);
      if (this.states.has(fence.logicalRoleInstanceId)) {
        throw new TypeError(`duplicate checkpoint lifecycle fence ${fence.logicalRoleInstanceId}`);
      }
      this.states.set(fence.logicalRoleInstanceId, {
        fence,
        candidateRevisions: new Set(),
        publication: null,
        ledgerEntry: null,
      });
    }
  }

  snapshot(logicalRoleInstanceId) {
    const state = this.states.get(logicalRoleInstanceId);
    if (!state) return null;
    return freeze({
      fence: { ...state.fence },
      publication: state.publication,
      ledgerEntry: state.ledgerEntry,
    });
  }

  compareAndSwapPublication({ expectedFence, publication, ledgerEntry, previousLedgerEntry = null }) {
    const { expected } = validateContextCheckpointPublicationAttempt({
      expectedFence,
      publication,
      ledgerEntry,
      previousLedgerEntry,
    });
    const state = this.states.get(expected.logicalRoleInstanceId);
    if (!state) return rejected("missing_lifecycle_fence");
    const current = state.fence;
    for (const [field, reason] of [
      ["threadId", "stale_runtime_binding"],
      ["bindingRevision", "stale_runtime_binding"],
      ["sourceRevision", "stale_source_revision"],
      ["authorityRevision", "stale_authority_revision"],
      ["publicationRevision", "publication_conflict"],
      ["ledgerRevision", "ledger_conflict"],
    ]) {
      if (current[field] !== expected[field]) return rejected(reason, { currentFence: current });
    }
    if (state.candidateRevisions.has(publication.subject.candidateRevision)) {
      return rejected("duplicate_candidate", { currentFence: current });
    }
    state.candidateRevisions.add(publication.subject.candidateRevision);
    state.publication = publication;
    state.ledgerEntry = ledgerEntry;
    state.fence = freeze({
      ...current,
      publicationRevision: publication.checkpointRevision,
      ledgerRevision: ledgerEntry.entryRevision,
    });
    return freeze({ status: "committed", currentFence: state.fence });
  }
}

export class ContextCheckpointPublisher {
  constructor({ store, resolvePublicKey, revalidateAuthority, now = () => new Date().toISOString() }) {
    if (!store || typeof store.compareAndSwapPublication !== "function") {
      throw new TypeError("checkpoint publisher requires an atomic publication store");
    }
    if (typeof resolvePublicKey !== "function" || typeof revalidateAuthority !== "function") {
      throw new TypeError("checkpoint publisher requires key resolution and authority revalidation");
    }
    this.store = store;
    this.resolvePublicKey = resolvePublicKey;
    this.revalidateAuthority = revalidateAuthority;
    this.now = now;
  }

  async publish({
    projection,
    candidate,
    verification,
    expectedPublicationRevision = null,
    previousLedgerEntry = null,
  }) {
    if (!verifySemanticContextVerification(verification, {
      candidate,
      projection,
      resolvePublicKey: this.resolvePublicKey,
    })) {
      throw new ContextCheckpointPublicationError(
        "invalid_verification",
        "checkpoint publication requires an integrity-valid semantic verification",
      );
    }
    if (verification.disposition !== "accepted") {
      return rejected("verification_not_accepted", {
        verificationRevision: verification.verificationRevision,
      });
    }

    const references = authorityReferences(candidate);
    const authority = normalizeAuthorityValidation(await this.revalidateAuthority({
      logicalRoleInstanceId: candidate.subject.logicalRoleInstanceId,
      references,
      requirements: [...candidate.authorityDependencies.revalidationRequired],
      sourceRevision: candidate.subject.sourceRevision,
      candidateRevision: candidate.candidateRevision,
    }), references);
    if (authority.status !== "current") {
      return rejected(`authority_${authority.status}`, {
        authorityRevision: authority.authorityRevision,
        evidenceRefs: authority.evidenceRefs,
      });
    }

    const publishedAt = text(this.now(), "checkpoint publication timestamp");
    if (Number.isNaN(Date.parse(publishedAt))) {
      throw new TypeError("checkpoint publication timestamp must be an ISO timestamp");
    }
    if (expectedPublicationRevision !== null) {
      shaRevision(expectedPublicationRevision, "expected publication revision");
    }
    const binding = projection.observedContext.runtimeBinding;
    const previousLedgerRevision = previousLedgerEntry?.entryRevision ?? null;
    const checkpoint = {
      schemaVersion: CONTEXT_CHECKPOINT_SCHEMA_VERSION,
      type: CONTEXT_CHECKPOINT_TYPE,
      subject: {
        logicalRoleInstanceId: candidate.subject.logicalRoleInstanceId,
        threadId: binding.threadId,
        bindingRevision: candidate.subject.runtimeBindingRevision,
        sourceRevision: candidate.subject.sourceRevision,
        candidateRevision: candidate.candidateRevision,
        verificationRevision: verification.verificationRevision,
        authorityRevision: authority.authorityRevision,
      },
      publishedAt,
      predecessorCheckpointRevision: expectedPublicationRevision,
      authority,
      continuationState: candidate,
      verification,
    };
    const publication = freeze({
      ...checkpoint,
      checkpointRevision: revision(checkpoint),
    });
    const ledgerEntry = appendLifecycleLedgerEntry(previousLedgerEntry, {
      eventType: "checkpoint_published",
      status: "observed",
      recordedAt: publishedAt,
      subject: {
        logicalRoleInstanceId: candidate.subject.logicalRoleInstanceId,
        threadId: binding.threadId,
        bindingRevision: candidate.subject.runtimeBindingRevision,
      },
      evidenceRefs: [
        candidate.subject.sourceRevision,
        candidate.candidateRevision,
        verification.verificationRevision,
        ...authority.evidenceRefs,
      ],
      details: {
        checkpointRevision: publication.checkpointRevision,
        candidateRevision: candidate.candidateRevision,
        verificationRevision: verification.verificationRevision,
        authorityRevision: authority.authorityRevision,
        predecessorCheckpointRevision: expectedPublicationRevision,
      },
    });
    const expectedFence = {
      logicalRoleInstanceId: candidate.subject.logicalRoleInstanceId,
      threadId: binding.threadId,
      bindingRevision: candidate.subject.runtimeBindingRevision,
      sourceRevision: candidate.subject.sourceRevision,
      authorityRevision: authority.authorityRevision,
      publicationRevision: expectedPublicationRevision,
      ledgerRevision: previousLedgerRevision,
    };
    const outcome = await this.store.compareAndSwapPublication({
      expectedFence,
      publication,
      ledgerEntry,
      previousLedgerEntry,
    });
    if (outcome?.status !== "committed") {
      return rejected(outcome?.reason ?? "publication_store_rejected", {
        currentFence: outcome?.currentFence ?? null,
      });
    }
    const committedFence = normalizeFence(outcome.currentFence);
    const expectedCommittedFence = {
      ...expectedFence,
      publicationRevision: publication.checkpointRevision,
      ledgerRevision: ledgerEntry.entryRevision,
    };
    if (Object.keys(expectedCommittedFence).some(
      (field) => committedFence[field] !== expectedCommittedFence[field],
    )) {
      throw new ContextCheckpointPublicationError(
        "invalid_store_receipt",
        "checkpoint publication store returned an invalid commit receipt",
      );
    }
    return freeze({ status: "published", publication, ledgerEntry, currentFence: committedFence });
  }
}
