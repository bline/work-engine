import { createHash, generateKeyPairSync } from "node:crypto";

import { CodexAppServerInferenceCapability } from "./codex-app-server-inference.mjs";
import { ContextCheckpointPublisher } from "./context-checkpoint-publication.mjs";
import { ContextInputCustodyController } from "./context-input-custody.mjs";
import { projectManifestRoleObservedContext } from "./manifest-role-observed-context.mjs";
import { createRetainedRoleLiveHost } from "./retained-role-live-host.mjs";
import { SemanticContextInferenceRuntime } from "./semantic-context-inference.mjs";
import { openSqliteAppServerStateStore } from "./sqlite-app-server-state.mjs";
import { projectThreadSnapshotVisibleMaterials } from "./thread-snapshot-visible-materials.mjs";

const EXPECTED_NEXT_WORK =
  "No successor work was supplied with this completed operator turn. The attached role awaits its next operator message.";

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonical(value[key])}`
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function revision(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function referenceKey(value) {
  return JSON.stringify([value.reference, value.sha256]);
}

function pressureProfileInput(profile) {
  return {
    schemaVersion: profile.schemaVersion,
    usageField: profile.usageField,
    windowField: profile.windowField,
    rounding: profile.rounding,
    saturation: profile.saturation,
  };
}

function pressurePolicyInput(policy) {
  return {
    schemaVersion: policy.schemaVersion,
    unit: policy.unit,
    approaching: { ...policy.approaching },
    replacementCandidate: { ...policy.replacementCandidate },
    critical: { ...policy.critical },
  };
}

function defaultSigning() {
  const keyPair = generateKeyPairSync("ed25519");
  const publicDer = keyPair.publicKey.export({ type: "spki", format: "der" });
  const fingerprint = createHash("sha256").update(publicDer).digest("hex");
  return {
    keyId: `local-live:${fingerprint}`,
    publicKey: keyPair.publicKey,
    privateKey: keyPair.privateKey,
  };
}

export async function createLocalSemanticLiveHost({
  adapter,
  manifest,
  transitionGate,
  stateFilePath,
  profile,
  inferenceModel = null,
  inferenceThreadOptions = {},
  signing = defaultSigning(),
  onLifecycleEvidenceError = null,
}) {
  if (!profile || profile.schemaVersion !== 1 || profile.mode !== "live"
      || !profile.liveSchedule) {
    throw new TypeError("local semantic live host requires an explicit live runtime profile");
  }
  const stateStore = await openSqliteAppServerStateStore({ filePath: stateFilePath });
  const inputCustody = new ContextInputCustodyController({ store: stateStore });
  const authorityBySource = new Map();
  const compiler = new CodexAppServerInferenceCapability({
    adapter,
    producer: "work-engine.semantic-context-compiler",
    version: "1",
    model: inferenceModel,
    threadOptions: inferenceThreadOptions,
  });
  const verifier = new CodexAppServerInferenceCapability({
    adapter,
    producer: "work-engine.semantic-context-verifier",
    version: "1",
    model: inferenceModel,
    threadOptions: inferenceThreadOptions,
  });
  const inferenceRuntime = new SemanticContextInferenceRuntime({
    compiler,
    verifier,
    resolvePublicKey: (keyId) => keyId === signing.keyId ? signing.publicKey : null,
  });
  const publisher = new ContextCheckpointPublisher({
    store: stateStore,
    resolvePublicKey: (keyId) => keyId === signing.keyId ? signing.publicKey : null,
    revalidateAuthority: async ({ sourceRevision, references }) => {
      const authority = authorityBySource.get(sourceRevision);
      const requested = references.map(referenceKey);
      const current = authority && requested.every((key) => authority.referenceKeys.has(key));
      return {
        status: current ? "current" : "invalid",
        authorityRevision: authority?.authorityRevision ?? revision({ sourceRevision, requested }),
        checkedReferences: references,
        evidenceRefs: [sourceRevision],
      };
    },
  });
  let host;
  try {
    host = createRetainedRoleLiveHost({
      adapter,
      manifest,
      episodeStore: stateStore,
      transitionGate,
      inputCustody,
      pressureProfile: pressureProfileInput(profile.pressureProfile),
      pressurePolicyForRole: async () => pressurePolicyInput(profile.pressurePolicy),
      inferenceRuntimeForRole: async () => inferenceRuntime,
      checkpointPublisherForRole: async () => publisher,
      transitionAt: profile.liveSchedule.transitionAt,
      projectionForPreparation: async ({
        subject, attestation, projectionContext, lifecycleSnapshot,
      }) => {
        const delivery = projectionContext?.delivery;
        if (!delivery) throw new TypeError("live projection requires completed role delivery");
        const snapshot = await adapter.readThreadContextSnapshot({ threadId: subject.threadId });
        const projected = await projectManifestRoleObservedContext({
          delivery: { ...delivery, turnId: attestation.delivery.turnId },
          lifecycleSnapshot,
          visibleMaterials: projectThreadSnapshotVisibleMaterials(snapshot, {
            excludedTurnIds: [attestation.delivery.turnId],
          }),
          expectedNextWork: {
            reference: `expected-next-work:${subject.logicalRoleInstanceId}:operator-input`,
            content: EXPECTED_NEXT_WORK,
          },
          sourceInventoryCompleteness: "complete",
          signing: {
            componentId: "work-engine.local-semantic-live-projector",
            buildRevision: profile.source.sha256,
            keyId: signing.keyId,
            privateKey: signing.privateKey,
          },
        });
        const referenceKeys = projected.sourceMaterials
          .map(({ contentRef }) => ({ reference: contentRef.reference, sha256: contentRef.sha256 }))
          .map(referenceKey).sort();
        const authorityRevision = revision({
          sourceRevision: projected.projection.sourceRevision,
          referenceKeys,
        });
        authorityBySource.set(projected.projection.sourceRevision, {
          authorityRevision,
          referenceKeys: new Set(referenceKeys),
        });
        const previous = stateStore.snapshot(subject.logicalRoleInstanceId);
        const nextFence = {
          logicalRoleInstanceId: subject.logicalRoleInstanceId,
          threadId: subject.threadId,
          bindingRevision: subject.bindingRevision,
          sourceRevision: projected.projection.sourceRevision,
          authorityRevision,
          publicationRevision: previous?.fence.publicationRevision ?? null,
          ledgerRevision: previous?.fence.ledgerRevision ?? null,
        };
        if (previous === null) {
          stateStore.initializeCheckpointFence(nextFence);
        } else {
          const updated = stateStore.compareAndSwapCheckpointFence({
            expectedFence: previous.fence,
            nextFence,
          });
          if (updated.status !== "committed") {
            throw new Error(`live checkpoint fence update failed: ${updated.reason}`);
          }
        }
        return {
          ...projected,
          expectedPublicationRevision: nextFence.publicationRevision,
          previousLedgerEntry: previous?.ledgerEntry ?? null,
        };
      },
      onLifecycleEvidenceError,
    });
  } catch (error) {
    stateStore.close();
    throw error;
  }
  let closed = false;
  return Object.freeze({
    mode: "semantic-live",
    profile,
    runtime: host.runtime,
    lifecycleEvidence: host.lifecycleEvidence,
    episodeStore: stateStore,
    inputCustody,
    transitionGate,
    close() {
      if (closed) return;
      closed = true;
      host.close();
      stateStore.close();
    },
  });
}
