import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash, generateKeyPairSync, randomUUID } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  CodexAppServerAdapter,
  CodexAppServerInferenceCapability,
  ContextCheckpointPublisher,
  ContextLifecycleEvidenceCollector,
  ContextTransitionLeaseRuntime,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  InMemoryContextCheckpointPublicationStore,
  InMemoryContextTransitionLeaseGate,
  ManifestRoleRuntime,
  MODEL_CONTEXT_REPLACEMENT_CAPABILITY,
  ObservableAppServerTransport,
  PINNED_PROTOCOL,
  StdioJsonRpcTransport,
  StrategicPlannerRuntime,
  SemanticContextInferenceRuntime,
  assertCompatibleServer,
  attachCodexLifecycleEvidence,
  createExecutableGenerationBootstrap,
  createLocalSemanticShadowHost,
  formatAppServerProtocolEvent,
  loadRuntimeManifest,
  openSqliteAppServerStateStore,
  projectManifestRoleObservedContext,
  projectThreadSnapshotVisibleMaterials,
  projectSemanticContextRuntimeProfile,
} from "../src/index.mjs";

const enabled = process.env.WORK_ENGINE_APP_SERVER_INTEGRATION === "1";
const transitionEnabled = process.env.WORK_ENGINE_CONTEXT_TRANSITION_INTEGRATION === "1";
const traceEnabled = process.env.WORK_ENGINE_APP_SERVER_TRACE === "1";
const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "../..");
const LIVE_SHADOW_USER_TEXT = "Assess the current App Server foundation and identify the next bounded step without changing repository state.";
const LIVE_TRANSITION_NEXT_WORK = "After reconciliation, await the next human-supervised strategic-planning request without inferring new authority.";

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(value[key])}`
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function shaRevision(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function semanticReferenceKey(value) {
  return JSON.stringify([value.reference, value.sha256]);
}

function candidateAuthorityReferences(candidate) {
  const values = [
    candidate.objective.authorityRef,
    candidate.authorizedNextAction.authorityRef,
    ...candidate.authorityDependencies.canonicalRecords,
    ...candidate.humanInteractions
      .map((interaction) => interaction.durableConsequenceRef)
      .filter(Boolean),
    ...candidate.humanInteractions.map((interaction) => interaction.sourceRef),
  ];
  const unique = new Map(values.map((value) => [semanticReferenceKey(value), value]));
  return [...unique.values()].sort((left, right) =>
    semanticReferenceKey(left).localeCompare(semanticReferenceKey(right))
  );
}

function spawnAppServer(options = {}, testContext = null) {
  const transport = StdioJsonRpcTransport.spawn(options);
  if (!traceEnabled) return transport;
  return new ObservableAppServerTransport({
    transport,
    onEvent: (event) => {
      const line = `[app-server] ${formatAppServerProtocolEvent(event)}`;
      if (testContext) testContext.diagnostic(line);
      else console.error(line);
    },
  });
}

async function sha256(filePath) {
  return createHash("sha256").update(await readFile(filePath)).digest("hex");
}

function parseContextLifecycleProbe(outputText, expectedPhase) {
  assert.equal(typeof outputText, "string", `${expectedPhase} probe returned no output`);
  const output = JSON.parse(outputText.trim());
  assert.equal(output.phase, expectedPhase);
  assert.equal(typeof output.marker, "string");
  assert.equal(typeof output.first_context_window_id, "string");
  assert.equal(typeof output.current_context_window_id, "string");
  assert.equal(
    output.previous_context_window_id === null
      || typeof output.previous_context_window_id === "string",
    true,
  );
  return output;
}

test("pinned Codex App Server completes the initialize handshake", { skip: !enabled }, async (t) => {
  const transport = spawnAppServer({}, t);
  t.after(() => transport.close());
  const response = await transport.request("initialize", {
    clientInfo: {
      name: "work-engine-integration-test",
      title: "Work Engine Integration Test",
      version: "0.1.0",
    },
    capabilities: { experimentalApi: false, requestAttestation: false },
  });
  assert.equal(assertCompatibleServer(response), PINNED_PROTOCOL.codexCliVersion);
  transport.notify("initialized");
  if (traceEnabled) {
    assert.equal(transport instanceof ObservableAppServerTransport, true);
    const trace = transport.snapshot();
    assert.equal(trace.events.length >= 4, true);
    assert.deepEqual(trace.observationErrors, []);
  }
  assert.equal(response.platformFamily.length > 0, true);
});

test("live manifest role can read request-bound context", {
  skip: !enabled,
  timeout: 120_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-manifest-role."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillRoot = path.join(directory, "skills");
  const skillDirectory = path.join(skillRoot, "context-echo");
  await mkdir(skillDirectory, { recursive: true });
  await writeFile(path.join(skillDirectory, "SKILL.md"), `---
name: context-echo
description: Prove that request-bound context is visible in a live model turn.
---

# Context Echo

Read the request-context entry named \`work-engine.probe\`. Return exactly its
value and no other text.
`, "utf8");
  const manifestPath = path.join(directory, "runtime-manifest.yaml");
  await writeFile(manifestPath, `
schema_version: 1
manifest_id: work-engine.live-context-probe
roles:
  context-echo:
    contract: skills/context-echo/SKILL.md
    developer_instructions: |-
      This is a bounded visibility probe. Read the request-context entry named
      work-engine.probe and return exactly its value with no other text.
    thread_options:
      cwd: ${JSON.stringify(ROOT)}
      approval_policy: never
      sandbox: read-only
    skills:
      - {name: context-echo, path: skills/context-echo/SKILL.md}
`, "utf8");

  const transport = spawnAppServer({ cwd: ROOT }, t);
  t.after(() => transport.close());
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([skillRoot]),
  });
  await adapter.initialize();
  const runtime = new ManifestRoleRuntime({
    adapter,
    manifest: await loadRuntimeManifest(manifestPath),
  });
  const probeValue = `visible-${randomUUID()}`;
  const delivery = await runtime.deliverTurn({
    roleId: "context-echo",
    instanceId: "live-probe",
    clientUserMessageId: `context-probe-${randomUUID()}`,
    text: "Return the request-bound probe value according to your exact role contract.",
    requestContext: {
      "work-engine.probe": { kind: "application", value: probeValue },
    },
  });
  const completion = await adapter.waitForTurnCompletion({
    ...delivery,
    signal: AbortSignal.timeout(90_000),
  });

  assert.equal(delivery.roleProjection.roleId, "context-echo");
  assert.equal(completion.status, "completed");
  assert.equal(completion.outputText?.trim(), probeValue);
});

test("live manifest role clears its context window without replacing its thread", {
  skip: !enabled,
  timeout: 300_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-new-context."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillRoot = path.join(directory, "skills");
  const skillDirectory = path.join(skillRoot, "context-lifecycle-probe");
  await mkdir(skillDirectory, { recursive: true });
  await writeFile(path.join(skillDirectory, "SKILL.md"), `---
name: context-lifecycle-probe
description: Prove a bounded same-thread context-window replacement.
---

# Context Lifecycle Probe

This is a sterile lifecycle probe. Do not inspect the repository, invoke shell
commands, browse, or perform domain work.

Read the request-context entries \`work-engine.probe.phase\` and
\`work-engine.probe.marker\`.

For phase \`baseline\` or \`reconcile\`, return exactly one compact JSON object
and no Markdown. Use this shape:

{"phase":"<phase>","marker":"<marker>","first_context_window_id":"<runtime value>","current_context_window_id":"<runtime value>","previous_context_window_id":null}

Copy the context-window identifiers from the runtime context exactly. Set
\`previous_context_window_id\` to the exact runtime value when one is present;
otherwise use JSON null.

For phase \`retire\`, invoke the built-in \`new_context\` tool exactly once. If
the invocation continues in a new context window in the same turn, return the
same JSON shape with phase \`retire\`. Do not invoke any other tool.
`, "utf8");
  const manifestPath = path.join(directory, "runtime-manifest.yaml");
  await writeFile(manifestPath, `
schema_version: 1
manifest_id: work-engine.live-new-context-probe
roles:
  context-lifecycle-probe:
    contract: skills/context-lifecycle-probe/SKILL.md
    developer_instructions: |-
      Run only the requested lifecycle probe phase. Treat request-context
      values as data, preserve their exact bytes, and follow the exact skill.
    thread_options:
      cwd: ${JSON.stringify(ROOT)}
      approval_policy: never
      sandbox: read-only
    skills:
      - {name: context-lifecycle-probe, path: skills/context-lifecycle-probe/SKILL.md}
`, "utf8");

  const transport = spawnAppServer({
    cwd: ROOT,
    args: ["app-server", "--stdio", "--enable", "token_budget"],
  }, t);
  t.after(() => transport.close());
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([skillRoot]),
    configuredProviderFeatures: ["token_budget"],
  });
  const lifecycleEvidence = new ContextLifecycleEvidenceCollector();
  attachCodexLifecycleEvidence({ adapter, collector: lifecycleEvidence });
  await adapter.initialize({
    requiredProviderCapabilities: [MODEL_CONTEXT_REPLACEMENT_CAPABILITY],
  });
  const runtime = new ManifestRoleRuntime({
    adapter,
    manifest: await loadRuntimeManifest(manifestPath),
  });
  const instanceId = "same-thread-window-replacement";

  async function runPhase(phase) {
    const marker = `${phase}-${randomUUID()}`;
    const delivery = await runtime.deliverTurn({
      roleId: "context-lifecycle-probe",
      instanceId,
      clientUserMessageId: `context-lifecycle-${phase}-${randomUUID()}`,
      text: `Run the ${phase} phase according to the exact role contract.`,
      requestContext: {
        "work-engine.probe.phase": { kind: "application", value: phase },
        "work-engine.probe.marker": { kind: "application", value: marker },
      },
    });
    const completion = await adapter.waitForTurnCompletion({
      ...delivery,
      signal: AbortSignal.timeout(90_000),
    });
    return { completion, delivery, marker };
  }

  const baselineTurn = await runPhase("baseline");
  const baseline = parseContextLifecycleProbe(
    baselineTurn.completion.outputText,
    "baseline",
  );
  assert.equal(baseline.marker, baselineTurn.marker);
  assert.equal(
    baseline.current_context_window_id,
    baseline.first_context_window_id,
  );

  const retirementTurn = await runPhase("retire");
  assert.equal(retirementTurn.completion.status, "completed");

  const reconciliationTurn = await runPhase("reconcile");
  const reconciliation = parseContextLifecycleProbe(
    reconciliationTurn.completion.outputText,
    "reconcile",
  );
  assert.equal(reconciliation.marker, reconciliationTurn.marker);

  assert.equal(retirementTurn.delivery.threadId, baselineTurn.delivery.threadId);
  assert.equal(reconciliationTurn.delivery.threadId, baselineTurn.delivery.threadId);
  assert.notEqual(
    reconciliation.current_context_window_id,
    baseline.current_context_window_id,
  );
  assert.equal(
    reconciliation.previous_context_window_id,
    baseline.current_context_window_id,
  );

  const lifecycleSnapshot = lifecycleEvidence.snapshot(baselineTurn.delivery.threadId);
  assert.equal(lifecycleSnapshot.latestTokenUsage?.observationType, "token_usage");
  assert.equal(lifecycleSnapshot.transitionSignals.some((observation) =>
    observation.turnId === retirementTurn.delivery.turnId
    && observation.details.signal === "context_compaction"
    && observation.details.classification === "unclassified"
  ), true);
  const effectItemTypes = new Set([
    "commandExecution",
    "dynamicToolCall",
    "fileChange",
    "imageGeneration",
    "mcpToolCall",
    "subAgentActivity",
    "webSearch",
  ]);
  assert.deepEqual(
    retirementTurn.completion.turn.items
      .filter((item) => effectItemTypes.has(item?.type))
      .map((item) => item.type),
    [],
  );
});

test("live strategic planner publishes, clears, and reconciles one stable context snapshot", {
  skip: !transitionEnabled,
  timeout: 900_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-planner-transition."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const transport = spawnAppServer({
    cwd: ROOT,
    args: ["app-server", "--stdio", "--enable", "token_budget"],
  }, t);
  t.after(() => transport.close());
  const gate = new InMemoryContextTransitionLeaseGate();
  const registry = new FileRoleBindingRegistry(
    path.join(directory, "bindings.json"),
    { transitionGate: gate },
  );
  const adapter = new CodexAppServerAdapter({
    transport,
    registry,
    skillResolver: await ExactSkillResolver.create([path.join(ROOT, "skills")]),
    configuredProviderFeatures: ["token_budget"],
    transitionGate: gate,
  });
  const lifecycleEvidence = new ContextLifecycleEvidenceCollector();
  attachCodexLifecycleEvidence({ adapter, collector: lifecycleEvidence });
  await adapter.initialize({
    requiredProviderCapabilities: [MODEL_CONTEXT_REPLACEMENT_CAPABILITY],
  });
  const manifest = await loadRuntimeManifest(
    path.join(ROOT, "app-server/runtime-manifest.yaml"),
  );
  const roles = new ManifestRoleRuntime({ adapter, manifest });
  const instanceId = `live-transition-${randomUUID()}`;
  const domainDelivery = await roles.deliverTurn({
    roleId: "strategic-planner",
    instanceId,
    clientUserMessageId: `live-transition-domain-${randomUUID()}`,
    text: "Acknowledge readiness for one bounded context-lifecycle transition.",
  });
  const domainCompletion = await adapter.waitForTurnCompletion({
    ...domainDelivery,
    signal: AbortSignal.timeout(180_000),
  });
  assert.equal(domainCompletion.status, "completed");

  const transition = new ContextTransitionLeaseRuntime({ gate, adapter });
  const preparation = await transition.beginPreparation({
    logicalRoleInstanceId: domainDelivery.logicalRoleInstanceId,
    threadId: domainDelivery.threadId,
    bindingRevision: domainDelivery.binding.bindingRevision,
  });
  const attestation = await transition.attestContextWindow({
    role: domainDelivery.roleProjection.role,
    preparation: preparation.preparation,
    clientUserMessageId: `live-transition-preparation-${randomUUID()}`,
    signal: AbortSignal.timeout(180_000),
  });
  assert.equal(attestation.validation.status, "accepted");
  assert.equal(attestation.attestation.status, "attested");
  assert.equal(attestation.contextSnapshot.threadId, domainDelivery.threadId);
  assert.equal(attestation.contextSnapshot.type, "work-engine.codex-effective-context-snapshot");
  const semanticThreadSnapshot = await adapter.readThreadContextSnapshot({
    threadId: domainDelivery.threadId,
  });

  const keys = generateKeyPairSync("ed25519");
  const keyId = `live-transition-projector-${randomUUID()}`;
  const lifecycleSnapshot = lifecycleEvidence.snapshot(domainDelivery.threadId);
  const projected = await projectManifestRoleObservedContext({
    delivery: {
      ...domainDelivery,
      turnId: attestation.delivery.turnId,
    },
    lifecycleSnapshot,
    visibleMaterials: projectThreadSnapshotVisibleMaterials(semanticThreadSnapshot, {
      excludedTurnIds: [attestation.delivery.turnId],
    }),
    expectedNextWork: {
      reference: `expected-next-work:${domainDelivery.logicalRoleInstanceId}:reconciled`,
      content: LIVE_TRANSITION_NEXT_WORK,
    },
    sourceInventoryCompleteness: "complete",
    signing: {
      componentId: "work-engine.live-transition-projector",
      buildRevision: `test:${randomUUID()}`,
      keyId,
      privateKey: keys.privateKey,
    },
  });
  const compiler = new CodexAppServerInferenceCapability({
    adapter,
    producer: "work-engine.live-transition-compiler",
    version: "1",
    threadOptions: {
      cwd: directory,
      approvalPolicy: "never",
      sandbox: "read-only",
    },
  });
  const verifier = new CodexAppServerInferenceCapability({
    adapter,
    producer: "work-engine.live-transition-verifier",
    version: "1",
    threadOptions: {
      cwd: directory,
      approvalPolicy: "never",
      sandbox: "read-only",
    },
  });
  const inspection = await new SemanticContextInferenceRuntime({
    compiler,
    verifier,
    resolvePublicKey: (candidateKeyId) => candidateKeyId === keyId ? keys.publicKey : null,
  }).inspect(projected);
  assert.equal(
    inspection.verification.disposition,
    "accepted",
    JSON.stringify({
      disposition: inspection.verification.disposition,
      checks: inspection.verification.checks.map(({ name, status, rationale }) => ({
        name,
        status,
        rationale,
      })),
      blockers: inspection.verification.blockers,
      uncertainty: inspection.verification.uncertainty,
    }),
  );

  const references = candidateAuthorityReferences(inspection.candidate);
  const availableReferences = new Set(projected.sourceMaterials.map(({ contentRef }) =>
    semanticReferenceKey(contentRef)
  ));
  assert.equal(references.every((reference) =>
    availableReferences.has(semanticReferenceKey(reference))
  ), true);
  const authorityRevision = shaRevision(canonicalJson(references));
  const store = new InMemoryContextCheckpointPublicationStore([{
    logicalRoleInstanceId: domainDelivery.logicalRoleInstanceId,
    threadId: domainDelivery.threadId,
    bindingRevision: domainDelivery.binding.bindingRevision,
    sourceRevision: projected.projection.sourceRevision,
    authorityRevision,
    publicationRevision: null,
    ledgerRevision: null,
  }]);
  const publisher = new ContextCheckpointPublisher({
    store,
    resolvePublicKey: (candidateKeyId) => candidateKeyId === keyId ? keys.publicKey : null,
    revalidateAuthority: async ({ references: requestedReferences }) => ({
      status: requestedReferences.every((reference) =>
        availableReferences.has(semanticReferenceKey(reference))
      ) ? "current" : "invalid",
      authorityRevision,
      checkedReferences: requestedReferences,
      evidenceRefs: [projected.projection.sourceRevision],
    }),
  });
  const publication = await publisher.publish({
    projection: projected.projection,
    candidate: inspection.candidate,
    verification: inspection.verification,
  });
  assert.equal(publication.status, "published");

  const lease = await transition.promotePreparation({
    preparation: preparation.preparation,
    publication: publication.publication,
    ledgerEntry: publication.ledgerEntry,
    expectedFence: {
      ...publication.currentFence,
      predecessorContextWindowId:
        attestation.validation.receipt.current_context_window_id,
    },
  });
  assert.equal(
    lease.lease.subject.preparedContextRevision,
    attestation.contextSnapshot.contextRevision,
  );
  const result = await transition.retireAndReconcile({
    role: domainDelivery.roleProjection.role,
    lease: lease.lease,
    retirementClientUserMessageId: `live-transition-retire-${randomUUID()}`,
    rehydrationClientUserMessageId: `live-transition-reconcile-${randomUUID()}`,
    receiptNonce: randomUUID(),
    skills: domainDelivery.roleProjection.skills,
    signal: AbortSignal.timeout(240_000),
  });
  assert.equal(result.reconciliation.status, "reconciled");
  assert.equal(result.retirementDelivery.threadId, domainDelivery.threadId);
  assert.equal(result.rehydrationDelivery.threadId, domainDelivery.threadId);
  assert.equal(
    result.validation.receipt.previous_context_window_id,
    attestation.validation.receipt.current_context_window_id,
  );
  assert.notEqual(
    result.validation.receipt.current_context_window_id,
    attestation.validation.receipt.current_context_window_id,
  );
});

test("live strategic planner returns an exact request-bound handoff", {
  skip: !enabled,
  timeout: 300_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-planner."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const transport = spawnAppServer({ cwd: ROOT }, t);
  t.after(() => transport.close());
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([path.join(ROOT, "skills")]),
  });
  await adapter.initialize();
  const planner = new StrategicPlannerRuntime({
    adapter,
    manifest: await loadRuntimeManifest(path.join(ROOT, "app-server/runtime-manifest.yaml")),
  });
  const objective = "Prove one live request-bound strategic-planner handoff.";
  const roadmapDigest = await sha256(path.join(ROOT, "roadmap.md"));
  const lifecyclePath = path.join(
    ROOT,
    "app-server/docs/semantic-context-lifecycle-manager.md",
  );
  const lifecycleDigest = await sha256(lifecyclePath);
  const repositoryRevision = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: ROOT,
    encoding: "utf8",
  }).trim();
  const result = await planner.requestReview({
    instanceId: "live-integration",
    clientUserMessageId: `live-planner-${randomUUID()}`,
    strategicObjective: objective,
    evidenceCutoff: {
      roadmapRevision: `sha256:${roadmapDigest}`,
      repositoryRevision: `git:${repositoryRevision}`,
    },
    canonicalReferences: [{
      owner: "app-server-foundation-plan",
      reference: "app-server/docs/semantic-context-lifecycle-manager.md",
      revision: `sha256:${lifecycleDigest}`,
      integritySha256: lifecycleDigest,
      freshnessRule: "exact_content_sha256",
    }],
    continuity: "initialized",
    signal: AbortSignal.timeout(270_000),
  });

  assert.equal(result.logicalRoleInstanceId, "strategic-planner:live-integration");
  assert.equal(result.completion.status, "completed");
  assert.equal(result.handoff.schema_version, 1);
  assert.equal(result.handoff.strategic_objective, objective);
  assert.equal(result.handoff.continuity, "initialized");
  assert.equal(
    result.handoff.evidence_cutoff.repository_revision,
    `git:${repositoryRevision}`,
  );
});

test("live strategic planner completes one non-clearing semantic shadow inspection", {
  skip: !enabled,
  timeout: 600_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-shadow."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const transport = spawnAppServer({ cwd: ROOT }, t);
  t.after(() => transport.close());
  const adapter = new CodexAppServerAdapter({
    transport,
    registry: new FileRoleBindingRegistry(path.join(directory, "bindings.json")),
    skillResolver: await ExactSkillResolver.create([path.join(ROOT, "skills")]),
  });
  await adapter.initialize();
  const profile = projectSemanticContextRuntimeProfile({
    schema_version: 1,
    profile_id: "live-shadow.integration",
    pressure_profile: {
      usage_field: "last.totalTokens",
      window_field: "modelContextWindow",
      rounding: "floor",
      saturation: "clamp_10000",
    },
    pressure_policy: {
      unit: "basis_points",
      approaching: { enter: 1, exit: 0 },
      replacement_candidate: { enter: 2, exit: 1 },
      critical: { enter: 10_000, exit: 9_999 },
    },
    shadow_schedule: {
      inspect_at: ["replacement_candidate"],
      publish_accepted_checkpoint: false,
    },
  }, { sha256: "c".repeat(64) });
  const userText = LIVE_SHADOW_USER_TEXT;
  const host = await createLocalSemanticShadowHost({
    adapter,
    manifest: await loadRuntimeManifest(path.join(ROOT, "app-server/runtime-manifest.yaml")),
    stateFilePath: path.join(directory, "app-server.sqlite3"),
    profile,
    inferenceThreadOptions: {
      cwd: directory,
      approvalPolicy: "never",
      sandbox: "read-only",
    },
  });
  t.after(host.close);

  const result = await host.runtime.deliverTurn({
    roleId: "strategic-planner",
    instanceId: `live-shadow-${randomUUID()}`,
    clientUserMessageId: `live-shadow-turn-${randomUUID()}`,
    text: userText,
    requestContext: {
      "work-engine.shadow-objective": { kind: "application", value: userText },
      "work-engine.shadow-mode": { kind: "application", value: "observe_only" },
    },
    signal: AbortSignal.timeout(570_000),
  });

  assert.equal(result.shadow.episode.pressure.disposition, "replacement_candidate");
  if (result.shadow.status === "failed") {
    t.diagnostic(`semantic shadow inference failed closed: ${JSON.stringify(
      result.shadow.episode.failure,
    )}`);
    assert.equal(result.shadow.episode.failure.stage, "inference");
    assert.equal(result.shadow.episode.inference.status, "failed");
    assert.equal(result.shadow.episode.checkpoint.reason, "inference_failed");
  } else {
    assert.equal(result.shadow.status, "recorded");
    assert.equal(["accepted", "rejected", "unresolved"].includes(
      result.shadow.episode.inference.status,
    ), true);
    assert.notEqual(result.shadow.episode.inference.compiler.inferenceId,
      result.shadow.episode.inference.verifier.inferenceId);
    assert.deepEqual(result.shadow.episode.checkpoint, {
      status: "not_attempted",
      checkpointRevision: null,
      ledgerRevision: null,
      reason: result.shadow.episode.inference.status === "accepted"
        ? "shadow_publication_disabled"
        : "verification_not_accepted",
    });
  }
  assert.deepEqual(result.shadow.episode.transition, {
    status: "not_requested",
    retirementAttempted: false,
  });
});

test("live executable host records one non-clearing strategic-planner shadow inspection", {
  skip: !enabled,
  timeout: 900_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-hosted-shadow."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const transport = spawnAppServer({ cwd: ROOT }, t);
  t.after(() => transport.close());
  const stateRoot = path.join(directory, "host-state");
  const semanticStatePath = path.join(stateRoot, "semantic-context.sqlite3");
  const bootstrap = await createExecutableGenerationBootstrap({
    workspaceRoot: ROOT,
    stateRoot,
    workerCwd: ROOT,
    transport,
    runtimeManifestPath: path.join(ROOT, "app-server/runtime-manifest.yaml"),
    semanticContextProfilePath: path.join(
      ROOT,
      "app-server/tests/fixtures/semantic-context-host-inspection-profile.yaml",
    ),
    semanticContextStatePath: semanticStatePath,
    roleBindingsPath: path.join(stateRoot, "role-bindings.json"),
    switchboardAttachmentPath: path.join(stateRoot, "switchboard-attachment.json"),
  });
  t.after(() => bootstrap.close({ abandonActiveWork: true }));

  const completedTurns = new Map();
  const completionWaiters = new Map();
  bootstrap.transport.onNotification((notification) => {
    if (notification.method !== "turn/completed") return;
    const turnId = notification.params?.turn?.id;
    if (typeof turnId !== "string") return;
    const waiter = completionWaiters.get(turnId);
    if (waiter) {
      completionWaiters.delete(turnId);
      waiter(notification.params.turn);
    } else {
      completedTurns.set(turnId, notification.params.turn);
    }
  });
  const waitForCompletion = (turnId) => {
    const completed = completedTurns.get(turnId);
    if (completed) {
      completedTurns.delete(turnId);
      return Promise.resolve(completed);
    }
    return new Promise((resolve) => completionWaiters.set(turnId, resolve));
  };

  await bootstrap.transport.request("initialize", {
    clientInfo: {
      name: "work-engine-live-hosted-shadow",
      title: "Work Engine Live Hosted Shadow",
      version: "0.1.0",
    },
    capabilities: { experimentalApi: false, requestAttestation: false },
  });
  bootstrap.transport.notify("initialized");
  const shell = await bootstrap.transport.request("thread/start", { cwd: ROOT });
  const instanceId = `live-hosted-shadow-${randomUUID()}`;

  const attachment = await bootstrap.transport.request("turn/start", {
    threadId: shell.thread.id,
    clientUserMessageId: `live-hosted-attach-${randomUUID()}`,
    input: [{
      type: "text",
      text: `:we attach strategic-planner:${instanceId}`,
      text_elements: [],
    }],
  });
  const attachmentCompletion = await waitForCompletion(attachment.turn.id);
  assert.equal(attachmentCompletion.status, "completed");

  const delivery = await bootstrap.transport.request("turn/start", {
    threadId: shell.thread.id,
    clientUserMessageId: `live-hosted-turn-${randomUUID()}`,
    input: [{ type: "text", text: LIVE_SHADOW_USER_TEXT, text_elements: [] }],
  });
  const completion = await waitForCompletion(delivery.turn.id);
  assert.equal(completion.status, "completed", JSON.stringify(completion.error));

  const lifecycleStore = await openSqliteAppServerStateStore({
    filePath: semanticStatePath,
  });
  t.after(() => lifecycleStore.close());
  const receipts = lifecycleStore.receipts({
    logicalRoleInstanceId: `strategic-planner:${instanceId}`,
  });
  assert.equal(receipts.length, 1);
  const [episode] = receipts;
  assert.equal(episode.pressure.disposition, "replacement_candidate");
  assert.equal(["accepted", "rejected", "unresolved"].includes(
    episode.inference.status,
  ), true);
  assert.notEqual(
    episode.inference.compiler.inferenceId,
    episode.inference.verifier.inferenceId,
  );
  assert.equal(episode.checkpoint.reason,
    episode.inference.status === "accepted"
      ? "shadow_publication_disabled"
      : "verification_not_accepted");
  assert.equal(episode.checkpoint.status, "not_attempted");
  assert.equal(episode.checkpoint.checkpointRevision, null);
  assert.equal(episode.checkpoint.ledgerRevision, null);
  assert.deepEqual(episode.transition, {
    status: "not_requested",
    retirementAttempted: false,
  });
  assert.deepEqual(bootstrap.manager.snapshot().admissions, []);
});

test("live executable host publishes, clears, and reconciles through the switchboard", {
  skip: !transitionEnabled,
  timeout: 900_000,
}, async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-live-hosted-transition."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const transport = spawnAppServer({
    cwd: ROOT,
    args: ["app-server", "--stdio", "--enable", "token_budget"],
  }, t);
  t.after(() => transport.close());
  const stateRoot = path.join(directory, "host-state");
  const semanticStatePath = path.join(stateRoot, "semantic-context.sqlite3");
  const bootstrap = await createExecutableGenerationBootstrap({
    workspaceRoot: ROOT,
    stateRoot,
    workerCwd: ROOT,
    transport,
    runtimeManifestPath: path.join(ROOT, "app-server/runtime-manifest.yaml"),
    semanticContextProfilePath: path.join(
      ROOT,
      "app-server/tests/fixtures/semantic-context-host-live-profile.yaml",
    ),
    semanticContextStatePath: semanticStatePath,
    roleBindingsPath: path.join(stateRoot, "role-bindings.json"),
    switchboardAttachmentPath: path.join(stateRoot, "switchboard-attachment.json"),
    configuredProviderFeatures: ["token_budget"],
    workerDispatchTimeoutMs: 15 * 60_000,
  });
  t.after(() => bootstrap.close({ abandonActiveWork: true }));
  const completions = new Map();
  const waiters = new Map();
  bootstrap.transport.onNotification((notification) => {
    if (notification.method !== "turn/completed") return;
    const turnId = notification.params?.turn?.id;
    const waiter = waiters.get(turnId);
    if (waiter) {
      waiters.delete(turnId);
      waiter(notification.params.turn);
    } else if (typeof turnId === "string") completions.set(turnId, notification.params.turn);
  });
  const waitForCompletion = (turnId) => {
    if (completions.has(turnId)) {
      const completion = completions.get(turnId);
      completions.delete(turnId);
      return Promise.resolve(completion);
    }
    return new Promise((resolve) => waiters.set(turnId, resolve));
  };
  await bootstrap.transport.request("initialize", {
    clientInfo: { name: "work-engine-live-hosted-transition", version: "0.1.0" },
    capabilities: { experimentalApi: false, requestAttestation: false },
  });
  bootstrap.transport.notify("initialized");
  const shell = await bootstrap.transport.request("thread/start", { cwd: ROOT });
  const instanceId = `live-hosted-transition-${randomUUID()}`;
  const attached = await bootstrap.transport.request("turn/start", {
    threadId: shell.thread.id,
    clientUserMessageId: `live-hosted-transition-attach-${randomUUID()}`,
    input: [{
      type: "text",
      text: `:we attach strategic-planner:${instanceId}`,
      text_elements: [],
    }],
  });
  assert.equal((await waitForCompletion(attached.turn.id)).status, "completed");
  const started = await bootstrap.transport.request("turn/start", {
    threadId: shell.thread.id,
    clientUserMessageId: `live-hosted-transition-turn-${randomUUID()}`,
    input: [{
      type: "text",
      text: "Acknowledge readiness for one executable-host context transition.",
      text_elements: [],
    }],
  });
  const completion = await waitForCompletion(started.turn.id);
  assert.equal(completion.status, "completed", JSON.stringify(completion.error));
  const store = await openSqliteAppServerStateStore({ filePath: semanticStatePath });
  t.after(() => store.close());
  const lifecycle = store.snapshot(`strategic-planner:${instanceId}`);
  assert.ok(lifecycle?.publication?.checkpointRevision);
  assert.equal(lifecycle.fence.publicationRevision, lifecycle.publication.checkpointRevision);
  assert.equal(lifecycle.fence.ledgerRevision, lifecycle.ledgerEntry.entryRevision);
  assert.deepEqual(bootstrap.manager.snapshot().admissions, []);
});
