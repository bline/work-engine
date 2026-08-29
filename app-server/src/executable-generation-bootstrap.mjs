import { createHash } from "node:crypto";
import { access, readFile, readdir } from "node:fs/promises";
import path from "node:path";

import { ExecutableGenerationDispatchHost, GenerationBoundAppServerTransport } from "./executable-generation-dispatch.mjs";
import { ExecutableGenerationManager, InMemoryReplaceableSubstrateArbiter } from "./executable-generation-manager.mjs";
import { captureExecutableGenerationSnapshot } from "./executable-generation-snapshot.mjs";
import { FileExecutableGenerationStore } from "./executable-generation-store.mjs";
import { ForkedExecutableGenerationWorker } from "./executable-generation-worker.mjs";
import {
  loadRuntimeManifestDocument,
  hydrateRuntimeRequirements,
  projectRuntimeManifest,
} from "./runtime-manifest.mjs";
import { loadSemanticContextRuntimeProfile } from "./semantic-context-runtime-profile.mjs";

export const DEFAULT_EXECUTABLE_GENERATION_FILES = Object.freeze([
  "app-server/src/default-executable-generation-worker.mjs",
  "app-server/src/executable-generation-worker-runtime.mjs",
]);

export const DEFAULT_ROLE_EXECUTABLE_GENERATION_FILES = Object.freeze([
  "app-server/src/capabilities.mjs",
  "app-server/src/codex-app-server-adapter.mjs",
  "app-server/src/executable-generation-role-environment.mjs",
  "app-server/src/operator-switchboard.mjs",
  "app-server/src/request-context-input.mjs",
  "app-server/src/role-binding-registry.mjs",
  "app-server/src/runtime-manifest.mjs",
  "app-server/src/skill-resolver.mjs",
  "app-server/src/codex-app-server-inference.mjs",
  "app-server/src/codex-lifecycle-notification-source.mjs",
  "app-server/src/context-checkpoint-publication.mjs",
  "app-server/src/context-input-custody.mjs",
  "app-server/src/context-lifecycle-episode.mjs",
  "app-server/src/context-lifecycle-evidence.mjs",
  "app-server/src/context-lifecycle-ledger.mjs",
  "app-server/src/context-pressure-controller.mjs",
  "app-server/src/context-pressure-recovery.mjs",
  "app-server/src/context-transition-lease.mjs",
  "app-server/src/dynamic-tool-bridge.mjs",
  "app-server/src/continuation-state.mjs",
  "app-server/src/human-interaction-evaluation.mjs",
  "app-server/src/local-semantic-shadow-host.mjs",
  "app-server/src/local-semantic-live-host.mjs",
  "app-server/src/live-context-lifecycle-coordinator.mjs",
  "app-server/src/manifest-role-observed-context.mjs",
  "app-server/src/observed-context-projection.mjs",
  "app-server/src/retained-role-shadow-host.mjs",
  "app-server/src/retained-role-shadow-lifecycle.mjs",
  "app-server/src/retained-role-live-lifecycle.mjs",
  "app-server/src/retained-role-live-host.mjs",
  "app-server/src/semantic-context-inference.mjs",
  "app-server/src/shadow-context-lifecycle-coordinator.mjs",
  "app-server/src/sqlite-app-server-state.mjs",
  "app-server/src/token-usage-pressure-projection.mjs",
  "app-server/src/thread-snapshot-visible-materials.mjs",
  "app-server/src/services/product-development/artifact-root.mjs",
  "app-server/src/services/product-development/intake-delivery.mjs",
  "app-server/src/services/product-development/proposal-delivery.mjs",
  "skills/idea-intake/scripts/idea_intake.py",
  "skills/idea-intake/schemas/intake-record-v1.schema.json",
  "skills/proposal-packets/scripts/proposal_packets.py",
  "skills/proposal-packets/schemas/proposal-packet-v1.schema.json",
  "skills/proposal-packets/schemas/proposal-decision-v1.schema.json",
]);

const ROLE_ENVIRONMENT_CONFIG = "app-server/generated/executable-role-environment.json";

export const DEFAULT_EXECUTABLE_ENVIRONMENT_FINGERPRINT =
  "work-engine.app-server-transparent-environment-v1";
export const DEFAULT_EXECUTABLE_BOOTSTRAP_FINGERPRINT =
  "work-engine.app-server-bootstrap-ipc-v1";

export class ExecutableGenerationStartupError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = "ExecutableGenerationStartupError";
    this.code = code;
    this.details = Object.freeze(structuredClone(details));
  }
}

function record(snapshot, { environmentFingerprint, bootstrapFingerprint }) {
  return {
    generationId: snapshot.snapshotId,
    sourceDigest: snapshot.sourceDigest,
    environmentFingerprint,
    bootstrapFingerprint,
  };
}

function snapshotFromStored(generationsRoot, activeGeneration) {
  return {
    schemaVersion: 1,
    snapshotId: activeGeneration.generationId,
    sourceDigest: activeGeneration.sourceDigest,
    directory: path.join(generationsRoot, activeGeneration.generationId),
  };
}

function sameSnapshotIdentity(snapshot, activeGeneration) {
  return snapshot?.snapshotId === activeGeneration?.generationId
    && snapshot?.sourceDigest === activeGeneration?.sourceDigest;
}

function generationFromWorker(worker) {
  return {
    generationId: worker.generationId,
    sourceDigest: worker.sourceDigest,
    environmentFingerprint: worker.environmentFingerprint,
    bootstrapFingerprint: worker.bootstrapFingerprint,
  };
}

function startupError(code, message, stored, currentSnapshot, details = {}) {
  return new ExecutableGenerationStartupError(code, message, {
    durableGenerationId: stored.activeGeneration?.generationId ?? null,
    workspaceGenerationId: currentSnapshot?.snapshotId ?? null,
    ...details,
  });
}

function relativeToWorkspace(workspaceRoot, absolutePath, label) {
  const relative = path.relative(path.resolve(workspaceRoot), path.resolve(absolutePath));
  if (relative === "" || relative === ".." || relative.startsWith(`..${path.sep}`)
      || path.isAbsolute(relative)) {
    throw new TypeError(`${label} must be a file inside the executable workspace`);
  }
  return relative.split(path.sep).join("/");
}

async function roleEnvironmentSource({
  workspaceRoot,
  manifestPath,
  semanticContextProfilePath,
  files,
}) {
  const loaded = await loadRuntimeManifestDocument(manifestPath);
  const runtimeRequirementsByRole = await hydrateRuntimeRequirements(loaded.document, {
    baseDirectory: path.dirname(loaded.sourcePath),
    workspaceRoot,
  });
  const projected = projectRuntimeManifest(loaded.document, {
    baseDirectory: path.dirname(loaded.sourcePath),
    sourcePath: loaded.sourcePath,
    sourceSha256: loaded.sourceSha256,
    runtimeRequirementsByRole,
  });
  const manifestRelativePath = relativeToWorkspace(
    workspaceRoot,
    loaded.sourcePath,
    "runtime manifest",
  );
  const skills = [...new Set(Object.values(projected.roles).flatMap((role) =>
    role.skills.map((skill) => skill.path)
  ))].sort();
  const skillFiles = [];
  for (const skillPath of skills) {
    const relativePath = relativeToWorkspace(workspaceRoot, skillPath, "runtime skill");
    const content = await readFile(skillPath);
    skillFiles.push({
      path: relativePath,
      sha256: createHash("sha256").update(content).digest("hex"),
    });
  }
  const semanticContext = semanticContextProfilePath === null
    ? null
    : {
      profile: await loadSemanticContextRuntimeProfile(semanticContextProfilePath),
      relativePath: relativeToWorkspace(
        workspaceRoot,
        semanticContextProfilePath,
        "semantic context runtime profile",
      ),
    };
  const semanticRuntimeDependencies = [];
  if (semanticContext) {
    const dependencyRoot = path.join(workspaceRoot, "node_modules", "yaml");
    const entries = await readdir(dependencyRoot, { recursive: true, withFileTypes: true });
    for (const entry of entries) {
      if (!entry.isFile()) continue;
      semanticRuntimeDependencies.push(relativeToWorkspace(
        workspaceRoot,
        path.join(entry.parentPath, entry.name),
        "semantic context runtime dependency",
      ));
    }
  }
  return {
    files: [...new Set([
      ...files,
      ...DEFAULT_ROLE_EXECUTABLE_GENERATION_FILES,
      manifestRelativePath,
      ...skillFiles.map((skill) => skill.path),
      ...(semanticContext ? [semanticContext.relativePath] : []),
      ...semanticRuntimeDependencies,
    ])],
    generatedFiles: {
      [ROLE_ENVIRONMENT_CONFIG]: `${JSON.stringify({
        schemaVersion: 1,
        manifest: {
          document: loaded.document,
          identityBaseDirectory: path.dirname(loaded.sourcePath),
          relativePath: manifestRelativePath,
          sha256: loaded.sourceSha256,
          runtimeRequirementsByRole,
        },
        skillFiles,
        ...(semanticContext ? { semanticContext } : {}),
      }, null, 2)}\n`,
    },
  };
}

export async function createExecutableGenerationBootstrap({
  workspaceRoot,
  stateRoot,
  transport,
  workerCwd = process.cwd(),
  files = DEFAULT_EXECUTABLE_GENERATION_FILES,
  runtimeManifestPath = null,
  semanticContextProfilePath = null,
  semanticContextStatePath = path.join(path.resolve(stateRoot), "semantic-context.sqlite3"),
  roleBindingsPath = path.join(path.resolve(stateRoot), "role-bindings.json"),
  switchboardAttachmentPath = path.join(path.resolve(stateRoot), "switchboard-attachment.json"),
  configuredProviderFeatures = [],
  developmentArtifactRoot = path.resolve(workspaceRoot),
  entryRelativePath = "app-server/src/default-executable-generation-worker.mjs",
  environmentFingerprint = DEFAULT_EXECUTABLE_ENVIRONMENT_FINGERPRINT,
  bootstrapFingerprint = DEFAULT_EXECUTABLE_BOOTSTRAP_FINGERPRINT,
  substrateArbiter = new InMemoryReplaceableSubstrateArbiter(),
  workerRequestTimeoutMs = 10_000,
  workerDispatchTimeoutMs = 30 * 60_000,
} = {}) {
  if (!transport) throw new TypeError("executable generation bootstrap requires a transport");
  const source = runtimeManifestPath === null
    ? { files, generatedFiles: {} }
    : await roleEnvironmentSource({
      workspaceRoot,
      manifestPath: runtimeManifestPath,
      semanticContextProfilePath,
      files,
    });
  const generationsRoot = path.join(path.resolve(stateRoot), "generations");
  const store = new FileExecutableGenerationStore(
    path.join(path.resolve(stateRoot), "generation-state.json"),
  );
  const stored = await store.read();
  let currentSnapshot = null;
  let currentSnapshotFailureType = null;
  try {
    currentSnapshot = await captureExecutableGenerationSnapshot({
      workspaceRoot,
      generationsRoot,
      files: source.files,
      generatedFiles: source.generatedFiles,
    });
  } catch (error) {
    currentSnapshotFailureType = error instanceof Error ? error.name : typeof error;
    throw startupError(
      "workspace_snapshot_invalid",
      "current executable workspace could not be captured; refusing stale generation recovery",
      stored,
      null,
      {
        failureType: currentSnapshotFailureType,
        ...(typeof error?.code === "string" ? { snapshotErrorCode: error.code } : {}),
      },
    );
  }
  const workspaceDiffers = Boolean(stored.activeGeneration)
    && !sameSnapshotIdentity(currentSnapshot, stored.activeGeneration);
  const bootstrapDiffers = Boolean(stored.activeGeneration)
    && stored.activeGeneration.bootstrapFingerprint !== bootstrapFingerprint;
  const reconcileStartup = workspaceDiffers || bootstrapDiffers;
  const activeSnapshot = reconcileStartup || !stored.activeGeneration
    ? currentSnapshot
    : snapshotFromStored(generationsRoot, stored.activeGeneration);

  const spawnSnapshot = async (snapshot, generationRecord = record(snapshot, {
    environmentFingerprint,
    bootstrapFingerprint,
  })) => {
    const roleConfigPath = path.join(snapshot.directory, ...ROLE_ENVIRONMENT_CONFIG.split("/"));
    let roleEnvironment = false;
    try {
      await access(roleConfigPath);
      roleEnvironment = true;
    } catch {}
    return ForkedExecutableGenerationWorker.spawn({
      entryPath: path.join(snapshot.directory, ...entryRelativePath.split("/")),
      cwd: workerCwd,
      requestTimeoutMs: workerRequestTimeoutMs,
      dispatchTimeoutMs: workerDispatchTimeoutMs,
      env: {
        ...process.env,
        WORK_ENGINE_EXECUTABLE_GENERATION: JSON.stringify(generationRecord),
        ...(roleEnvironment ? {
          WORK_ENGINE_EXECUTABLE_ROLE_ENVIRONMENT: "1",
          WORK_ENGINE_EXECUTABLE_SNAPSHOT_ROOT: snapshot.directory,
          WORK_ENGINE_ROLE_BINDINGS_PATH: path.resolve(roleBindingsPath),
          WORK_ENGINE_SWITCHBOARD_ATTACHMENT_PATH: path.resolve(switchboardAttachmentPath),
          ...(semanticContextProfilePath ? {
            WORK_ENGINE_SEMANTIC_CONTEXT_STATE_PATH: path.resolve(semanticContextStatePath),
          } : {}),
          WORK_ENGINE_CONFIGURED_PROVIDER_FEATURES: JSON.stringify(configuredProviderFeatures),
          WORK_ENGINE_LIVE_REPOSITORY_ROOT: path.resolve(workspaceRoot),
          WORK_ENGINE_DEVELOPMENT_ARTIFACT_ROOT: path.resolve(developmentArtifactRoot),
        } : {}),
      },
    });
  };

  const activeRecord = reconcileStartup || !stored.activeGeneration
    ? record(activeSnapshot, {
      environmentFingerprint,
      bootstrapFingerprint,
    })
    : stored.activeGeneration;
  let activeWorker;
  try {
    activeWorker = await spawnSnapshot(activeSnapshot, activeRecord);
  } catch (error) {
    throw startupError(
      "candidate_invalid",
      "selected executable generation could not be built",
      stored,
      currentSnapshot,
      { stage: "building", failureType: error instanceof Error ? error.name : typeof error },
    );
  }
  try {
    const validation = await activeWorker.validate();
    if (!validation.valid) {
      throw startupError(
        "candidate_invalid",
        "selected executable generation did not validate",
        stored,
        currentSnapshot,
        { stage: "validating" },
      );
    }
    if (stored.activeGeneration
        && activeWorker.environmentFingerprint !== stored.activeGeneration.environmentFingerprint) {
      throw startupError(
        "environment_migration_required",
        "current executable workspace changes the bound role environment",
        stored,
        currentSnapshot,
      );
    }
    await activeWorker.activate();
    const startupReconciliation = reconcileStartup
      ? await store.activateStartupCandidate({
        expectedActiveGenerationId: stored.activeGeneration.generationId,
        successor: generationFromWorker(activeWorker),
      })
      : null;
    const manager = await ExecutableGenerationManager.create({
      activeGeneration: activeWorker,
      store,
      substrateArbiter,
      snapshotter: async () => captureExecutableGenerationSnapshot({
        workspaceRoot,
        generationsRoot,
        files: source.files,
        generatedFiles: source.generatedFiles,
      }),
      candidateBuilder: async (snapshot) => spawnSnapshot(snapshot),
    });
    const dispatchHost = new ExecutableGenerationDispatchHost(manager);
    return Object.freeze({
      manager,
      dispatchHost,
      transport: new GenerationBoundAppServerTransport({ transport, dispatchHost }),
      currentSnapshot,
      currentSnapshotFailureType,
      startupSelection: Object.freeze({
        outcome: reconcileStartup
          ? startupReconciliation.outcome
          : stored.activeGeneration ? "durable_active_current" : "initialized_current",
        durableGenerationId: stored.activeGeneration?.generationId ?? null,
        workspaceGenerationId: currentSnapshot.snapshotId,
        selectedGenerationId: activeWorker.generationId,
        reconciliationId: startupReconciliation?.reconciliationId ?? null,
      }),
      statePath: store.filePath,
      close: (options) => manager.close(options),
    });
  } catch (error) {
    await activeWorker.dispose();
    if (error instanceof ExecutableGenerationStartupError) throw error;
    throw startupError(
      "candidate_invalid",
      "selected executable generation failed during startup",
      stored,
      currentSnapshot,
      { failureType: error instanceof Error ? error.name : typeof error },
    );
  }
}
