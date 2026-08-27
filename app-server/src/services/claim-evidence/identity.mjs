import { createHash } from "node:crypto";

import { ClaimEvidenceError } from "./contract.mjs";

function fail(message) {
  throw new ClaimEvidenceError(message);
}

function compareUnicode(left, right) {
  const a = Array.from(left, (character) => character.codePointAt(0));
  const b = Array.from(right, (character) => character.codePointAt(0));
  for (let index = 0; index < Math.min(a.length, b.length); index += 1) {
    if (a[index] !== b[index]) return a[index] - b[index];
  }
  return a.length - b.length;
}

export function validateTransportSafeJson(value, label = "value") {
  if (value === null || typeof value === "boolean" || typeof value === "string") return;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) fail(`${label} number must be finite`);
    if (Number.isInteger(value) && !Number.isSafeInteger(value)) {
      fail(`${label} integer is not lossless across JSON transports`);
    }
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => validateTransportSafeJson(item, `${label}[${index}]`));
    return;
  }
  if (typeof value === "object" && Object.getPrototypeOf(value) === Object.prototype) {
    for (const [key, item] of Object.entries(value)) {
      validateTransportSafeJson(item, `${label}.${key}`);
    }
    return;
  }
  fail(`${label} is not a JSON value`);
}

function serialize(value) {
  if (value === null || typeof value === "boolean" || typeof value === "number" || typeof value === "string") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) return `[${value.map(serialize).join(",")}]`;
  return `{${Object.keys(value).sort(compareUnicode).map((key) => `${JSON.stringify(key)}:${serialize(value[key])}`).join(",")}}`;
}

export function canonicalJson(value) {
  validateTransportSafeJson(value);
  return `${serialize(value)}\n`;
}

export function digest(value) {
  return createHash("sha256").update(canonicalJson(value), "utf8").digest("hex");
}

export function stableClaimId(subject) {
  const keys = Object.keys(subject ?? {}).sort();
  const expected = ["content_set", "evidence_baseline", "namespace", "stable_subject_id", "subject_kind"];
  if (JSON.stringify(keys) !== JSON.stringify(expected)) fail("subject has missing or unknown fields");
  const identity = {};
  for (const key of ["namespace", "subject_kind", "stable_subject_id"]) {
    if (typeof subject[key] !== "string" || subject[key].length === 0) fail(`subject.${key} must be a nonempty string`);
    identity[key] = subject[key];
  }
  return `claim-v1@${digest(identity)}`;
}

export function revisionId(revisionWithoutId) {
  return `${revisionWithoutId.claim_id}@${digest(revisionWithoutId)}`;
}

