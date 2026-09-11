import { createHash } from "node:crypto";
import { validateProductionPathClaimRevision } from "../claim-evidence/production-path-contract.mjs";

export const SLICE_CAMPAIGN_SCHEMA_VERSION = 2;
export const SLICE_PHASES = Object.freeze(["accepted", "implementing", "gate_ready", "review_ready", "terminal"]);

export function requireRecord(value, label) {
  if (!value || Array.isArray(value) || typeof value !== "object") throw new TypeError(`${label} must be an object`);
  return value;
}

export function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(`${label} must be a non-empty string`);
  return value;
}

export function requireSha256(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{64}$/.test(value)) throw new TypeError(`${label} must be a SHA-256 digest`);
  return value;
}

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}

export function digest(value) {
  return createHash("sha256").update(canonicalJson(value)).digest("hex");
}

export function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export function normalizeIdentity(value) {
  requireRecord(value, "slice identity");
  const identity = {
    runId: requireText(value.runId, "slice identity runId"),
    sliceNumber: value.sliceNumber,
    attemptId: requireText(value.attemptId, "slice identity attemptId"),
    planVersion: requireText(value.planVersion, "slice identity planVersion"),
  };
  if (!Number.isInteger(identity.sliceNumber) || identity.sliceNumber < 1) throw new TypeError("slice identity sliceNumber must be a positive integer");
  return freeze(identity);
}

export function identityKey(identity) {
  return `${identity.runId}:${identity.sliceNumber}:${identity.attemptId}:${identity.planVersion}`;
}

export function validateReviewSelection(value, expectedSubject, campaignIdentity = null) {
  requireRecord(value, "review selection");
  const fields = ["schemaVersion", "owner", "selectionId", "subject", "specialists"];
  if (Object.keys(value).some((field) => !fields.includes(field)) || fields.some((field) => !(field in value))) {
    throw new TypeError("review selection fields are invalid");
  }
  if (![1, 2].includes(value.schemaVersion) || value.owner !== "slice-supervisor") throw new TypeError("review selection owner is invalid");
  requireText(value.selectionId, "review selection selectionId");
  requireRecord(value.subject, "review selection subject");
  if (digest(value.subject) !== digest(expectedSubject)) throw new TypeError("review selection subject does not match the immutable candidate");
  if (!Array.isArray(value.specialists) || value.specialists.length === 0) throw new TypeError("review selection requires specialist dispositions");
  const ids = [];
  for (const [index, specialist] of value.specialists.entries()) {
    requireRecord(specialist, `review selection specialists[${index}]`);
    const specialistFields = value.schemaVersion === 1
      ? ["obligationId", "skill", "selection"]
      : ["obligationId", "skill", "selection", "requiredClaims"];
    if (Object.keys(specialist).some((field) => !specialistFields.includes(field))
        || specialistFields.some((field) => !(field in specialist))) throw new TypeError("review specialist disposition fields are invalid");
    requireText(specialist.obligationId, "review specialist obligationId");
    requireText(specialist.skill, "review specialist skill");
    if (!["selected", "omitted"].includes(specialist.selection)) throw new TypeError("review specialist disposition is invalid");
    if (value.schemaVersion === 2) {
      if (!Array.isArray(specialist.requiredClaims)) throw new TypeError("review specialist requiredClaims must be an array");
      if (specialist.selection === "selected") {
        if (specialist.requiredClaims.length !== 2) throw new TypeError("selected review specialist requires builder and terminal claims");
        specialist.requiredClaims.forEach((claim) => validateProductionPathClaimRevision(claim, expectedSubject));
        const boundaries = new Set(specialist.requiredClaims.map(({consumptionBoundary}) => consumptionBoundary));
        if (boundaries.size !== 2 || !boundaries.has("builder_projection") || !boundaries.has("campaign_terminalization")) {
          throw new TypeError("selected review specialist required claim boundaries are incomplete");
        }
      } else if (specialist.requiredClaims.length !== 0) {
        throw new TypeError("omitted review specialist cannot declare required claims");
      }
    }
    ids.push(specialist.obligationId);
  }
  if (new Set(ids).size !== ids.length) throw new TypeError("review specialist obligation IDs must be unique");
  if (value.schemaVersion === 2) {
    const key = identityKey(normalizeIdentity(campaignIdentity));
    const expectedConsumers = new Map([
      ["builder_projection", `slice-builder:${key}`],
      ["campaign_terminalization", `slice-campaign:${key}`],
    ]);
    for (const specialist of value.specialists.filter(({selection}) => selection === "selected")) {
      for (const claim of specialist.requiredClaims) {
        if (claim.consumer !== expectedConsumers.get(claim.consumptionBoundary)) {
          throw new TypeError(`review selection ${claim.consumptionBoundary} consumer does not match campaign identity`);
        }
      }
    }
    const claimIds = value.specialists.flatMap(({requiredClaims}) => requiredClaims.map(({claimId}) => claimId));
    if (new Set(claimIds).size !== claimIds.length) throw new TypeError("review selection required claim IDs must be unique");
  }
  return value;
}

export function validateReviewSelectionSuccession(predecessor, successor, expectedSubject, campaignIdentity) {
  validateReviewSelection(predecessor, expectedSubject, campaignIdentity);
  validateReviewSelection(successor, expectedSubject, campaignIdentity);
  if (predecessor.schemaVersion !== 2 || successor.schemaVersion !== 2
      || predecessor.selectionId !== successor.selectionId
      || predecessor.owner !== successor.owner
      || digest(predecessor.subject) !== digest(successor.subject)
      || predecessor.specialists.length !== successor.specialists.length) {
    throw new TypeError("review selection successor rewrites immutable selection fields");
  }
  predecessor.specialists.forEach((prior, index) => {
    const next = successor.specialists[index];
    if (prior.obligationId !== next.obligationId || prior.skill !== next.skill
        || prior.selection !== next.selection || prior.requiredClaims.length !== next.requiredClaims.length) {
      throw new TypeError("review selection successor rewrites specialist disposition");
    }
    prior.requiredClaims.forEach((claim, claimIndex) => {
      const replacement = next.requiredClaims[claimIndex];
      if (claim.claimId !== replacement.claimId
          || claim.consumptionBoundary !== replacement.consumptionBoundary
          || claim.consumer !== replacement.consumer
          || digest(claim.subject) !== digest(replacement.subject)
          || digest(claim.acceptance) !== digest(replacement.acceptance)
          || replacement.profile.requiredRealization !== "claude-sonnet-5"
          || replacement.profile.continuity !== "retained") {
        throw new TypeError("review selection successor has invalid claim succession");
      }
    });
  });
  if (digest(predecessor) === digest(successor)) throw new TypeError("review selection successor must be a new revision");
  return successor;
}
