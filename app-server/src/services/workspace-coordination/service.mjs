import { randomUUID } from "node:crypto";
import {
  digest, freeze, normalizeLease, normalizeResource, requireOperationId, requireText,
  WORKSPACE_COORDINATION_SCHEMA_VERSION,
} from "./contract.mjs";

export class InMemoryWorkspaceCoordinationStore {
  constructor() { this.resources = new Map(); this.admissions = new Map(); }
  inspect(resourceKey) { return structuredClone(this.resources.get(resourceKey) ?? null); }
  acquire(request) {
    const current = this.resources.get(request.resource.key) ?? { generation: 0, lease: null };
    if (current.lease && Date.parse(current.lease.expiresAt) > Date.parse(request.issuedAt)) {
      return { status: "blocked", current: structuredClone(current.lease) };
    }
    const lease = { ...request, fencingToken: current.generation + 1 };
    this.resources.set(request.resource.key, { generation: lease.fencingToken, lease });
    return { status: "acquired", lease: structuredClone(lease) };
  }
  release(lease) {
    const current = this.resources.get(lease.resource.key);
    if (!current?.lease || current.lease.leaseId !== lease.leaseId
        || current.generation !== lease.fencingToken) return false;
    this.resources.set(lease.resource.key, { generation: current.generation, lease: null });
    return true;
  }
  withAdmission({ lease, operationId, admittedAt }, mutate) {
    if (this.admissions.has(operationId)) throw new Error("mutation operation id was already admitted");
    const current = this.resources.get(lease.resource.key);
    if (!current?.lease || current.generation !== lease.fencingToken
        || current.lease.leaseId !== lease.leaseId || current.lease.holder !== lease.holder) {
      throw new Error("resource lease is absent or superseded");
    }
    if (Date.parse(current.lease.expiresAt) <= Date.parse(admittedAt)) throw new Error("resource lease is expired");
    const result = mutate();
    if (result && typeof result.then === "function") throw new TypeError("mutation admission callback returned an asynchronous result");
    this.admissions.set(operationId, { lease: structuredClone(lease), admittedAt, result: structuredClone(result) });
    return result;
  }
}

export function createWorkspaceCoordinationService({
  store = new InMemoryWorkspaceCoordinationStore(), now = () => new Date().toISOString(),
  newId = randomUUID,
} = {}) {
  return Object.freeze({
    acquire({ resource, holder, intentId, ttlMs }) {
      const normalized = normalizeResource(resource);
      requireText(holder, "resource holder"); requireText(intentId, "resource intent");
      if (!Number.isSafeInteger(ttlMs) || ttlMs < 1) throw new TypeError("resource lease ttlMs must be positive");
      const issuedAt = now();
      if (Number.isNaN(Date.parse(issuedAt))) throw new TypeError("coordination clock must return an ISO timestamp");
      const request = {
        schemaVersion: WORKSPACE_COORDINATION_SCHEMA_VERSION, leaseId: newId(),
        resource: normalized, holder, intentId, issuedAt,
        expiresAt: new Date(Date.parse(issuedAt) + ttlMs).toISOString(),
      };
      const outcome = store.acquire(request);
      if (outcome.status === "blocked") return freeze({ status: "blocked", current: normalizeLease(outcome.current) });
      const lease = normalizeLease(outcome.lease);
      return freeze({ status: "acquired", lease, revision: digest(lease) });
    },
    inspect(resource) {
      const normalized = normalizeResource(resource);
      const state = store.inspect(normalized.key);
      return state ? freeze(structuredClone(state)) : null;
    },
    release(value) { return store.release(normalizeLease(value)); },
    admitMutation({ lease: value, operationId, mutate }) {
      const lease = normalizeLease(value); requireOperationId(operationId);
      if (typeof mutate !== "function") throw new TypeError("mutation admission requires a callback");
      if (mutate.constructor?.name === "AsyncFunction" || mutate[Symbol.toStringTag] === "AsyncFunction") {
        throw new TypeError("mutation admission callback must be synchronous");
      }
      const admittedAt = now();
      const result = store.withAdmission({ lease, operationId, admittedAt }, mutate);
      return freeze({
        status: "admitted", operationId, resource: lease.resource,
        fencingToken: lease.fencingToken, admittedAt, result: structuredClone(result),
      });
    },
  });
}
