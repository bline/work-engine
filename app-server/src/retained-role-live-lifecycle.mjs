function requiredFunction(value, label) {
  if (typeof value !== "function") throw new TypeError(`${label} must be a function`);
  return value;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export class RetainedRoleLiveLifecycleRuntime {
  constructor({
    roleRuntime,
    lifecycleEvidence,
    pressureProjector,
    pressureControllerForRole,
    coordinatorForRole,
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
    return freeze({ delivery, completion: this.#completeTurn(turn, delivery) });
  }

  async #completeTurn(turn, delivery) {
    const completion = await this.roleRuntime.adapter.waitForTurnCompletion({
      threadId: delivery.threadId,
      turnId: delivery.turnId,
      replayedDelivery: delivery.replayedDelivery,
      signal: turn.signal,
    });
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
    const lifecycle = await coordinator.run({
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
    return freeze({ completion, lifecycle });
  }
}
