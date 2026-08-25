import { attachCodexLifecycleEvidence } from "./codex-lifecycle-notification-source.mjs";
import { ContextLifecycleEvidenceCollector } from "./context-lifecycle-evidence.mjs";
import {
  lifecyclePressureSequenceFloor,
  restoreContextPressureController,
} from "./context-pressure-recovery.mjs";
import { RetainedRoleShadowLifecycleRuntime } from "./retained-role-shadow-lifecycle.mjs";
import { ManifestRoleRuntime, RuntimeManifest } from "./runtime-manifest.mjs";
import { ShadowContextLifecycleCoordinator } from "./shadow-context-lifecycle-coordinator.mjs";
import { TokenUsagePressureProjector } from "./token-usage-pressure-projection.mjs";

function requiredFunction(value, label) {
  if (typeof value !== "function") throw new TypeError(`${label} must be a function`);
  return value;
}

export function createRetainedRoleShadowHost({
  adapter,
  manifest,
  episodeStore,
  pressureProfile,
  pressurePolicyForRole,
  scheduleForRole,
  inferenceRuntimeForRole,
  checkpointPublisherForRole = async () => null,
  projectionForTurn,
  lifecycleRetentionLimit = 256,
  onLifecycleEvidenceError = null,
  now = () => new Date().toISOString(),
  monotonicNow = () => performance.now(),
}) {
  if (!adapter || typeof adapter.deliverTurn !== "function") {
    throw new TypeError("retained-role shadow host requires an App Server adapter");
  }
  if (!(manifest instanceof RuntimeManifest)) {
    throw new TypeError("retained-role shadow host requires a runtime manifest");
  }
  if (!episodeStore || typeof episodeStore.receipts !== "function"
      || typeof episodeStore.get !== "function" || typeof episodeStore.append !== "function") {
    throw new TypeError("retained-role shadow host requires a lifecycle episode store");
  }
  requiredFunction(pressurePolicyForRole, "role pressure policy resolver");
  requiredFunction(scheduleForRole, "role shadow schedule resolver");
  requiredFunction(inferenceRuntimeForRole, "role inference runtime resolver");
  requiredFunction(checkpointPublisherForRole, "role checkpoint publisher resolver");
  requiredFunction(now, "retained-role shadow host clock");
  requiredFunction(monotonicNow, "retained-role shadow host monotonic clock");

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
  const coordinators = new Map();
  const recoveries = new Map();

  const coordinatorForRole = async (logicalRoleInstanceId) => {
    let pending = coordinators.get(logicalRoleInstanceId);
    if (pending) return pending;
    pending = (async () => {
      const policy = await pressurePolicyForRole(logicalRoleInstanceId);
      const recovery = restoreContextPressureController({
        policy,
        episodeStore,
        logicalRoleInstanceId,
      });
      const [schedule, inferenceRuntime, checkpointPublisher] = await Promise.all([
        scheduleForRole(logicalRoleInstanceId),
        inferenceRuntimeForRole(logicalRoleInstanceId),
        checkpointPublisherForRole(logicalRoleInstanceId),
      ]);
      const coordinator = new ShadowContextLifecycleCoordinator({
        logicalRoleInstanceId,
        pressureController: recovery.controller,
        inferenceRuntime,
        checkpointPublisher,
        episodeStore,
        schedule,
        now,
        monotonicNow,
      });
      recoveries.set(logicalRoleInstanceId, recovery);
      return coordinator;
    })().catch((error) => {
      coordinators.delete(logicalRoleInstanceId);
      throw error;
    });
    coordinators.set(logicalRoleInstanceId, pending);
    return pending;
  };

  const runtime = new RetainedRoleShadowLifecycleRuntime({
    roleRuntime: new ManifestRoleRuntime({ adapter, manifest }),
    lifecycleEvidence,
    pressureProjector: new TokenUsagePressureProjector({ profile: pressureProfile, now }),
    coordinatorForRole,
    ...(projectionForTurn ? { projectionForTurn } : {}),
  });

  return Object.freeze({
    runtime,
    lifecycleEvidence,
    sequenceFloor,
    coordinatorForRole,
    recoveryForRole: (logicalRoleInstanceId) => recoveries.get(logicalRoleInstanceId) ?? null,
    close: detachLifecycleEvidence,
  });
}
