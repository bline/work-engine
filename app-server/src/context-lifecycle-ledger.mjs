import { createHash } from "node:crypto";

export const CONTEXT_LIFECYCLE_LEDGER_SCHEMA_VERSION = 1;
const EVENT_STATUSES = new Set(["accepted", "attempted", "failed", "observed", "rejected", "unresolved"]);
const EVENT_TYPES = new Set([
  "actuation_requested", "checkpoint_candidate_recorded", "checkpoint_published",
  "failure_recorded", "observation_recorded", "readiness_recorded",
  "reconciliation_recorded", "transition_observed", "verification_recorded",
]);
const REQUIRED_STATUS = new Map([
  ["actuation_requested", new Set(["attempted"])],
  ["checkpoint_candidate_recorded", new Set(["observed"])],
  ["checkpoint_published", new Set(["observed"])],
  ["failure_recorded", new Set(["failed"])],
  ["observation_recorded", new Set(["observed"])],
  ["transition_observed", new Set(["observed", "unresolved"])],
]);

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`${label} must be an object`);
  return value;
}
function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(`${label} must be non-empty`);
  return value;
}
function rejectUnknown(value, allowed, label) {
  const unknown = Object.keys(value).filter((key) => !allowed.includes(key)).sort();
  if (unknown.length) throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
}
function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}
function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}
function json(value, label) {
  if (value === null || ["string", "boolean"].includes(typeof value)) return value;
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) throw new TypeError(`${label} numbers must be safe integers`);
    return value;
  }
  if (Array.isArray(value)) return value.map((item, index) => json(item, `${label}[${index}]`));
  record(value, label);
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, json(value[key], `${label}.${key}`)]));
}

function normalizeInput(input) {
  record(input, "lifecycle ledger input");
  rejectUnknown(
    input,
    ["eventType", "status", "recordedAt", "subject", "evidenceRefs", "details"],
    "lifecycle ledger input",
  );
  if (!EVENT_TYPES.has(input.eventType)) throw new TypeError("lifecycle ledger event type is unsupported");
  if (!EVENT_STATUSES.has(input.status)) throw new TypeError("lifecycle ledger event status is unsupported");
  if (REQUIRED_STATUS.has(input.eventType) && !REQUIRED_STATUS.get(input.eventType).has(input.status)) {
    throw new TypeError(`${input.eventType} cannot claim status ${input.status}`);
  }
  const recordedAt = text(input.recordedAt, "lifecycle ledger recordedAt");
  if (Number.isNaN(Date.parse(recordedAt))) throw new TypeError("lifecycle ledger recordedAt must be an ISO timestamp");
  const subject = record(input.subject, "lifecycle ledger subject");
  rejectUnknown(
    subject,
    ["logicalRoleInstanceId", "threadId", "bindingRevision"],
    "lifecycle ledger subject",
  );
  if (!Number.isSafeInteger(subject.bindingRevision) || subject.bindingRevision < 1) {
    throw new TypeError("lifecycle ledger binding revision must be positive");
  }
  return {
    eventType: input.eventType,
    status: input.status,
    recordedAt,
    subject: {
      logicalRoleInstanceId: text(subject.logicalRoleInstanceId, "ledger logical role instance id"),
      threadId: text(subject.threadId, "ledger thread id"),
      bindingRevision: subject.bindingRevision,
    },
    evidenceRefs: (input.evidenceRefs ?? []).map((value, index) => text(value, `ledger evidenceRefs[${index}]`)),
    details: json(input.details ?? {}, "ledger details"),
  };
}

function revisionFor(entry) {
  return `sha256:${createHash("sha256").update(canonical(entry)).digest("hex")}`;
}

function validateEntryIntegrity(candidate) {
  record(candidate, "lifecycle ledger entry");
  rejectUnknown(candidate, [
    "schemaVersion", "sequence", "previousRevision", "eventType", "status",
    "recordedAt", "subject", "evidenceRefs", "details", "entryRevision",
  ], "lifecycle ledger entry");
  if (candidate.schemaVersion !== CONTEXT_LIFECYCLE_LEDGER_SCHEMA_VERSION) {
    throw new TypeError("unsupported lifecycle ledger schema version");
  }
  if (!Number.isSafeInteger(candidate.sequence) || candidate.sequence < 1) {
    throw new TypeError("lifecycle ledger sequence must be positive");
  }
  if (candidate.sequence === 1 && candidate.previousRevision !== null) {
    throw new TypeError("first lifecycle ledger entry cannot have a previous revision");
  }
  if (candidate.sequence > 1
      && !/^sha256:[a-f0-9]{64}$/.test(candidate.previousRevision ?? "")) {
    throw new TypeError("later lifecycle ledger entries require a SHA-256 previous revision");
  }
  if (!/^sha256:[a-f0-9]{64}$/.test(candidate.entryRevision ?? "")) {
    throw new TypeError("lifecycle ledger entry revision must be SHA-256 bound");
  }
  const input = normalizeInput({
    eventType: candidate.eventType,
    status: candidate.status,
    recordedAt: candidate.recordedAt,
    subject: candidate.subject,
    evidenceRefs: candidate.evidenceRefs,
    details: candidate.details,
  });
  const body = {
    schemaVersion: CONTEXT_LIFECYCLE_LEDGER_SCHEMA_VERSION,
    sequence: candidate.sequence,
    previousRevision: candidate.previousRevision,
    ...input,
  };
  if (revisionFor(body) !== candidate.entryRevision) {
    throw new TypeError("lifecycle ledger entry revision does not match its content");
  }
  return body;
}

export function appendLifecycleLedgerEntry(previousEntry, input) {
  const normalized = normalizeInput(input);
  const previousRevision = previousEntry?.entryRevision ?? null;
  if (previousEntry) validateEntryIntegrity(previousEntry);
  const entry = {
    schemaVersion: CONTEXT_LIFECYCLE_LEDGER_SCHEMA_VERSION,
    sequence: previousEntry ? previousEntry.sequence + 1 : 1,
    previousRevision,
    ...normalized,
  };
  return freeze({
    ...entry,
    entryRevision: revisionFor(entry),
  });
}

export function verifyLifecycleLedgerEntry(candidate, previousEntry = null) {
  try {
    validateEntryIntegrity(candidate);
    if (previousEntry) validateEntryIntegrity(previousEntry);
    return candidate.sequence === (previousEntry?.sequence ?? 0) + 1
      && candidate.previousRevision === (previousEntry?.entryRevision ?? null);
  } catch {
    return false;
  }
}

export function verifyLifecycleLedger(entries) {
  try {
    if (!Array.isArray(entries)) return false;
    let previous = null;
    for (const candidate of entries) {
      if (!verifyLifecycleLedgerEntry(candidate, previous)) return false;
      previous = candidate;
    }
    return true;
  } catch {
    return false;
  }
}
