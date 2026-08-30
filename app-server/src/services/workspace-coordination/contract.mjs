import { createHash } from "node:crypto";

export const WORKSPACE_COORDINATION_SCHEMA_VERSION = 1;
export const RESOURCE_TYPES = Object.freeze([
  "directory", "git-ref", "git-index", "port", "index", "review-budget", "database",
]);

export function requireRecord(value, label) {
  if (!value || Array.isArray(value) || typeof value !== "object") throw new TypeError(`${label} must be an object`);
  return value;
}

export function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(`${label} must be a non-empty string`);
  return value;
}

export function requireOperationId(value, label = "operation id") {
  requireText(value, label);
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(value)) throw new TypeError(`${label} must be a safe identifier`);
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

export function normalizeResource(value) {
  requireRecord(value, "resource");
  if (!RESOURCE_TYPES.includes(value.type)) throw new TypeError("resource type is unsupported");
  const resource = { type: value.type, id: requireText(value.id, "resource id") };
  return freeze({ ...resource, key: `${resource.type}:${resource.id}` });
}

export function normalizeLease(value) {
  requireRecord(value, "resource lease");
  const resource = normalizeResource(value.resource);
  if (!Number.isSafeInteger(value.fencingToken) || value.fencingToken < 1) throw new TypeError("resource lease fencing token must be positive");
  for (const field of ["issuedAt", "expiresAt"]) {
    requireText(value[field], `resource lease ${field}`);
    if (Number.isNaN(Date.parse(value[field]))) throw new TypeError(`resource lease ${field} must be an ISO timestamp`);
  }
  if (Date.parse(value.expiresAt) <= Date.parse(value.issuedAt)) throw new TypeError("resource lease must expire after issuance");
  return freeze({
    schemaVersion: WORKSPACE_COORDINATION_SCHEMA_VERSION,
    leaseId: requireText(value.leaseId, "resource lease id"), resource,
    holder: requireText(value.holder, "resource lease holder"),
    intentId: requireText(value.intentId, "resource lease intent"),
    fencingToken: value.fencingToken, issuedAt: value.issuedAt, expiresAt: value.expiresAt,
  });
}

