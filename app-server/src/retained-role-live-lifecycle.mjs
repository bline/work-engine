function requiredFunction(value, label) {
  if (typeof value !== "function") throw new TypeError(`${label} must be a function`);
  return value;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

function lifecycleFailure(error) {
  return freeze({
    status: "failed",
    reason: "post_turn_lifecycle_failed",
    error: {
      name: error instanceof Error ? error.name : "Error",
      message: error instanceof Error ? error.message : "post-turn lifecycle processing failed",
      ...(typeof error?.code === "string" ? { code: error.code } : {}),
    },
  });
}

export class RetainedRoleLiveLifecycleRuntime {
  constructor({
    roleRuntime,
    lifecycleEvidence,
    pressureProjector,
    pressureControllerForRole,
    coordinatorForRole,
    activeTurnScheduler = null,
  }) {
    if (!roleRuntime || typeof roleRuntime.deliverTurn !== "function"
        || typeof roleRuntime.adapter?.waitForTurnCompletion !== "function") {
      throw new TypeError("retained-role live runtime requires a manifest role runtime");
    }
    if (!lifecycleEvidence || typeof lifecycleEvidence.snapshot !== "function") {
      throw new TypeError("retained-role live runtime requires lifecycle evidence");
    }
    if (!pressureProjector || typeof pressureProjector.project !== "function") {
      throw new TypeError("retained-role live runtime requires a pressure projector");
    }
    this.roleRuntime = roleRuntime;
    this.lifecycleEvidence = lifecycleEvidence;
    this.pressureProjector = pressureProjector;
    this.pressureControllerForRole = requiredFunction(
      pressureControllerForRole,
      "live pressure controller resolver",
    );
    this.coordinatorForRole = requiredFunction(coordinatorForRole, "live coordinator resolver");
    this.activeTurnScheduler = activeTurnScheduler;
    this.activeTurns = new Map();
  }

  async deliverTurn(turn) {
    const started = await this.startTurn(turn);
    const outcome = await started.completion;
    return freeze({
      delivery: started.delivery,
      completion: outcome.completion,
      lifecycle: outcome.lifecycle,
    });
  }

  async startTurn(turn) {
    const delivery = await this.roleRuntime.deliverTurn(turn);
    this.activeTurns.set(delivery.threadId, { turn, delivery });
    return freeze({ delivery, completion: this.#completeTurn(turn, delivery) });
  }

  async observeLifecycleObservation(observation) {
    if (observation?.observationType !== "token_usage" || !this.activeTurnScheduler) return null;
    const active = this.activeTurns.get(observation.threadId);
    if (!active || active.delivery.turnId !== observation.turnId) return null;
    const snapshot = this.lifecycleEvidence.snapshot(observation.threadId);
    const pressure = this.pressureProjector.project(snapshot);
    if (pressure.status !== "projected") return pressure;
    const controller = await this.pressureControllerForRole(active.delivery.logicalRoleInstanceId);
    const decision = controller.observe(pressure.observation);
    if (!["replacement_candidate", "critical"].includes(decision.disposition)) return decision;
    return this.activeTurnScheduler.observe({
      logicalRoleInstanceId: active.delivery.logicalRoleInstanceId,
      bindingRevision: active.delivery.binding.bindingRevision,
      threadId: active.delivery.threadId, turnId: active.delivery.turnId,
      triggeringObservationId: pressure.observation.observationId,
      disposition: decision.disposition, observedAt: observation.observedAt,
    });
  }

  async #completeTurn(turn, delivery) {
    const completion = await this.roleRuntime.adapter.waitForTurnCompletion({
      threadId: delivery.threadId,
      turnId: delivery.turnId,
      replayedDelivery: delivery.replayedDelivery,
      signal: turn.signal,
    });
    this.activeTurns.delete(delivery.threadId);
    let lifecycle;
    try {
      const lifecycleSnapshot = this.lifecycleEvidence.snapshot(delivery.threadId);
      if (lifecycleSnapshot.latestTokenUsage?.turnId !== delivery.turnId) {
        return freeze({
          completion,
          lifecycle: { status: "not_observed", reason: "completed_turn_token_usage_unavailable" },
        });
      }
      const pressure = this.pressureProjector.project(lifecycleSnapshot);
      if (pressure.status !== "projected") {
        return freeze({ completion, lifecycle: pressure });
      }
      const pressureController = await this.pressureControllerForRole(
        delivery.logicalRoleInstanceId,
      );
      if (!pressureController || typeof pressureController.observe !== "function") {
        throw new TypeError("live pressure controller resolver must return a controller");
      }
      const pressureDecision = pressureController.observe(pressure.observation);
      const coordinator = await this.coordinatorForRole(delivery.logicalRoleInstanceId);
      if (!coordinator || typeof coordinator.run !== "function") {
        throw new TypeError("live coordinator resolver must return a coordinator");
      }
      lifecycle = await coordinator.run({
        episodeId:
          `turn:${delivery.logicalRoleInstanceId}:${delivery.turnId}:${pressure.observation.sequence}`,
        subject: {
          logicalRoleInstanceId: delivery.logicalRoleInstanceId,
          threadId: delivery.threadId,
          bindingRevision: delivery.binding.bindingRevision,
        },
        pressureDisposition: pressureDecision.disposition,
        role: delivery.roleProjection.role,
        skills: delivery.roleProjection.skills,
        projectionContext: {
          turn,
          delivery,
          completion,
          lifecycleSnapshot,
          pressure,
          pressureDecision,
        },
        signal: turn.signal,
      });
    } catch (error) {
      lifecycle = lifecycleFailure(error);
    }
    return freeze({ completion, lifecycle });
  }
}
