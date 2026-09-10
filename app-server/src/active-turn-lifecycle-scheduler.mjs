function required(value, label) { if (typeof value !== "function") throw new TypeError(`${label} is required`); return value; }
function keyOf(value) { return [value.logicalRoleInstanceId, value.bindingRevision, value.threadId,
  value.turnId, value.triggeringObservationId].join("\u0000"); }

export class ActiveTurnLifecycleScheduler {
  constructor({ store, transitionGate, reserve, now = () => new Date().toISOString(),
    claimTimeoutMs = 60_000 }) {
    for (const method of ["scheduleActiveTurnLifecycle", "claimActiveTurnLifecycle",
      "reclaimActiveTurnLifecycle", "abortActiveTurnLifecycle", "completeActiveTurnLifecycle",
      "recoverableActiveTurnLifecycles"]) {
      required(store?.[method]?.bind(store), `active-turn lifecycle store ${method}`);
    }
    if (!Number.isSafeInteger(claimTimeoutMs) || claimTimeoutMs < 1) {
      throw new TypeError("active-turn lifecycle claim timeout must be positive");
    }
    required(reserve, "lifecycle reserve");
    required(transitionGate?.mintLifecycleReservePermit?.bind(transitionGate), "lifecycle reserve permit minter");
    this.store = store; this.transitionGate = transitionGate; this.reserve = reserve;
    this.now = now; this.claimTimeoutMs = claimTimeoutMs; this.tails = new Map();
  }
  #serial(key, operation) {
    const prior = this.tails.get(key) ?? Promise.resolve();
    const current = prior.then(operation, operation);
    this.tails.set(key, current.catch(() => {}));
    return current.finally(() => { if (this.tails.get(key) === current) this.tails.delete(key); });
  }
  observe(input) { return this.#serial(keyOf(input), () => this.#run(this.store.scheduleActiveTurnLifecycle(input))); }
  recover() { return Promise.all(this.store.recoverableActiveTurnLifecycles(this.now()).map((item) =>
    this.#serial(keyOf(item), () => this.#run(item)))); }
  async #run(scheduled) {
    if (["reconciled", "recovery_required"].includes(scheduled.status)) return scheduled;
    if (scheduled.status === "preparing") {
      const observedAt = this.now();
      if (scheduled.nextEligibleAt > observedAt) return scheduled;
      scheduled = this.store.reclaimActiveTurnLifecycle({identity: scheduled.identity,
        expectedRevision: scheduled.revision, reclaimedAt: observedAt,
        error: {name: "LifecycleClaimExpired",
          message: "preparing lifecycle claim expired before reconciliation"}});
      if (scheduled.status === "recovery_required") return scheduled;
    }
    const claimedAt = this.now();
    const claimExpiresAt = new Date(Date.parse(claimedAt) + this.claimTimeoutMs).toISOString();
    const claimed = this.store.claimActiveTurnLifecycle({ identity: scheduled.identity,
      expectedRevision: scheduled.revision, claimedAt, claimExpiresAt });
    if (claimed.status === "replayed") return claimed;
    try {
      const permit = this.transitionGate.mintLifecycleReservePermit({ ...claimed.identity,
        disposition: claimed.disposition, lifecycleRevision: claimed.revision });
      const result = await this.reserve(permit);
      return this.store.completeActiveTurnLifecycle({ identity: claimed.identity,
        expectedRevision: claimed.revision, result, completedAt: this.now() });
    } catch (error) {
      const aborted = this.store.abortActiveTurnLifecycle({ identity: claimed.identity,
        expectedRevision: claimed.revision, error: { name: error.name, message: error.message },
        abortedAt: this.now() });
      if (aborted.status === "aborted_retryable") return this.#run(aborted);
      return aborted;
    }
  }
}
