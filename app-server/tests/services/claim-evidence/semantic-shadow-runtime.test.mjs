import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { ClaimEvidenceSemanticShadowRuntime } from "../../../src/services/claim-evidence/semantic-shadow-runtime.mjs";
import { openSqliteClaimEvidenceStore } from "../../../src/services/claim-evidence/sqlite-store.mjs";
import { SEMANTIC_SHADOW_CHECKS, normalizeSemanticSourceProjection } from "../../../src/services/claim-evidence/semantic-shadow-contract.mjs";
import { digest } from "../../../src/services/claim-evidence/identity.mjs";
import { normalizeObservation, observationEventIdentity } from "../../../src/services/claim-evidence/observation.mjs";

const authority = { schema_version: 1, grant_id: "grant:test", actor: "test", profile: "proposal-research-v1", permissions: [], decision_scope: "proposal-formation", authority_reference: { owner: "repository", reference: "authority", revision: "blob", integrity_sha256: "a".repeat(64), freshness: "current", status: "verified" } };
const exactReference = (name) => ({ owner: "repository", reference: name, revision: "blob-1", integrity_sha256: "a".repeat(64), freshness: "current", status: "verified" });
const admittedObservation = () => normalizeObservation({ event_identity: observationEventIdentity("shadow-test", "source-one"), producer: { identity: "shadow-test", kind: "deterministic_test_adapter" }, origin: { kind: "test_receipt", reference: "receipt:shadow", trust_classification: "untrusted_input" }, subject: { namespace: "research", subject_kind: "proposal", stable_subject_id: "placement", content_set: ["proposal.md"] }, evidence_baseline: exactReference("baseline"), artifact: { kind: "test_receipt", reference: "receipt:shadow", digest: { algorithm: "sha256", value: "b".repeat(64) }, verification: "verified", checkpoint: { object_format: "sha256", commit: "receipt-shadow", tree: "suite-shadow" } }, observed_at: "2026-08-27T11:59:00.000Z", provider_sequence: null, completeness: "available", exclusions: [], collection_failures: [], executable_generation: "test", adapter_version: "test-v1" });
const projection = (observation = admittedObservation()) => { const content = "Evidence says the boundary is host-owned."; return normalizeSemanticSourceProjection({ projection_id: "projection:one", host_binding: { identity: "authenticated-host", revision: "binding-1" }, source_watermark: "git:tree-1", observations: [{ observation_id: observation.id, observation_digest: digest(observation) }], subject: { namespace: "research", subject_kind: "proposal", stable_subject_id: "placement", content_set: ["proposal.md"] }, evidence_baseline: exactReference("baseline"), profile: "proposal-research-v1", judgment_scope: "proposal-formation", materials: [{ reference: "material:one", observation_id: observation.id, origin: "proposal.md", trust_classification: "untrusted_input", content_digest: digest(content), content }], authority_references: [exactReference("authority")], predecessors: ["revision:prior"], competitors: ["revision:competing"], completeness: "available", omissions: [], collection_failures: [] }); };
const provenance = (id) => ({ producer: "recorded", model: "fixture", version: "1", inferenceId: id });
const candidateBody = () => ({ outcome: "candidate", candidate: { stable_claim_identity: "claim:placement", proposition: "The boundary is host-owned", support_qualification: "supported", assumptions: [], limitations: ["One source"], references: ["material:one"], profile_payload: { materiality: "material", support_qualification: "supported" }, judgment_scope: "proposal-formation", predecessors: ["revision:prior"], competitors: ["revision:competing"], unresolved_blockers: [] } });
const verificationBody = () => ({ outcome: "verification", verification: { checks: SEMANTIC_SHADOW_CHECKS.map((name) => ({ name, status: "pass", reason: `${name} checked`, references: ["material:one"] })), findings: [], blockers: [], uncertainty: [] } });
class RecordedInference {
  constructor(result) { this.result = result; this.requests = []; }
  async infer(request) { this.requests.push(structuredClone({ ...request, signal: undefined })); if (this.result instanceof Error) throw this.result; return structuredClone(this.result); }
}
const result = (body, id, usage) => ({ outputText: JSON.stringify(body), provenance: provenance(id), ...(usage ? { usage } : {}) });
async function database(t) { const directory = await mkdtemp(path.join(tmpdir(), "claim-shadow-runtime-")); t.after(() => rm(directory, { recursive: true, force: true })); return path.join(directory, "claims.sqlite3"); }
function clocks() { let tick = 0; return { now: () => new Date(Date.UTC(2026, 7, 27, 12, 0, tick++)).toISOString(), monotonicNow: () => tick++ * 5 }; }

test("vertical compiler to distinct verifier persists across restart and replays without canonical effects", async (t) => {
  const filePath = await database(t); let store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] });
  const observation = admittedObservation(); store.recordObservation(observation);
  const before = store.exportStore(); const observationsBefore = store.listObservations();
  const compiler = new RecordedInference(result(candidateBody(), "compiler:vertical", { inputTokens: 10, cachedInputTokens: 2, outputTokens: 5 }));
  const verifier = new RecordedInference(result(verificationBody(), "verifier:vertical"));
  const runtime = new ClaimEvidenceSemanticShadowRuntime({ compiler, verifier, store, ...clocks() });
  const first = await runtime.inspect({ episodeIdentity: "episode:vertical", projection: projection(observation) });
  assert.equal(first.idempotent, false); assert.equal(first.episode.eligibility_disposition, "accepted");
  assert.equal(first.episode.compiler_attempt.usage.input_tokens, 10); assert.equal(first.episode.verifier_attempt.usage.input_tokens, null);
  assert.equal(compiler.requests.length, 1); assert.equal(verifier.requests.length, 1);
  assert.equal(verifier.requests[0].input.candidate.candidate_revision, first.episode.candidate.candidate_revision);
  assert.deepEqual(store.exportStore(), before); assert.deepEqual(store.listObservations(), observationsBefore); store.close();

  store = await openSqliteClaimEvidenceStore({ filePath }); t.after(() => store.close());
  const replayCompiler = new RecordedInference(new Error("must not run")); const replayVerifier = new RecordedInference(new Error("must not run"));
  const replayRuntime = new ClaimEvidenceSemanticShadowRuntime({ compiler: replayCompiler, verifier: replayVerifier, store });
  const replay = await replayRuntime.inspect({ episodeIdentity: "episode:vertical", projection: projection(observation) });
  assert.equal(replay.idempotent, true); assert.deepEqual(replay.episode, first.episode); assert.equal(replayCompiler.requests.length, 0); assert.equal(replayVerifier.requests.length, 0);
  assert.deepEqual(store.exportStore(), before); assert.deepEqual(store.listObservations(), observationsBefore);
  const changedProjection = structuredClone(projection(observation)); changedProjection.source_watermark = "git:different-tree"; delete changedProjection.projection_revision;
  const rebound = normalizeSemanticSourceProjection(changedProjection);
  await assert.rejects(replayRuntime.inspect({ episodeIdentity: "episode:vertical", projection: rebound }), /episode identity conflict/);
});

test("compiler and verifier failure/refusal stages close durable truthful episodes", async (t) => {
  const cases = [
    ["compiler-refusal", result({ outcome: "refusal", refusal: { category: "insufficient", reason: "not enough evidence", references: ["material:one"] } }, "c:refuse"), result(verificationBody(), "v:unused"), "compiler_refusal", false],
    ["compiler-parse", result({ invented: true }, "c:parse"), result(verificationBody(), "v:unused"), "compiler_parse_failure", false],
    ["compiler-failure", new Error("compiler unavailable"), result(verificationBody(), "v:unused"), "compiler_inference_failure", false],
    ["verifier-refusal", result(candidateBody(), "c:ok1"), result({ outcome: "refusal", refusal: { category: "uncertain", reason: "cannot verify", references: ["material:one"] } }, "v:refuse"), "verifier_refusal", true],
    ["verifier-parse", result(candidateBody(), "c:ok2"), result({ invented: true }, "v:parse"), "verifier_parse_failure", true],
    ["verifier-failure", result(candidateBody(), "c:ok3"), new Error("verifier unavailable"), "verifier_inference_failure", true],
    ["same-inference", result(candidateBody(), "same:id"), result(verificationBody(), "same:id"), "inference_identity_reuse", true],
  ];
  const filePath = await database(t); const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] }); t.after(() => store.close()); const observation = admittedObservation(); store.recordObservation(observation);
  for (const [name, compilerResult, verifierResult, failureKind, hasCandidate] of cases) {
    const compiler = new RecordedInference(compilerResult); const verifier = new RecordedInference(verifierResult);
    const output = await new ClaimEvidenceSemanticShadowRuntime({ compiler, verifier, store, ...clocks() }).inspect({ episodeIdentity: `episode:${name}`, projection: projection(observation) });
    assert.equal(output.episode.failure.kind, failureKind, name); assert.equal(output.episode.candidate !== null, hasCandidate, name); assert.equal(output.episode.eligibility_disposition, null, name);
    assert.deepEqual(store.readShadowEpisode(`episode:${name}`), output.episode);
    if (!hasCandidate) assert.equal(verifier.requests.length, 0, name);
  }
});

test("separate capability objects are mandatory and disagreement is retained as unresolved", async (t) => {
  const filePath = await database(t); const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] }); t.after(() => store.close()); const observation = admittedObservation(); store.recordObservation(observation);
  const shared = new RecordedInference(result(candidateBody(), "shared"));
  assert.throws(() => new ClaimEvidenceSemanticShadowRuntime({ compiler: shared, verifier: shared, store }), /separate capabilities/);
  const disputed = verificationBody(); disputed.verification.checks = SEMANTIC_SHADOW_CHECKS.map((name, index) => ({ name, status: index === 0 ? "disagree" : "pass", reason: "bounded finding", references: ["material:one"] })); disputed.verification.findings = [{ status: "disagree", reason: "identity continuity disputed", references: ["material:one"] }];
  const episode = (await new ClaimEvidenceSemanticShadowRuntime({ compiler: new RecordedInference(result(candidateBody(), "c:dispute")), verifier: new RecordedInference(result(disputed, "v:dispute")), store, ...clocks() }).inspect({ episodeIdentity: "episode:dispute", projection: projection(observation) })).episode;
  assert.equal(episode.eligibility_disposition, "unresolved"); assert.equal(episode.verification.findings[0].status, "disagree");
});

test("unadmitted observation projection is rejected before either inference request", async (t) => {
  const filePath = await database(t); const store = await openSqliteClaimEvidenceStore({ filePath, bootstrapAuthorities: [authority] }); t.after(() => store.close());
  const compiler = new RecordedInference(result(candidateBody(), "compiler:must-not-run"));
  const verifier = new RecordedInference(result(verificationBody(), "verifier:must-not-run"));
  const runtime = new ClaimEvidenceSemanticShadowRuntime({ compiler, verifier, store });
  await assert.rejects(runtime.inspect({ episodeIdentity: "episode:unadmitted", projection: projection() }), /unadmitted observation binding/);
  assert.equal(compiler.requests.length, 0); assert.equal(verifier.requests.length, 0);
  assert.deepEqual(store.listShadowEpisodes(), []);
});
