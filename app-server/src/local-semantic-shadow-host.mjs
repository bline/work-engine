import { createHash, generateKeyPairSync } from "node:crypto";

import { CodexAppServerInferenceCapability } from "./codex-app-server-inference.mjs";
import { projectManifestRoleObservedContext } from "./manifest-role-observed-context.mjs";
import { createRetainedRoleShadowHost } from "./retained-role-shadow-host.mjs";
import { SemanticContextInferenceRuntime } from "./semantic-context-inference.mjs";
import { openSqliteAppServerStateStore } from "./sqlite-app-server-state.mjs";

const EXPECTED_NEXT_WORK =
  "No successor work was supplied with this completed operator turn. The attached role awaits its next operator message.";

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

function shadowScheduleInput(schedule) {
  return {
    schemaVersion: schedule.schemaVersion,
    inspectAt: [...schedule.inspectAt],
    publishAcceptedCheckpoint: schedule.publishAcceptedCheckpoint,
  };
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function defaultSigning() {
  const keyPair = generateKeyPairSync("ed25519");
  const publicDer = keyPair.publicKey.export({ type: "spki", format: "der" });
  const fingerprint = createHash("sha256").update(publicDer).digest("hex");
  return {
    keyId: `local-shadow:${fingerprint}`,
    publicKey: keyPair.publicKey,
    privateKey: keyPair.privateKey,
  };
}

export async function createLocalSemanticShadowHost({
  adapter,
  manifest,
  stateFilePath,
  profile,
  inferenceModel = null,
  inferenceThreadOptions = {},
  signing = defaultSigning(),
  onLifecycleEvidenceError = null,
}) {
  if (!adapter || typeof adapter.runEphemeralTurn !== "function") {
    throw new TypeError("local semantic shadow host requires an App Server adapter");
  }
  if (!profile || profile.schemaVersion !== 1 || !profile.source?.sha256) {
    throw new TypeError("local semantic shadow host requires a projected runtime profile");
  }
  if (profile.shadowSchedule?.publishAcceptedCheckpoint !== false) {
    throw new TypeError("local semantic shadow host cannot publish checkpoints");
  }
  const stateStore = await openSqliteAppServerStateStore({
    filePath: text(stateFilePath, "local semantic shadow state path"),
  });
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
  let host;
  try {
    host = createRetainedRoleShadowHost({
      adapter,
      manifest,
      episodeStore: stateStore,
      pressureProfile: pressureProfileInput(profile.pressureProfile),
      pressurePolicyForRole: async () => pressurePolicyInput(profile.pressurePolicy),
      scheduleForRole: async () => shadowScheduleInput(profile.shadowSchedule),
      inferenceRuntimeForRole: async () => inferenceRuntime,
      projectionForTurn: async ({ turn, delivery, lifecycleSnapshot }) =>
        projectManifestRoleObservedContext({
          delivery,
          lifecycleSnapshot,
          visibleMaterials: [{
            identity: `operator-turn:${delivery.logicalRoleInstanceId}:${delivery.turnId}`,
            origin: "human",
            trustClass: "human_authority_input",
            instructionApplicability: "contract_defined",
            contentRef: {
              kind: "thread-item",
              reference: `operator-turn:${delivery.threadId}:${delivery.turnId}`,
            },
            content: text(turn.text, "local semantic shadow operator turn text"),
          }],
          expectedNextWork: {
            reference: `expected-next-work:${delivery.logicalRoleInstanceId}:${delivery.turnId}`,
            content: EXPECTED_NEXT_WORK,
          },
          sourceInventoryCompleteness: "partial",
          omissions: [{
            scope: "provider-effective-prompt",
            reason: "the provider does not expose the exact effective model input",
          }],
          signing: {
            componentId: "work-engine.local-semantic-shadow-projector",
            buildRevision: profile.source.sha256,
            keyId: signing.keyId,
            privateKey: signing.privateKey,
          },
        }),
      onLifecycleEvidenceError,
    });
  } catch (error) {
    stateStore.close();
    throw error;
  }
  let closed = false;
  return Object.freeze({
    mode: "semantic-shadow",
    profile,
    runtime: host.runtime,
    lifecycleEvidence: host.lifecycleEvidence,
    episodeStore: stateStore,
    signingKeyId: signing.keyId,
    close() {
      if (closed) return;
      closed = true;
      host.close();
      stateStore.close();
    },
  });
}
