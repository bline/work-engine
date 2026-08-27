import { digest } from "./identity.mjs";
import {
  CLAIM_COMPILER_INSTRUCTIONS, CLAIM_COMPILER_OUTPUT_CONTRACT, CLAIM_VERIFIER_INSTRUCTIONS,
  CLAIM_VERIFIER_OUTPUT_CONTRACT, SEMANTIC_SHADOW_INSTRUCTION_BUNDLE_REVISION,
  closeSemanticShadowEpisode, normalizeInferenceEvidence, parseCompilerOutput,
  parseVerifierOutput, validateSemanticSourceProjection,
} from "./semantic-shadow-contract.mjs";

function message(error) { return error instanceof Error && error.message ? error.message : "inference failed without an error message"; }

export class ClaimEvidenceSemanticShadowRuntime {
  constructor({ compiler, verifier, store, now = () => new Date().toISOString(), monotonicNow = () => performance.now() }) {
    if (!compiler || typeof compiler.infer !== "function") throw new TypeError("semantic shadow compiler capability is required");
    if (!verifier || typeof verifier.infer !== "function") throw new TypeError("semantic shadow verifier capability is required");
    if (compiler === verifier) throw new TypeError("semantic shadow compiler and verifier must be separate capabilities");
    if (!store || typeof store.readObservation !== "function" || typeof store.readShadowEpisode !== "function" || typeof store.recordShadowEpisode !== "function") throw new TypeError("semantic shadow episode store is required");
    this.compiler = compiler; this.verifier = verifier; this.store = store; this.now = now; this.monotonicNow = monotonicNow;
  }

  async inspect({ episodeIdentity, projection, signal } = {}) {
    if (typeof episodeIdentity !== "string" || episodeIdentity.length === 0) throw new TypeError("semantic shadow episode identity is required");
    validateSemanticSourceProjection(projection);
    for (const binding of projection.observations) {
      const observation = this.store.readObservation(binding.observation_id);
      if (!observation || digest(observation) !== binding.observation_digest) {
        throw new TypeError("semantic shadow source projection contains an unadmitted observation binding");
      }
    }
    const requestRevision = digest({ episode_identity: episodeIdentity, instruction_bundle_revision: SEMANTIC_SHADOW_INSTRUCTION_BUNDLE_REVISION, source_projection_revision: projection.projection_revision, observation_bindings: projection.observations, profile: projection.profile, judgment_scope: projection.judgment_scope });
    const prior = this.store.readShadowEpisode(episodeIdentity);
    if (prior) {
      if (prior.request_revision !== requestRevision) throw new TypeError("semantic shadow episode identity conflict");
      return { idempotent: true, episode: prior };
    }
    const startedAt = this.now(); let compilerAttempt = null; let candidate = null; let verifierAttempt = null;
    const close = (fields) => this.store.recordShadowEpisode(closeSemanticShadowEpisode({ episode_identity: episodeIdentity, request_revision: requestRevision, instruction_bundle_revision: SEMANTIC_SHADOW_INSTRUCTION_BUNDLE_REVISION, source_projection_revision: projection.projection_revision, observation_bindings: projection.observations, started_at: startedAt, closed_at: this.now(), compiler_attempt: compilerAttempt, candidate, verifier_attempt: verifierAttempt, verification: null, failure: null, eligibility_disposition: null, ...fields })).episode;
    let compilerResult;
    const compilerStarted = this.monotonicNow();
    try {
      compilerResult = await this.compiler.infer({ instructions: CLAIM_COMPILER_INSTRUCTIONS, input: projection, outputContract: CLAIM_COMPILER_OUTPUT_CONTRACT, signal });
      compilerAttempt = normalizeInferenceEvidence(compilerResult, Math.max(0, Math.round(this.monotonicNow() - compilerStarted)), "compiler inference");
    } catch (error) {
      return { idempotent: false, episode: close({ terminal_stage: "compiler", terminal_status: "failed", failure: { kind: "compiler_inference_failure", message: message(error) } }) };
    }
    let parsed;
    try { parsed = parseCompilerOutput(compilerResult.outputText, { projection, provenance: compilerResult.provenance, compiledAt: this.now() }); }
    catch (error) { return { idempotent: false, episode: close({ terminal_stage: "compiler", terminal_status: "failed", failure: { kind: "compiler_parse_failure", message: message(error) } }) }; }
    if (parsed.outcome === "refusal") return { idempotent: false, episode: close({ terminal_stage: "compiler", terminal_status: "refused", failure: { kind: "compiler_refusal", message: parsed.refusal.reason } }) };
    candidate = parsed;
    let verifierResult; const verifierStarted = this.monotonicNow();
    try {
      verifierResult = await this.verifier.infer({ instructions: CLAIM_VERIFIER_INSTRUCTIONS, input: { projection, candidate }, outputContract: CLAIM_VERIFIER_OUTPUT_CONTRACT, signal });
      verifierAttempt = normalizeInferenceEvidence(verifierResult, Math.max(0, Math.round(this.monotonicNow() - verifierStarted)), "verifier inference");
    } catch (error) { return { idempotent: false, episode: close({ terminal_stage: "verifier", terminal_status: "failed", failure: { kind: "verifier_inference_failure", message: message(error) } }) }; }
    if (verifierResult.provenance.inferenceId === compilerResult.provenance.inferenceId) return { idempotent: false, episode: close({ terminal_stage: "verifier", terminal_status: "distinctness_violation", failure: { kind: "inference_identity_reuse", message: "compiler and verifier returned the same inference identity" } }) };
    let verification;
    try { verification = parseVerifierOutput(verifierResult.outputText, { projection, candidate, provenance: verifierResult.provenance, verifiedAt: this.now() }); }
    catch (error) { return { idempotent: false, episode: close({ terminal_stage: "verifier", terminal_status: "failed", failure: { kind: "verifier_parse_failure", message: message(error) } }) }; }
    if (verification.outcome === "refusal") return { idempotent: false, episode: close({ terminal_stage: "verifier", terminal_status: "refused", failure: { kind: "verifier_refusal", message: verification.refusal.reason } }) };
    const episode = close({ terminal_stage: "complete", terminal_status: "completed", verification, eligibility_disposition: verification.eligibility_disposition });
    return { idempotent: false, episode };
  }
}
