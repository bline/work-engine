import { createHash } from "node:crypto";

export const CONTEXT_PRESSURE_POLICY_SCHEMA_VERSION = 1;
export const CONTEXT_PRESSURE_OBSERVATION_SCHEMA_VERSION = 1;

export const CONTEXT_PRESSURE_DISPOSITIONS = Object.freeze([
  "comfortable",
  "approaching",
  "replacement_candidate",
  "critical",
]);

const DISPOSITION_SET = new Set(CONTEXT_PRESSURE_DISPOSITIONS);
const SHA_REVISION = /^sha256:[a-f0-9]{64}$/;
const POLICY_FIELDS = ["schemaVersion", "unit", "approaching", "replacementCandidate", "critical"];
const BAND_FIELDS = ["enter", "exit"];

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function rejectUnknown(value, allowed, label) {
  const unknown = Object.keys(value).filter((field) => !allowed.includes(field)).sort();
  if (unknown.length > 0) {
    throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
  }
}

function basisPoints(value, label) {
  if (!Number.isSafeInteger(value) || value < 0 || value > 10_000) {
    throw new TypeError(`${label} must be an integer from 0 through 10000 basis points`);
  }
  return value;
}

function positiveInteger(value, label) {
  if (!Number.isSafeInteger(value) || value < 1) {
    throw new TypeError(`${label} must be a positive safe integer`);
  }
  return value;
}

function shaRevision(value, label) {
  text(value, label);
  if (!SHA_REVISION.test(value)) throw new TypeError(`${label} must be SHA-256 bound`);
  return value;
}

function timestamp(value, label) {
  text(value, label);
  if (Number.isNaN(Date.parse(value))) throw new TypeError(`${label} must be an ISO timestamp`);
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

function normalizeBand(value, label) {
  record(value, label);
  rejectUnknown(value, BAND_FIELDS, label);
  const enter = basisPoints(value.enter, `${label}.enter`);
  const exit = basisPoints(value.exit, `${label}.exit`);
  if (exit >= enter) throw new TypeError(`${label}.exit must be lower than ${label}.enter`);
  return { enter, exit };
}

export function validateContextPressurePolicy(value) {
  record(value, "context pressure policy");
  rejectUnknown(value, POLICY_FIELDS, "context pressure policy");
  if (value.schemaVersion !== CONTEXT_PRESSURE_POLICY_SCHEMA_VERSION) {
    throw new TypeError("unsupported context pressure policy schema version");
  }
  if (value.unit !== "basis_points") {
    throw new TypeError("context pressure policy unit must be basis_points");
  }
  const policy = {
    schemaVersion: CONTEXT_PRESSURE_POLICY_SCHEMA_VERSION,
    unit: "basis_points",
    approaching: normalizeBand(value.approaching, "context pressure approaching band"),
    replacementCandidate: normalizeBand(
      value.replacementCandidate,
      "context pressure replacementCandidate band",
    ),
    critical: normalizeBand(value.critical, "context pressure critical band"),
  };
  const ordered = [
    policy.approaching.exit,
    policy.approaching.enter,
    policy.replacementCandidate.exit,
    policy.replacementCandidate.enter,
    policy.critical.exit,
    policy.critical.enter,
  ];
  if (ordered.some((threshold, index) => index > 0 && threshold < ordered[index - 1])) {
    throw new TypeError(
      "context pressure thresholds must be ordered across approaching, replacementCandidate, and critical bands",
    );
  }
  return freeze({ ...policy, policyRevision: revision(policy) });
}

function normalizeObservation(value) {
  record(value, "context pressure observation");
  rejectUnknown(
    value,
    [
      "schemaVersion", "observationId", "sequence", "observedAt",
      "pressureBasisPoints", "source", "sourceRevision",
    ],
    "context pressure observation",
  );
  if (value.schemaVersion !== CONTEXT_PRESSURE_OBSERVATION_SCHEMA_VERSION) {
    throw new TypeError("unsupported context pressure observation schema version");
  }
  return freeze({
    schemaVersion: CONTEXT_PRESSURE_OBSERVATION_SCHEMA_VERSION,
    observationId: text(value.observationId, "context pressure observation id"),
    sequence: positiveInteger(value.sequence, "context pressure observation sequence"),
    observedAt: timestamp(value.observedAt, "context pressure observedAt"),
    pressureBasisPoints: basisPoints(
      value.pressureBasisPoints,
      "context pressure observation pressureBasisPoints",
    ),
    source: text(value.source, "context pressure observation source"),
    sourceRevision: shaRevision(
      value.sourceRevision,
      "context pressure observation source revision",
    ),
  });
}

function dispositionFor(current, pressure, policy) {
  if (current === "comfortable") {
    if (pressure >= policy.critical.enter) return "critical";
    if (pressure >= policy.replacementCandidate.enter) return "replacement_candidate";
    if (pressure >= policy.approaching.enter) return "approaching";
    return "comfortable";
  }
  if (current === "approaching") {
    if (pressure >= policy.critical.enter) return "critical";
    if (pressure >= policy.replacementCandidate.enter) return "replacement_candidate";
    return pressure >= policy.approaching.exit ? "approaching" : "comfortable";
  }
  if (current === "replacement_candidate") {
    if (pressure >= policy.critical.enter) return "critical";
    if (pressure >= policy.replacementCandidate.exit) return "replacement_candidate";
    return pressure >= policy.approaching.exit ? "approaching" : "comfortable";
  }
  if (pressure >= policy.critical.exit) return "critical";
  if (pressure >= policy.replacementCandidate.exit) return "replacement_candidate";
  return pressure >= policy.approaching.exit ? "approaching" : "comfortable";
}

function transitionReason(previous, current, pressure, policy) {
  if (previous !== current) return "threshold_crossed";
  if (previous === "comfortable") return "below_entry_threshold";
  const enter = previous === "approaching"
    ? policy.approaching.enter
    : previous === "replacement_candidate"
      ? policy.replacementCandidate.enter
      : policy.critical.enter;
  return pressure < enter ? "hysteresis_held" : "within_disposition";
}

export class ContextPressureController {
  constructor({ policy, initialDisposition = "comfortable", minimumSequence = 0 }) {
    if (!DISPOSITION_SET.has(initialDisposition)) {
      throw new TypeError("context pressure initial disposition is unsupported");
    }
    if (!Number.isSafeInteger(minimumSequence) || minimumSequence < 0) {
      throw new TypeError("context pressure minimum sequence must be a non-negative safe integer");
    }
    this.policy = validateContextPressurePolicy(policy);
    this.disposition = initialDisposition;
    this.minimumSequence = minimumSequence;
    this.lastObservation = null;
    this.lastTransition = null;
  }

  snapshot() {
    return freeze({
      schemaVersion: 1,
      policyRevision: this.policy.policyRevision,
      disposition: this.disposition,
      minimumSequence: this.minimumSequence,
      lastObservation: this.lastObservation,
      lastTransition: this.lastTransition,
    });
  }

  observe(value) {
    const observation = normalizeObservation(value);
    if (this.lastObservation?.observationId === observation.observationId) {
      if (canonical(this.lastObservation) !== canonical(observation)) {
        throw new TypeError("context pressure observation id was reused for different evidence");
      }
      return freeze({ status: "replayed", ...this.lastTransition });
    }
    if (this.lastObservation && observation.sequence <= this.lastObservation.sequence) {
      throw new TypeError("context pressure observation sequence must increase monotonically");
    }
    if (!this.lastObservation && observation.sequence <= this.minimumSequence) {
      throw new TypeError("context pressure observation sequence must exceed the recovered floor");
    }
    const previousDisposition = this.disposition;
    const disposition = dispositionFor(
      previousDisposition,
      observation.pressureBasisPoints,
      this.policy,
    );
    const transition = freeze({
      schemaVersion: 1,
      policyRevision: this.policy.policyRevision,
      observation,
      previousDisposition,
      disposition,
      changed: disposition !== previousDisposition,
      reason: transitionReason(
        previousDisposition,
        disposition,
        observation.pressureBasisPoints,
        this.policy,
      ),
    });
    this.disposition = disposition;
    this.lastObservation = observation;
    this.lastTransition = transition;
    return freeze({ status: "observed", ...transition });
  }
}
