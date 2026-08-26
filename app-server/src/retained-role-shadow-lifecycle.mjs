function requireFunction(value, label) {
  if (typeof value !== "function") throw new TypeError(`${label} must be a function`);
  return value;
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export class RetainedRoleShadowLifecycleRuntime {
  constructor({
    roleRuntime,
    lifecycleEvidence,
    pressureProjector,
    coordinatorForRole,
    projectionForTurn = async () => ({}),
  }) {
    if (!roleRuntime || typeof roleRuntime.deliverTurn !== "function"
        || typeof roleRuntime.adapter?.waitForTurnCompletion !== "function") {
      throw new TypeError("retained-role shadow runtime requires a manifest role runtime");
    }
    if (!lifecycleEvidence || typeof lifecycleEvidence.snapshot !== "function") {
      throw new TypeError("retained-role shadow runtime requires lifecycle evidence");
    }
    if (!pressureProjector || typeof pressureProjector.project !== "function") {
      throw new TypeError("retained-role shadow runtime requires a pressure projector");
    }
    this.roleRuntime = roleRuntime;
    this.lifecycleEvidence = lifecycleEvidence;
    this.pressureProjector = pressureProjector;
    this.coordinatorForRole = requireFunction(coordinatorForRole, "shadow coordinator resolver");
    this.projectionForTurn = requireFunction(projectionForTurn, "observed-context projection resolver");
  }

  async deliverTurn(turn) {
    const started = await this.startTurn(turn);
    const outcome = await started.completion;
    return freeze({
      delivery: started.delivery,
      completion: outcome.completion,
      shadow: outcome.shadow,
    });
  }

  async startTurn(turn) {
    const delivery = await this.roleRuntime.deliverTurn(turn);
    const completion = this.#completeTurn(turn, delivery);
    return freeze({ delivery, completion });
  }

  async #completeTurn(turn, delivery) {
    const completion = await this.roleRuntime.adapter.waitForTurnCompletion({
      threadId: delivery.threadId,
      turnId: delivery.turnId,
      replayedDelivery: delivery.replayedDelivery,
      signal: turn.signal,
    });
    const snapshot = this.lifecycleEvidence.snapshot(delivery.threadId);
    if (snapshot.latestTokenUsage?.turnId !== delivery.turnId) {
      return freeze({
        completion,
        shadow: { status: "not_observed", reason: "completed_turn_token_usage_unavailable" },
      });
    }
    const pressure = this.pressureProjector.project(snapshot);
    if (pressure.status !== "projected") {
      return freeze({ completion, shadow: pressure });
    }
    const roleInput = await this.projectionForTurn({
      turn,
      delivery,
      completion,
      lifecycleSnapshot: snapshot,
      pressure,
    });
    if (!roleInput || typeof roleInput !== "object" || Array.isArray(roleInput)) {
      throw new TypeError("observed-context projection resolver must return an object");
    }
    const coordinator = await this.coordinatorForRole(delivery.logicalRoleInstanceId);
    if (!coordinator || typeof coordinator.observe !== "function") {
      throw new TypeError("shadow coordinator resolver must return a coordinator");
    }
    const shadow = await coordinator.observe({
      episodeId: `turn:${delivery.logicalRoleInstanceId}:${delivery.turnId}:${pressure.observation.sequence}`,
      subject: {
        logicalRoleInstanceId: delivery.logicalRoleInstanceId,
        threadId: delivery.threadId,
        bindingRevision: delivery.binding.bindingRevision,
      },
      pressureObservation: pressure.observation,
      projection: roleInput.projection ?? null,
      sourceMaterials: roleInput.sourceMaterials ?? [],
      expectedPublicationRevision: roleInput.expectedPublicationRevision ?? null,
      previousLedgerEntry: roleInput.previousLedgerEntry ?? null,
      contextTelemetry: {
        retainedInputTokens: pressure.reportedInputTokens,
        reportedContextWindowTokens: pressure.contextWindow,
        ...(roleInput.contextTelemetry ?? {}),
      },
      signal: turn.signal,
    });
    return freeze({ completion, shadow });
  }
}
