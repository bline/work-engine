import {
  SLICE_CAMPAIGN_SCHEMA_VERSION, SLICE_PHASES, digest, freeze, identityKey,
  normalizeIdentity, requireRecord, requireSha256, requireText,
} from "./contract.mjs";

const NEXT_PHASE = new Map([["accepted", "implementing"], ["implementing", "gate_ready"], ["gate_ready", "review_ready"], ["review_ready", "terminal"]]);

export class InMemorySliceCampaignStore {
  constructor() { this.states = new Map(); this.workspaceAdmissions = new Map(); }
  get(key) { return this.states.get(key) ?? null; }
  admit(key, workspace, state) {
    if (this.states.has(key)) throw new Error("slice campaign attempt already exists");
    const holder = this.workspaceAdmissions.get(workspace);
    if (holder && holder !== key) throw new Error("workspace already has an admitted mutable slice");
    this.workspaceAdmissions.set(workspace, key); this.states.set(key, state);
  }
  put(key, state, expectedRevision, { releaseWorkspace = null } = {}) {
    if (this.states.get(key)?.revision !== expectedRevision) throw new Error("slice campaign revision conflict");
    this.states.set(key, state);
    if (releaseWorkspace !== null && this.workspaceAdmissions.get(releaseWorkspace) === key) this.workspaceAdmissions.delete(releaseWorkspace);
  }
}

export function createSliceCampaignService({
  store = new InMemorySliceCampaignStore(), reviewSubject, legacyReview,
  implementationReview = null, receiptFinalizer, completionOffer = null,
} = {}) {
  for (const [owner, methods] of [[reviewSubject, ["createCandidate", "createPhysicalProfile"]], [legacyReview, ["review"]], [receiptFinalizer, ["finalize"]]]) {
    if (!owner || methods.some((method) => typeof owner[method] !== "function")) throw new TypeError(`slice campaign requires composed owner ${methods.join("/")}`);
  }

  const publish = (state, expectedRevision, options = {}) => {
    const revision = digest({ ...state, revision: undefined });
    const published = freeze({ ...state, revision });
    store.put(identityKey(state.identity), published, expectedRevision, options);
    return published;
  };
  const current = (identity) => {
    const normalized = normalizeIdentity(identity);
    const state = store.get(identityKey(normalized));
    if (!state) throw new Error("slice campaign attempt does not exist");
    return state;
  };
  const requireRevision = (state, expectedRevision) => {
    requireSha256(expectedRevision, "expected campaign revision");
    if (state.revision !== expectedRevision) throw new Error("slice campaign revision conflict");
  };

  return Object.freeze({
    admit({ identity, workspace, acceptedBoundary, expectedImpact = null, baseline }) {
      const normalized = normalizeIdentity(identity);
      requireText(workspace, "workspace");
      requireRecord(acceptedBoundary, "accepted boundary");
      requireText(acceptedBoundary.reference, "accepted boundary reference");
      requireSha256(acceptedBoundary.sha256, "accepted boundary sha256");
      requireRecord(baseline, "campaign baseline");
      for (const field of ["acceptedCommit", "acceptedTree", "interSliceCommit"]) requireText(baseline[field], `campaign baseline ${field}`);
      if (expectedImpact !== null) { requireRecord(expectedImpact, "expected impact"); requireSha256(expectedImpact.sha256, "expected impact sha256"); }
      const key = identityKey(normalized);
      const initial = { schemaVersion: SLICE_CAMPAIGN_SCHEMA_VERSION, identity: normalized, workspace,
        acceptedBoundary: freeze(structuredClone(acceptedBoundary)), expectedImpact: expectedImpact && freeze(structuredClone(expectedImpact)),
        baseline: freeze(structuredClone(baseline)), phase: "accepted", latestConsequence: null,
        candidateRequestDigest: null, candidate: null, physicalProfile: null,
        review: null, implementationReview: null, terminal: null };
      const published = freeze({ ...initial, revision: digest(initial) });
      store.admit(key, workspace, published);
      return published;
    },
    recover(identity) { return current(identity); },
    advance({ identity, expectedRevision, phase, consequence }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (!SLICE_PHASES.includes(phase) || NEXT_PHASE.get(state.phase) !== phase) throw new Error("slice campaign phase transition is invalid");
      requireRecord(consequence, "phase consequence");
      return publish({ ...state, phase, latestConsequence: freeze(structuredClone(consequence)) }, state.revision);
    },
    async bindCandidate({ identity, expectedRevision, request }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "gate_ready") throw new Error("candidate requires gate-ready campaign state");
      const requestDigest = digest(request);
      if (state.candidateRequestDigest && state.candidateRequestDigest !== requestDigest) throw new Error("candidate request conflicts with bound candidate");
      const candidate = state.candidate ?? await reviewSubject.createCandidate(request);
      const candidateState = state.candidate ? state : publish({ ...state, candidateRequestDigest: requestDigest, candidate }, state.revision);
      const physicalProfile = await reviewSubject.createPhysicalProfile({ subject: {
        schema_version: 2,
        construction_method: "slice_checkpoint_candidate_receipt",
        evidence_cutoff: candidate.created_at,
        checkpoint: candidate,
      } });
      return publish({ ...candidateState, physicalProfile }, candidateState.revision);
    },
    async runLegacyReview({ identity, expectedRevision, selectionPlan }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "review_ready" || state.review || state.implementationReview) {
        throw new Error("legacy review requires review-ready state without a prior review");
      }
      if (!state.candidate || !state.physicalProfile) throw new Error("legacy review requires an immutable candidate and physical profile");
      const review = await legacyReview.review({ subject: state.candidate, profile: state.physicalProfile, selectionPlan });
      return publish({ ...state, review }, state.revision);
    },
    bindImplementationReview({ identity, expectedRevision, result }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "review_ready" || state.review || state.implementationReview) {
        throw new Error("implementation review requires review-ready state without a prior review");
      }
      if (!state.candidate || !state.physicalProfile) throw new Error("implementation review requires an immutable candidate and physical profile");
      if (!implementationReview || typeof implementationReview.admit !== "function") {
        throw new Error("native implementation-review service is unavailable");
      }
      const subject = {
        commit: state.candidate.checkpoint_commit_oid ?? state.candidate.commit,
        tree: state.candidate.checkpoint_tree_oid ?? state.candidate.tree,
        patchIdentity: state.candidate.task_patch_digest ?? state.candidate.manifestSha256,
      };
      const admitted = implementationReview.admit({ result, expectedSubject: subject });
      return publish({ ...state, implementationReview: admitted }, state.revision);
    },
    async terminalize({ identity, expectedRevision, outcome, receipt }) {
      const state = current(identity); requireRevision(state, expectedRevision);
      if (state.phase !== "review_ready" || (!state.review && !state.implementationReview)) {
        throw new Error("terminalization requires completed compatibility or implementation review");
      }
      requireText(outcome, "terminal outcome"); requireRecord(receipt, "terminal receipt");
      const finalizedReceipt = await receiptFinalizer.finalize({ identity: state.identity, outcome, receipt, candidate: state.candidate });
      const offer = completionOffer ? await completionOffer.offer({ identity: state.identity, outcome, candidate: state.candidate }) : null;
      return publish({ ...state, phase: "terminal", terminal: freeze({ outcome, finalizedReceipt, completionOffer: offer }) }, state.revision, { releaseWorkspace: state.workspace });
    },
  });
}
