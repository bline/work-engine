import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import {
  AgentInstructionReviewError,
  ImplementationReviewerRuntime,
  OpenRouterCodexReviewerAdapter,
  ReviewerProfileRegistry,
  createAgentInstructionReviewService,
  createImplementationReviewService,
  createReviewEpisodeService,
  implementationReviewDigest,
  loadRuntimeManifest,
  reviewerRuntimeDigest,
} from "../src/index.mjs";

const root = path.resolve(new URL("../..", import.meta.url).pathname);
const fixtureRoot = path.join(root, "app-server/tests/fixtures/agent-instruction-review");
const evidence = {
  path: "app-server/runtime-manifest.yaml", startLine: 80, endLine: 114,
  sha256: "b".repeat(64),
};

const loadFixture = async (name) => JSON.parse(await readFile(path.join(fixtureRoot, `${name}.json`), "utf8"));
async function declaredReferences() {
  const references = [
    ["repo-search.backend-capabilities", "skills/repo-search/references/backend-capabilities.md"],
    ["agent-instruction-review.finding-contract", "skills/agent-instruction-review/references/finding-contract.md"],
  ];
  return Promise.all(references.map(async ([id, relativePath]) => ({
    id, kind: "conditional_reference", path: path.join(root, relativePath),
    content: await readFile(path.join(root, relativePath), "utf8"),
    precedence: "skill", inclusion: "loaded",
    condition: {
      reference: relativePath, decision: "included", authority: "canonical skill instruction",
      sha256: reviewerRuntimeDigest({ id, relativePath, decision: "included" }),
    },
  })));
}
const reference = (owner, referenceValue, revision, sha256) => ({
  owner, reference: referenceValue, revision, sha256, freshness: "exact immutable revision",
});

function profile() {
  const value = {
    schemaVersion: 1,
    profileId: "fixture.agent-instruction-review-v1",
    enabled: true,
    requestedModel: "openai/gpt-5.2-codex",
    provider: "openrouter",
    reasoning: "high",
    capabilities: ["structured_output", "repository_read"],
    outputSchema: "work-engine.agent-instruction-review.v1",
    effectiveInstructions: "Return the exact specialist result contract supplied by the canonical role.",
    isolatedHome: true,
    limitations: ["Fixture admission does not approve a production profile."],
    acceptingAuthority: "work-engine.s11.accepted-plan-v1",
  };
  value.configurationDigest = reviewerRuntimeDigest(value);
  return value;
}

const catalog = {
  schemaVersion: 1,
  catalogId: "fixture",
  observedAt: "2026-08-30T00:00:00Z",
  expiresAt: "2026-09-01T00:00:00Z",
  source: "fixture",
  sourceSha256: "a".repeat(64),
  models: [{
    slug: "openai/gpt-5.2-codex", provider: "openrouter",
    capabilities: ["structured_output", "repository_read"], routingConstraints: [],
  }],
};
const policy = {
  classification: "confidential", access: "episode actors",
  retention: "bounded projection retained", exactRetentionAuthorized: false,
  redaction: "raw bodies omitted", tamperEvidence: "sha256 digest",
};

function genericFinding(status, remediation = false) {
  return {
    id: "AIR-001", severity: "high", title: "Causal explanation is not loaded",
    evidence: [evidence],
    observed: "The command is loaded while its reason and failure mode are absent from the effective closure.",
    violatedExpectation: "A structural command must expose its causal parent at the governed loading boundary.",
    consequence: "The governed agent receives an opaque command and cannot preserve its protected distinction.",
    basis: "reproduced", confidence: "high",
    recommendedRemediation: "Project the reason and concrete failure mode into the effective closure.",
    status,
    remediationEvidence: remediation ? [evidence] : [],
  };
}

function detail(status, closure) {
  const fragment = closure.fragments.find(({ id }) => id === "manifest.developer-instructions");
  return {
    id: "AIR-001", severity: "high",
    instruction: {
      fragmentId: fragment.id, path: fragment.path,
      startLine: fragment.startLine, endLine: fragment.endLine,
    },
    placement: {
      semanticOwner: "runtime-manifest", consumer: "governed-agent", audience: "role instance",
      scope: "exact runtime role", precedence: "developer", loadingReach: "command only",
    },
    protectedDistinction: "Opaque commands cannot support causal generalization.",
    causalExposure: {
      reasonLoaded: status === "verified_resolved",
      failureModeLoaded: status === "verified_resolved",
      sourceTraceable: true, reviewerReconstructed: true,
    },
    exactRouteNecessary: false,
    authoritySource: "DESIGN.md runtime instruction projection contract",
    consequence: status === "verified_resolved" ? "The causal parent is now loaded." : "Logical continuity is missing.",
    confidence: "high", limitations: [], advisoryOutcome: "restate",
  };
}

function specialistResult({ subject, closure, fixture, remediated = false }) {
  const hasFinding = fixture.finding_status !== null;
  const genericLimitations = fixture.generic_verdict === "incomplete" ? fixture.limitations : [];
  const generic = {
    schemaVersion: 1,
    subject,
    verdict: fixture.generic_verdict,
    findings: hasFinding ? [genericFinding(fixture.finding_status, remediated)] : [],
    decisiveEvidence: fixture.generic_verdict === "incomplete" ? [] : [evidence],
    limitations: genericLimitations,
  };
  return {
    schemaVersion: 1,
    perspective: "agent-instruction-review",
    subject,
    closureRevision: closure.closureRevision,
    applicability: fixture.applicability,
    applicabilityReason: fixture.applicability === "omitted"
      ? "The subject contains no present-day materially normative instruction consequence."
      : "The subject contains a loaded structural command.",
    result: generic,
    findingDetails: hasFinding ? [detail(fixture.finding_status, closure)] : [],
    limitations: fixture.limitations,
  };
}

test("specialist closure binds effective manifest inputs, precedence, omissions, and limitations", async () => {
  const service = createAgentInstructionReviewService();
  const manifest = await loadRuntimeManifest(path.join(root, "app-server/runtime-manifest.yaml"));
  const roleProjection = manifest.projectRole("implementation-reviewer", "closure-proof");
  const requiredReferences = await declaredReferences();
  const subject = { commit: "candidate-1", tree: "tree-1", patchIdentity: "patch-1" };
  const reason = "Reason: the consumer cannot preserve an invariant it never receives.\nFailure: an opaque command creates logical discontinuity.\n";
  const condition = {
    reference: "finding-contract", decision: "included", authority: "runtime-manifest",
    sha256: reviewerRuntimeDigest({ reference: "finding-contract", decision: "included" }),
  };
  const closure = await service.projectClosure({
    subject, roleProjection, sourceInventoryComplete: true,
    conditionalReferences: [...requiredReferences, {
      id: "conditional.causal-parent", kind: "conditional_reference",
      path: "fixture/causal-parent.md", content: reason, precedence: "skill",
      inclusion: "loaded", condition,
    }],
  });
  assert.equal(closure.sourceInventoryComplete, true);
  assert.equal(closure.fragments.some(({ id, content }) => id === "conditional.causal-parent" && content === reason), true);
  assert.deepEqual(closure.fragments.map(({ precedence }) => precedence).slice(0, 3), ["developer", "skill", "skill"]);
  assert.match(closure.closureRevision, /^[a-f0-9]{64}$/);

  const omitted = await service.projectClosure({
    subject, roleProjection, sourceInventoryComplete: true,
    conditionalReferences: [...requiredReferences, {
      id: "conditional.closed-omission", kind: "conditional_reference",
      path: "fixture/not-applicable.md", content: "Not loaded.\n", precedence: "subject",
      inclusion: "omitted",
      condition: {
        reference: "closed-condition", decision: "accepted_safe_omission", authority: "accepted contract",
        sha256: reviewerRuntimeDigest({ decision: "accepted_safe_omission" }),
      },
    }],
  });
  assert.deepEqual(omitted.omissions, ["conditional.closed-omission"]);
  assert.equal(omitted.fragments.at(-1).content, null);

  await assert.rejects(service.projectClosure({
    subject, roleProjection, sourceInventoryComplete: true,
    conditionalReferences: [...requiredReferences, {
      id: "conditional.unsafe", kind: "conditional_reference", path: "fixture/unsafe.md",
      content: "Hidden.\n", precedence: "subject", inclusion: "omitted",
      condition: {
        reference: "open-condition", decision: "declared_without_closure", authority: "caller",
        sha256: reviewerRuntimeDigest({ decision: "declared_without_closure" }),
      },
    }],
  }), AgentInstructionReviewError);
  await assert.rejects(service.projectClosure({
    subject, roleProjection, sourceInventoryComplete: true,
  }), /complete instruction inventory omits declared reference/);
  await assert.rejects(service.projectClosure({
    subject, roleProjection, sourceInventoryComplete: false, limitations: [],
  }), /incomplete instruction inventory requires limitations/);
  const limited = await service.projectClosure({
    subject, roleProjection, sourceInventoryComplete: false,
    limitations: ["Provider-hidden system instructions are unavailable."],
  });
  assert.deepEqual(limited.limitations, ["Provider-hidden system instructions are unavailable."]);
});

test("applicability, no-finding, omission, and limitation remain distinct from authority", async () => {
  const service = createAgentInstructionReviewService();
  const manifest = await loadRuntimeManifest(path.join(root, "app-server/runtime-manifest.yaml"));
  const roleProjection = manifest.projectRole("implementation-reviewer", "result-proof");
  const subject = { commit: "candidate", tree: "tree", patchIdentity: "patch" };
  const closure = await service.projectClosure({
    subject, roleProjection, sourceInventoryComplete: true,
    conditionalReferences: await declaredReferences(),
  });
  for (const name of ["applicable-finding", "applicable-no-finding", "omitted", "limitation"]) {
    const fixture = await loadFixture(name);
    const admission = service.admit({ result: specialistResult({ subject, closure, fixture }), closure });
    assert.equal(admission.result.applicability, fixture.applicability);
    assert.equal(admission.authority.mutationAuthorized, false);
    assert.equal(admission.authority.reviewerSelectionAuthorized, false);
    assert.equal(admission.authority.implementationAcceptanceAuthorized, false);
    assert.equal(admission.authority.selfCertificationAuthorized, false);
  }
  const findingFixture = await loadFixture("applicable-finding");
  const drifted = specialistResult({ subject, closure, fixture: findingFixture });
  drifted.findingDetails[0].instruction.fragmentId = "missing-fragment";
  assert.throws(() => service.admit({ result: drifted, closure }), /outside the exact closure/);
});

test("fresh specialist entry and retained remediation preserve exact subjects and generic episode lineage", async () => {
  const [initialFixture, remediationFixture, freshFixture, retainedFixture] = await Promise.all([
    loadFixture("applicable-finding"), loadFixture("remediation"),
    loadFixture("fresh-entry"), loadFixture("retained-remediation"),
  ]);
  const manifest = await loadRuntimeManifest(path.join(root, "app-server/runtime-manifest.yaml"));
  const service = createAgentInstructionReviewService();
  const governed = manifest.projectRole("implementation-reviewer", "governed-subject");
  const requiredReferences = await declaredReferences();
  const subject1 = { commit: "candidate-1", tree: "tree-1", patchIdentity: "patch-1" };
  const closure1 = await service.projectClosure({
    subject: subject1, roleProjection: governed, sourceInventoryComplete: true,
    conditionalReferences: requiredReferences,
  });
  const subject2 = { commit: "candidate-2", tree: "tree-2", patchIdentity: "patch-2" };
  const closure2 = await service.projectClosure({
    subject: subject2, roleProjection: governed, sourceInventoryComplete: true,
    conditionalReferences: [...requiredReferences, {
      id: "conditional.remediated-causal-parent", kind: "causal_reason",
      path: "fixture/remediation.md",
      content: "Reason and failure mode are now loaded for the governed agent.\n",
      precedence: "developer", inclusion: "loaded",
      condition: {
        reference: "remediation-delta", decision: "included", authority: "builder delta",
        sha256: reviewerRuntimeDigest({ candidate: 2 }),
      },
    }],
  });
  let nextResult = specialistResult({ subject: subject1, closure: closure1, fixture: initialFixture });
  const homes = [];
  const delivered = [];
  const registry = new ReviewerProfileRegistry({ profiles: [profile()] });
  const adapter = new OpenRouterCodexReviewerAdapter({
    registry,
    now: () => Date.parse("2026-08-30T12:00:00Z"),
    executeProcess: async ({ env, input }) => {
      homes.push(env.CODEX_HOME);
      const request = JSON.parse(input);
      delivered.push(request.instructions);
      return {
        exitCode: 0, stderr: "",
        stdout: `${JSON.stringify({
          type: "review.completed",
          observed: { model: "openai/gpt-5.2-codex", provider: "openrouter", servingVariant: "fixture" },
          result: nextResult,
        })}\n`,
      };
    },
  });
  const reviewer = new ImplementationReviewerRuntime({ manifest, adapter, agentInstructionReview: service });
  const initial = await reviewer.reviewAgentInstructions({
    instanceId: "specialist-episode", profileId: profile().profileId,
    subject: subject1, closure: closure1, catalogProjection: catalog, rawEventPolicy: policy,
  });
  assert.equal(initial.isolation.freshEntry, freshFixture.fresh_entry);
  assert.equal(initial.specialistReview.result.applicability, "applicable");
  assert.match(delivered[0], /WORK_ENGINE_AGENT_INSTRUCTION_REVIEW_V1/);
  assert.match(delivered[0], new RegExp(closure1.closureRevision));
  assert.equal(initial.specialistReview.authority.independenceClaimed, false);

  const implementationReview = createImplementationReviewService();
  const episode = createReviewEpisodeService({ implementationReview });
  const identity = {
    runId: "run", sliceNumber: 3, attemptId: "attempt", planVersion: "plan",
    reviewObligationId: "agent-instruction-review", reviewEpisodeId: "episode",
  };
  const authority = {
    schemaVersion: 1, grantId: "grant", identity,
    source: reference("supervisor", "selection", "v1", "c".repeat(64)),
    writer: {
      actorId: "reviewer", provider: "openrouter", generation: freshFixture.writer_generation,
      runtimeSession: reference("runtime", "session-1", "v1", "d".repeat(64)),
    },
    readers: ["reviewer", "builder", "supervisor"],
    initialSubject: reference("checkpoint", "candidate-1", "v1", implementationReviewDigest(subject1)),
    predecessorRevision: null,
  };
  let state = episode.begin({ authority, transitionId: "begin" });
  state = episode.transition({
    authority, expectedRevision: state.revision, transitionId: "initial", action: "record_result",
    payload: { result: initial.specialistReview.implementationReviewResult, unresolvedQuestions: [] },
  });
  assert.equal(state.phase, "remediation");
  state = episode.transition({
    authority, expectedRevision: state.revision, transitionId: "subject-2", action: "record_remediation_subject",
    payload: { subject: reference("checkpoint", "candidate-2", "v2", implementationReviewDigest(subject2)) },
  });

  nextResult = specialistResult({
    subject: subject2, closure: closure2, fixture: remediationFixture, remediated: true,
  });
  const continued = await reviewer.reviewAgentInstructions({
    instanceId: "specialist-episode", profileId: profile().profileId,
    subject: subject2, closure: closure2, catalogProjection: catalog, rawEventPolicy: policy,
    continuationSessionId: "session-1",
  });
  assert.equal(continued.isolation.freshEntry, retainedFixture.fresh_entry);
  assert.equal(continued.isolation.continuation, retainedFixture.continuation);
  assert.equal(homes[0], homes[1]);
  state = episode.transition({
    authority, expectedRevision: state.revision, transitionId: "reevaluate", action: "record_result",
    payload: { result: continued.specialistReview.implementationReviewResult, unresolvedQuestions: [] },
  });
  assert.equal(state.continuity, "same_session");
  assert.equal(state.phase, "reported");
  assert.equal(state.currentResult.findings[0].status, "verified_resolved");
  assert.equal(await reviewer.retire("specialist-episode"), true);
  await assert.rejects(access(homes[0]));
});
