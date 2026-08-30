import {
  digest, freeze, identityKey, sha256, text, validateAuthority, validateReference, validateState,
} from "./contract.mjs";

export class InMemoryReviewEpisodeStore {
  constructor() { this.episodes = new Map(); this.revisions = new Map(); }
  get(key) { return this.episodes.get(key) ?? null; }
  put(key, state, expectedRevision) {
    const current = this.episodes.get(key) ?? null;
    if ((current?.revision ?? null) !== expectedRevision) throw new Error("review episode revision conflict");
    this.episodes.set(key, state);
    const history = this.revisions.get(key) ?? [];
    history.push(state); this.revisions.set(key, history);
  }
  history(key) { return [...(this.revisions.get(key) ?? [])]; }
}

const semantic = (state) => Object.fromEntries(Object.entries(state).filter(([key]) => key !== "revision"));
const authorityBinding = (authority) => ({
  grantId: authority.grantId, source: authority.source,
  manifestRevision: digest(authority), readers: authority.readers,
});

function preserveFindingLineage(previous, next) {
  const current = new Map(previous.map((finding) => [finding.id, finding]));
  const updated = new Map(next.map((finding) => [finding.id, finding]));
  for (const [id, prior] of current) {
    const finding = updated.get(id);
    if (!finding) throw new Error("review episode findings cannot be deleted");
    for (const field of ["id", "severity", "title", "evidence", "observed", "violatedExpectation", "consequence", "basis", "confidence", "recommendedRemediation"]) {
      if (digest(finding[field]) !== digest(prior[field])) throw new Error("review episode finding attribution or evidence cannot be rewritten");
    }
  }
}

export function createReviewEpisodeService({ store = new InMemoryReviewEpisodeStore(), implementationReview } = {}) {
  if (!implementationReview || typeof implementationReview.admit !== "function") throw new TypeError("review episode service requires implementation-review admission");
  const recover = (identity) => store.get(identityKey(identity));
  const publish = (state, expectedRevision) => {
    validateState(state);
    const published = freeze({ ...state, revision: digest(state) });
    store.put(identityKey(state.identity), published, expectedRevision);
    return published;
  };
  const transition = ({ authority, expectedRevision, transitionId, action, payload }) => {
    validateAuthority(authority); text(transitionId, "review episode transitionId");
    const current = recover(authority.identity);
    const transitionRevision = digest({ action, payload });
    const previous = current?.handledTransitions?.[transitionId];
    if (previous) {
      if (previous === transitionRevision) return current;
      throw new Error("review episode transition identity conflicts with durable content");
    }
    if (!current || current.revision !== expectedRevision) throw new Error("review episode expected revision does not match current state");
    if (current.status === "retired") throw new Error("retired review episode cannot transition");
    let updated;
    if (action === "replace_writer") {
      if (authority.writer.generation !== current.writer.generation + 1
          || authority.predecessorRevision !== current.revision
          || authority.initialSubject.sha256 !== current.subject.sha256) throw new Error("review episode successor authority is invalid");
      text(payload.reason, "review episode replacement.reason"); text(payload.pendingAction, "review episode replacement.pendingAction");
      updated = { ...semantic(current), authority: authorityBinding(authority), writer: authority.writer,
        status: "active", uncertainty: null, pendingAction: payload.pendingAction,
        continuity: "reconstructed_continuation" };
    } else {
      if (digest(current.authority) !== digest(authorityBinding(authority)) || digest(current.writer) !== digest(authority.writer)) {
        throw new Error("review episode authority does not match current writer generation");
      }
      if (action === "record_result") {
        if (current.status !== "active" || !["initial_review", "re_evaluation"].includes(current.phase)) throw new Error("review episode result transition is invalid");
        const admitted = implementationReview.admit({ result: payload.result, expectedSubject: payload.result.subject });
        if (digest(admitted.result.subject) !== current.subject.sha256) throw new Error("review episode result does not match exact subject");
        if (current.currentResult) preserveFindingLineage(current.currentResult.findings, admitted.result.findings);
        if (!Array.isArray(payload.unresolvedQuestions)) throw new Error("review episode unresolvedQuestions must be an array");
        const pending = admitted.result.verdict !== "acceptable_as_is" || payload.unresolvedQuestions.length > 0;
        updated = { ...semantic(current), currentResult: admitted.result,
          unresolvedQuestions: payload.unresolvedQuestions, phase: pending ? "remediation" : "reported",
          pendingAction: pending ? "await_remediation" : "return_review_result_to_builder", continuity: "same_session" };
      } else if (action === "record_remediation_subject") {
        if (current.status !== "active" || !["remediation", "reported"].includes(current.phase)) throw new Error("review episode remediation subject transition is invalid");
        validateReference(payload.subject, "review episode remediation subject");
        updated = { ...semantic(current), subject: payload.subject, phase: "re_evaluation",
          pendingAction: "re_evaluate_delta_in_retained_session", continuity: "same_session" };
      } else if (action === "mark_uncertain") {
        text(payload.reason, "review episode uncertainty.reason"); text(payload.reconciliationAction, "review episode uncertainty.reconciliationAction");
        updated = { ...semantic(current), status: "uncertain", uncertainty: payload,
          pendingAction: payload.reconciliationAction };
      } else if (action === "retire") {
        updated = { ...semantic(current), status: "retired", uncertainty: null, retirement: payload,
          pendingAction: "none_review_episode_retired" };
      } else throw new Error("review episode action is unsupported");
    }
    updated.handledTransitions = { ...current.handledTransitions, [transitionId]: transitionRevision };
    return publish(updated, expectedRevision);
  };
  return Object.freeze({
    begin({ authority, transitionId, unresolvedQuestions = [] }) {
      validateAuthority(authority); text(transitionId, "review episode transitionId");
      if (authority.writer.generation !== 1) throw new Error("review episode initial writer generation is invalid");
      if (!Array.isArray(unresolvedQuestions)) throw new Error("review episode unresolvedQuestions must be an array");
      const existing = recover(authority.identity);
      const transitionRevision = digest({ action: "begin", unresolvedQuestions });
      if (existing) {
        if (existing.handledTransitions[transitionId] === transitionRevision) return existing;
        throw new Error("review episode already exists");
      }
      return publish({ schemaVersion: 1, identity: authority.identity, authority: authorityBinding(authority),
        writer: authority.writer, status: "active", phase: "initial_review", subject: authority.initialSubject,
        currentResult: null, unresolvedQuestions, pendingAction: "perform_initial_review",
        handledTransitions: { [transitionId]: transitionRevision }, continuity: "fresh_initial",
        uncertainty: null, retirement: null }, null);
    },
    transition,
    recover,
    read({ identity, revision = null }) {
      const values = store.history(identityKey(identity));
      if (revision === null) return values.at(-1) ?? null;
      sha256(revision, "review episode revision");
      return values.find((state) => state.revision === revision) ?? null;
    },
    history({ identity }) { return freeze(store.history(identityKey(identity))); },
  });
}
