import { attachCodexLifecycleEvidence } from "./codex-lifecycle-notification-source.mjs";
import { ContextLifecycleEvidenceCollector } from "./context-lifecycle-evidence.mjs";
import { ContextTransitionLeaseRuntime } from "./context-transition-lease.mjs";
import {
  lifecyclePressureSequenceFloor,
  restoreContextPressureController,
} from "./context-pressure-recovery.mjs";
import { LiveContextLifecycleCoordinator } from "./live-context-lifecycle-coordinator.mjs";
import { RetainedRoleLiveLifecycleRuntime } from "./retained-role-live-lifecycle.mjs";
import { ManifestRoleRuntime, RuntimeManifest } from "./runtime-manifest.mjs";
import { TokenUsagePressureProjector } from "./token-usage-pressure-projection.mjs";

function requiredFunction(value, label) {
  if (typeof value !== "function") throw new TypeError(`${label} must be a function`);
  return value;
}

export function createRetainedRoleLiveHost({
  adapter,
  manifest,
  episodeStore,
  transitionGate,
  inputCustody,
  pressureProfile,
  pressurePolicyForRole,
  inferenceRuntimeForRole,
  checkpointPublisherForRole,
  projectionForPreparation,
  transitionAt = ["replacement_candidate", "critical"],
  lifecycleRetentionLimit = 256,
  onLifecycleEvidenceError = null,
  now = () => new Date().toISOString(),
}) {
  if (!adapter || typeof adapter.deliverTurn !== "function") {
    throw new TypeError("retained-role live host requires an App Server adapter");
  }
  if (!(manifest instanceof RuntimeManifest)) {
    throw new TypeError("retained-role live host requires a runtime manifest");
  }
  if (!episodeStore || typeof episodeStore.receipts !== "function") {
    throw new TypeError("retained-role live host requires a lifecycle episode store");
  }
  if (!transitionGate || typeof transitionGate.beginPreparation !== "function"
      || typeof transitionGate.acquire !== "function"
      || typeof transitionGate.runTurnAdmission !== "function") {
    throw new TypeError("retained-role live host requires a transition gate");
  }
  if (adapter.transitionGate !== transitionGate) {
    throw new TypeError("retained-role live host and adapter must share one transition gate");
  }
  if (!inputCustody || typeof inputCustody.closeAdmission !== "function"
      || typeof inputCustody.admission !== "function"
      || typeof inputCustody.releaseAfterReconciliation !== "function") {
    throw new TypeError("retained-role live host requires durable input custody");
  }
  requiredFunction(pressurePolicyForRole, "role pressure policy resolver");
  requiredFunction(inferenceRuntimeForRole, "role inference runtime resolver");
  requiredFunction(checkpointPublisherForRole, "role checkpoint publisher resolver");
  requiredFunction(projectionForPreparation, "role preparation projection resolver");

  const sequenceFloor = lifecyclePressureSequenceFloor(episodeStore);
  const lifecycleEvidence = new ContextLifecycleEvidenceCollector({
    retentionLimit: lifecycleRetentionLimit,
    initialSequence: sequenceFloor,
  });
  const detachLifecycleEvidence = attachCodexLifecycleEvidence({
    adapter,
    collector: lifecycleEvidence,
    onError: onLifecycleEvidenceError,
  });
  const transitionRuntime = new ContextTransitionLeaseRuntime({
    gate: transitionGate,
    adapter,
    inputCustody,
  });
  const controllers = new Map();
  const coordinators = new Map();

  const pressureControllerForRole = async (logicalRoleInstanceId) => {
    let controller = controllers.get(logicalRoleInstanceId);
    if (controller) return controller;
    const recovery = restoreContextPressureController({
      policy: await pressurePolicyForRole(logicalRoleInstanceId),
      episodeStore,
      logicalRoleInstanceId,
    });
    controller = recovery.controller;
    controllers.set(logicalRoleInstanceId, controller);
    return controller;
  };

  const coordinatorForRole = async (logicalRoleInstanceId) => {
    let pending = coordinators.get(logicalRoleInstanceId);
    if (pending) return pending;
    pending = Promise.all([
      inferenceRuntimeForRole(logicalRoleInstanceId),
      checkpointPublisherForRole(logicalRoleInstanceId),
    ]).then(([inferenceRuntime, checkpointPublisher]) =>
      new LiveContextLifecycleCoordinator({
        transitionRuntime,
        inferenceRuntime,
        checkpointPublisher,
        transitionAt,
        projectionForPreparation: (input) => projectionForPreparation({
          ...input,
          lifecycleSnapshot: lifecycleEvidence.snapshot(input.subject.threadId),
        }),
      })
    ).catch((error) => {
      coordinators.delete(logicalRoleInstanceId);
      throw error;
    });
    coordinators.set(logicalRoleInstanceId, pending);
    return pending;
  };

  const runtime = new RetainedRoleLiveLifecycleRuntime({
    roleRuntime: new ManifestRoleRuntime({ adapter, manifest }),
    lifecycleEvidence,
    pressureProjector: new TokenUsagePressureProjector({ profile: pressureProfile, now }),
    pressureControllerForRole,
    coordinatorForRole,
  });

  return Object.freeze({
    runtime,
    lifecycleEvidence,
    transitionRuntime,
    sequenceFloor,
    pressureControllerForRole,
    coordinatorForRole,
    close: detachLifecycleEvidence,
  });
}
