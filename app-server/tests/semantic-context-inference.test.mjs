import assert from "node:assert/strict";
import { createHash, generateKeyPairSync } from "node:crypto";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { parse, stringify } from "yaml";

import {
  SemanticContextInferenceRuntime,
  projectObservedContext,
  verifySemanticContextVerification,
} from "../src/index.mjs";

const FIXTURES = new URL("./fixtures/semantic-context/", import.meta.url);
const digest = (value) => createHash("sha256").update(value, "utf8").digest("hex");

const CONTENT = Object.freeze({
  human: "The user authorized the next bounded App Server slice.",
  evidence: "The observed-context projection and continuation schema tests pass.",
  skill: "The strategic planner is advisory and preserves canonical authority.",
  next: "Build the hidden semantic compiler and independent verifier against fixtures.",
});

const SHA = Object.freeze(Object.fromEntries(
  Object.entries(CONTENT).map(([key, value]) => [key, digest(value)]),
));

async function fixture(name, replacements = {}) {
  let value = await readFile(new URL(name, FIXTURES), "utf8");
  for (const [token, replacement] of Object.entries({
    HUMAN_SHA: SHA.human,
    EVIDENCE_SHA: SHA.evidence,
    SKILL_SHA: SHA.skill,
    NEXT_SHA: SHA.next,
    ...replacements,
  })) value = value.replaceAll(`__${token}__`, replacement);
  return value;
}

async function changedCompilerOutput(change) {
  const output = parse(await fixture("compiler-valid.yaml"));
  change(output);
  return stringify(output);
}

function observedFixture() {
  const keys = generateKeyPairSync("ed25519");
  const projection = projectObservedContext({
    logicalRoleInstanceId: "strategic-planner:main",
    runtimeBinding: {
      threadId: "thread-semantic-1",
      bindingRevision: 4,
      environmentRevision: "runtime-environment-1",
    },
    lastCompletedTurnId: "turn-7",
    visibleItems: [{
      identity: "user-message:proceed",
      origin: "human",
      trustClass: "human_authority_input",
      instructionApplicability: "contract_defined",
      contentRef: {
        kind: "thread-item",
        reference: "user-message:proceed",
        sha256: SHA.human,
      },
    }, {
      identity: "evidence:observed-context",
      origin: "tool",
      trustClass: "attributed_evidence",
      instructionApplicability: "none",
      producer: "app-server-test-gate",
      contentRef: {
        kind: "evidence",
        reference: "evidence:observed-context",
        sha256: SHA.evidence,
      },
    }],
    governingSources: [{
      identity: "role-contract",
      owner: "skills/strategic-planner/SKILL.md",
      contentRef: {
        kind: "skill",
        reference: "skills/strategic-planner/SKILL.md",
        sha256: SHA.skill,
      },
    }],
    activatedSkills: [{
      name: "strategic-planner",
      path: "skills/strategic-planner/SKILL.md",
      sha256: SHA.skill,
    }],
    lifecycleSnapshot: {
      schemaVersion: 1,
      threadId: "thread-semantic-1",
      retainedObservationCount: 2,
    },
    expectedNextWork: {
      reference: "expected-next-work:semantic-inference",
      sha256: SHA.next,
    },
    sourceInventoryCompleteness: "partial",
    omissions: [{
      scope: "provider-effective-prompt",
      reason: "provider does not expose exact effective input",
    }],
  }, {
    componentId: "work-engine.observed-context-projector",
    buildRevision: "fixture-build-1",
    keyId: "fixture-key-1",
    privateKey: keys.privateKey,
  });
  return { keys, projection };
}

function materials() {
  return [{
    contentRef: { kind: "thread-item", reference: "user-message:proceed", sha256: SHA.human },
    content: CONTENT.human,
  }, {
    contentRef: { kind: "evidence", reference: "evidence:observed-context", sha256: SHA.evidence },
    content: CONTENT.evidence,
  }, {
    contentRef: { kind: "skill", reference: "skills/strategic-planner/SKILL.md", sha256: SHA.skill },
    content: CONTENT.skill,
  }, {
    contentRef: { kind: "expected_next_work", reference: "expected-next-work:semantic-inference", sha256: SHA.next },
    content: CONTENT.next,
  }];
}

class RecordedCompiler {
  constructor(outputText, { inferenceId = "compiler-inference-1", usage = undefined } = {}) {
    this.outputText = outputText;
    this.inferenceId = inferenceId;
    this.usage = usage;
    this.requests = [];
  }

  async infer(request) {
    this.requests.push(request);
    return {
      outputText: this.outputText,
      provenance: {
        producer: "recorded-semantic-compiler",
        model: "fixture-model",
        version: "1",
        inferenceId: this.inferenceId,
      },
      ...(this.usage === undefined ? {} : { usage: this.usage }),
    };
  }
}

class RecordedVerifier {
  constructor(outputFactory, { inferenceId = "verifier-inference-1", usage = undefined } = {}) {
    this.outputFactory = outputFactory;
    this.inferenceId = inferenceId;
    this.usage = usage;
    this.requests = [];
  }

  async infer(request) {
    this.requests.push(request);
    return {
      outputText: await this.outputFactory(request),
      provenance: {
        producer: "recorded-semantic-verifier",
        model: "fixture-model",
        version: "1",
        inferenceId: this.inferenceId,
      },
      ...(this.usage === undefined ? {} : { usage: this.usage }),
    };
  }
}

function runtime({ compiler, verifier, keys, timestamps, monotonicNow } = {}) {
  return new SemanticContextInferenceRuntime({
    compiler,
    verifier,
    resolvePublicKey: (keyId) => keyId === "fixture-key-1" ? keys.publicKey : null,
    now: () => timestamps.shift(),
    ...(monotonicNow ? { monotonicNow } : {}),
  });
}

async function acceptedVerifierOutput(request) {
  return fixture("verifier-accepted.yaml", {
    SOURCE_SHA: request.input.sourceRevision.slice("sha256:".length),
    CANDIDATE_SHA: request.input.candidate.candidateRevision.slice("sha256:".length),
  });
}

test("recorded compiler and distinct verifier produce one bound inspection result", async () => {
  const { keys, projection } = observedFixture();
  const compiler = new RecordedCompiler(await fixture("compiler-valid.yaml"));
  const verifier = new RecordedVerifier(acceptedVerifierOutput);
  const inspection = await runtime({
    compiler,
    verifier,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() });

  assert.equal(inspection.sourceRevision, projection.sourceRevision);
  assert.equal(inspection.candidate.subject.logicalRoleInstanceId, "strategic-planner:main");
  assert.equal(inspection.candidate.subject.runtimeBindingRevision, 4);
  assert.equal(inspection.candidate.compiler.inferenceId, "compiler-inference-1");
  assert.equal(inspection.verification.verifier.inferenceId, "verifier-inference-1");
  assert.equal(inspection.verification.disposition, "accepted");
  assert.equal(inspection.verification.interactionEvaluations.length, 1);
  assert.equal(inspection.verification.interactionEvaluations[0].closure, "supported");
  assert.equal(Object.hasOwn(inspection, "checkpointRevision"), false);
  assert.equal(Object.hasOwn(inspection.verification, "retirementReady"), false);
  assert.equal(Object.isFrozen(inspection.verification.checks), true);
  assert.equal(verifySemanticContextVerification(inspection.verification, {
    candidate: inspection.candidate,
    projection,
    resolvePublicKey: () => keys.publicKey,
  }), true);
  assert.match(compiler.requests[0].instructions, /does not gain authority/);
  assert.match(verifier.requests[0].instructions, /Do not rewrite the candidate/);
  assert.equal(compiler.requests[0].input.materials[0].contentRef != null, true);
  assert.equal(verifier.requests[0].input.candidate.candidateRevision, inspection.candidate.candidateRevision);
  assert.equal(JSON.stringify(inspection).includes(CONTENT.skill), false);
  assert.equal(inspection.measurements.compiler.inputTokens, null);
  assert.ok(inspection.measurements.compiler.durationMs >= 0);
});

test("inference telemetry is normalized and host-timed without affecting semantic output", async () => {
  const { keys, projection } = observedFixture();
  const compiler = new RecordedCompiler(await fixture("compiler-valid.yaml"), {
    usage: { inputTokens: 800, cachedInputTokens: 200, outputTokens: 90, costMicrounits: 12 },
  });
  const verifier = new RecordedVerifier(acceptedVerifierOutput, {
    usage: { inputTokens: 1_000, cachedInputTokens: 0, outputTokens: 70, costMicrounits: 15 },
  });
  const times = [0, 7, 10, 19];
  const inspection = await runtime({
    compiler,
    verifier,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
    monotonicNow: () => times.shift(),
  }).inspect({ projection, sourceMaterials: materials() });
  assert.deepEqual(inspection.measurements, {
    compiler: {
      durationMs: 7,
      inputTokens: 800,
      cachedInputTokens: 200,
      outputTokens: 90,
      costMicrounits: 12,
    },
    verifier: {
      durationMs: 9,
      inputTokens: 1_000,
      cachedInputTokens: 0,
      outputTokens: 70,
      costMicrounits: 15,
    },
  });
  assert.equal(inspection.verification.disposition, "accepted");
});

test("bounded material loading rejects omissions, additions, digest mismatch, and projection tampering", async () => {
  const { keys, projection } = observedFixture();
  const compiler = new RecordedCompiler(await fixture("compiler-valid.yaml"));
  const verifier = new RecordedVerifier(acceptedVerifierOutput);
  const createRuntime = () => runtime({
    compiler,
    verifier,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  });

  await assert.rejects(
    createRuntime().inspect({ projection, sourceMaterials: materials().slice(1) }),
    /omit 1 projected references/,
  );
  await assert.rejects(
    createRuntime().inspect({
      projection,
      sourceMaterials: [...materials(), {
        contentRef: { kind: "other", reference: "unprojected", sha256: digest("other") },
        content: "other",
      }],
    }),
    /not authorized by the observed projection/,
  );
  const changed = materials();
  changed[0].content = "different bytes";
  await assert.rejects(
    createRuntime().inspect({ projection, sourceMaterials: changed }),
    /does not match its SHA-256 reference/,
  );
  const tampered = structuredClone(projection);
  tampered.observedContext.visibleItems[0].trustClass = "governing_instruction";
  await assert.rejects(
    createRuntime().inspect({ projection: tampered, sourceMaterials: materials() }),
    /requires a verified observed-context projection/,
  );
  assert.equal(compiler.requests.length, 0);
});

test("compiler cannot emit host-owned fields or references outside supplied material", async () => {
  const { keys, projection } = observedFixture();
  const verifier = new RecordedVerifier(acceptedVerifierOutput);
  const hostOwned = new RecordedCompiler(`subject: {}\n${await fixture("compiler-valid.yaml")}`);
  await assert.rejects(runtime({
    compiler: hostOwned,
    verifier,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /unsupported fields: subject/);

  const invented = new RecordedCompiler(
    (await fixture("compiler-valid.yaml")).replace(SHA.human, "f".repeat(64)),
  );
  await assert.rejects(runtime({
    compiler: invented,
    verifier,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /not bound to supplied semantic material/);

  const promotedData = new RecordedCompiler(
    (await fixture("compiler-valid.yaml")).replace(
      /instructionsToReload:\n    - reference: skills\/strategic-planner\/SKILL\.md\n      sha256: "[a-f0-9]+"/,
      `instructionsToReload:\n    - reference: evidence:observed-context\n      sha256: "${SHA.evidence}"`,
    ),
  );
  await assert.rejects(runtime({
    compiler: promotedData,
    verifier,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /instructionsToReload contain a non-governing source/);
  assert.equal(verifier.requests.length, 0);
});

test("human interaction status and loading remain separate fail-closed dimensions", async () => {
  const { keys, projection } = observedFixture();
  const verifier = new RecordedVerifier(acceptedVerifierOutput);
  const rejectsCompiler = async (change, pattern) => {
    const compiler = new RecordedCompiler(await changedCompilerOutput(change));
    await assert.rejects(runtime({
      compiler,
      verifier,
      keys,
      timestamps: ["2026-08-25T18:00:00.000Z"],
    }).inspect({ projection, sourceMaterials: materials() }), pattern);
  };

  await rejectsCompiler((output) => {
    output.humanInteractions[0].status = "ambiguous";
    output.humanInteractions[0].nextContextDisposition = "reference_only";
  }, /ambiguous meaning must remain exact or escalate/);
  await rejectsCompiler((output) => {
    output.humanInteractions[0].status = "open";
    output.humanInteractions[0].nextContextDisposition = "omit_from_working_context";
  }, /open meaning cannot be omitted/);
  await rejectsCompiler((output) => {
    output.humanInteractions[0].nextContextDisposition = "reference_only";
  }, /active governing meaning must be loaded or escalate/);
  await rejectsCompiler((output) => {
    output.humanInteractions[0].durableConsequenceRef = null;
  }, /compiled_consequence requires a durable consequence/);
  await rejectsCompiler((output) => {
    output.humanInteractions[0].sourceRef = {
      reference: "evidence:observed-context",
      sha256: SHA.evidence,
    };
  }, /not an observed human-authored source/);
  assert.equal(verifier.requests.length, 0);
});

test("verifier disposition is host-derived and uncertainty remains unresolved", async () => {
  const { keys, projection } = observedFixture();
  const compiler = new RecordedCompiler(await fixture("compiler-valid.yaml"));
  const verifier = new RecordedVerifier(async (request) => fixture("verifier-unresolved.yaml", {
    SOURCE_SHA: request.input.sourceRevision.slice("sha256:".length),
    CANDIDATE_SHA: request.input.candidate.candidateRevision.slice("sha256:".length),
  }));
  const inspection = await runtime({
    compiler,
    verifier,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() });

  assert.equal(inspection.verification.disposition, "unresolved");
  assert.equal(inspection.verification.uncertainty.length, 1);

  const declaring = new RecordedVerifier(async (request) =>
    `${await acceptedVerifierOutput(request)}\ndisposition: accepted\n`
  );
  await assert.rejects(runtime({
    compiler: new RecordedCompiler(await fixture("compiler-valid.yaml")),
    verifier: declaring,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /unsupported fields: disposition/);
});

test("escalated loading remains unresolved even when the verifier supports the classification", async () => {
  const { keys, projection } = observedFixture();
  const compiler = new RecordedCompiler(await changedCompilerOutput((output) => {
    output.humanInteractions[0].nextContextDisposition = "escalate";
  }));
  const verifier = new RecordedVerifier(async (request) => {
    const output = parse(await acceptedVerifierOutput(request));
    output.interactionEvaluations[0].nextContextDisposition = "escalate";
    output.checks.find((check) => check.name === "interaction_closure").status = "uncertain";
    return stringify(output);
  });
  const inspection = await runtime({
    compiler,
    verifier,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() });

  assert.equal(inspection.verification.disposition, "unresolved");
  assert.equal(inspection.verification.blockers.length, 0);
  assert.equal(inspection.verification.uncertainty.length, 0);
});

test("verifier must evaluate every interaction and agree with its aggregate closure check", async () => {
  const { keys, projection } = observedFixture();
  const compilerOutput = await fixture("compiler-valid.yaml");
  const missing = new RecordedVerifier(async (request) => {
    const output = parse(await acceptedVerifierOutput(request));
    output.interactionEvaluations = [];
    return stringify(output);
  });
  await assert.rejects(runtime({
    compiler: new RecordedCompiler(compilerOutput),
    verifier: missing,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /evaluate every candidate human interaction exactly once/);

  const disagreement = new RecordedVerifier(async (request) => {
    const output = parse(await acceptedVerifierOutput(request));
    output.interactionEvaluations[0].closure = "uncertain";
    return stringify(output);
  });
  await assert.rejects(runtime({
    compiler: new RecordedCompiler(compilerOutput),
    verifier: disagreement,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /interaction_closure check contradicts/);

  const rebound = new RecordedVerifier(async (request) => {
    const output = parse(await acceptedVerifierOutput(request));
    output.interactionEvaluations[0].status = "resolved";
    return stringify(output);
  });
  await assert.rejects(runtime({
    compiler: new RecordedCompiler(compilerOutput),
    verifier: rebound,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /status does not match the candidate interaction/);
});

test("verifier must be a separate invocation and return every required check", async () => {
  const { keys, projection } = observedFixture();
  const sameIdentity = new RecordedVerifier(acceptedVerifierOutput, {
    inferenceId: "compiler-inference-1",
  });
  await assert.rejects(runtime({
    compiler: new RecordedCompiler(await fixture("compiler-valid.yaml")),
    verifier: sameIdentity,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /distinct inference invocation identity/);

  const incomplete = new RecordedVerifier(async (request) =>
    (await acceptedVerifierOutput(request)).replace(
      /  - name: sufficiency[\s\S]*?(?=  - name: attribution)/,
      "",
    )
  );
  await assert.rejects(runtime({
    compiler: new RecordedCompiler(await fixture("compiler-valid.yaml")),
    verifier: incomplete,
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() }), /every required check exactly once/);
});

test("verification integrity rejects changed findings, disposition, and candidate binding", async () => {
  const { keys, projection } = observedFixture();
  const inspection = await runtime({
    compiler: new RecordedCompiler(await fixture("compiler-valid.yaml")),
    verifier: new RecordedVerifier(acceptedVerifierOutput),
    keys,
    timestamps: ["2026-08-25T18:00:00.000Z", "2026-08-25T18:00:01.000Z"],
  }).inspect({ projection, sourceMaterials: materials() });
  const changed = structuredClone(inspection.verification);
  changed.checks[0].rationale = "Changed after verification";
  assert.equal(verifySemanticContextVerification(changed, {
    candidate: inspection.candidate,
    projection,
    resolvePublicKey: () => keys.publicKey,
  }), false);
  const falseDisposition = structuredClone(inspection.verification);
  falseDisposition.disposition = "rejected";
  assert.equal(verifySemanticContextVerification(falseDisposition, {
    candidate: inspection.candidate,
    projection,
    resolvePublicKey: () => keys.publicKey,
  }), false);
  const changedInteraction = structuredClone(inspection.verification);
  changedInteraction.interactionEvaluations[0].loading = "insufficient";
  assert.equal(verifySemanticContextVerification(changedInteraction, {
    candidate: inspection.candidate,
    projection,
    resolvePublicKey: () => keys.publicKey,
  }), false);
  const otherCandidate = structuredClone(inspection.candidate);
  otherCandidate.candidateRevision = `sha256:${"0".repeat(64)}`;
  assert.equal(verifySemanticContextVerification(inspection.verification, {
    candidate: otherCandidate,
    projection,
    resolvePublicKey: () => keys.publicKey,
  }), false);
});
