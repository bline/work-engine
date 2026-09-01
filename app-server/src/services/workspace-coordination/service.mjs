import { randomUUID } from "node:crypto";
import {
  digest, freeze, normalizeLease, normalizeResource, requireOperationId, requireRecord, requireText,
  WORKSPACE_COORDINATION_SCHEMA_VERSION,
} from "./contract.mjs";

export class InMemoryWorkspaceCoordinationStore {
  constructor() { this.resources = new Map(); this.admissions = new Map(); this.publications = new Map(); }
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
  inspectAdmission(operationId) {
    const value = this.admissions.get(operationId);
    return value ? structuredClone({
      status: "admitted", operationId, resource: value.lease.resource,
      fencingToken: value.lease.fencingToken, admittedAt: value.admittedAt,
      result: value.result,
    }) : null;
  }
  loadPublication(operationId) {
    return structuredClone(this.publications.get(operationId) ?? null);
  }
  savePublication({ operationId, expectedRevision, revision, record }) {
    const current = this.publications.get(operationId) ?? null;
    if (current?.revision === revision) return structuredClone(current);
    if ((current?.revision ?? null) !== expectedRevision) {
      throw new Error("prepared publication revision conflict");
    }
    const next = { revision, record: structuredClone(record) };
    this.publications.set(operationId, next);
    return structuredClone(next);
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
    inspectAdmission(operationId) {
      requireOperationId(operationId);
      const value = store.inspectAdmission(operationId);
      return value ? freeze(structuredClone(value)) : null;
    },
    loadPublication(operationId) {
      requireOperationId(operationId);
      const value = store.loadPublication(operationId);
      return value ? freeze(structuredClone(value)) : null;
    },
    savePublication({ operationId, expectedRevision = null, record }) {
      requireOperationId(operationId); requireRecord(record, "prepared publication record");
      if (record.operationId !== operationId) {
        throw new Error("prepared publication operation identity mismatch");
      }
      if (expectedRevision !== null && !/^[0-9a-f]{64}$/.test(expectedRevision)) {
        throw new TypeError("prepared publication expected revision must be SHA-256 or null");
      }
      const frozen = freeze(structuredClone(record));
      return freeze(store.savePublication({
        operationId, expectedRevision, revision: digest(frozen), record: frozen,
      }));
    },
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
