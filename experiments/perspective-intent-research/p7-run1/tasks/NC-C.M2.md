You are extracting obligations from documents into a fixed schema. You have only the documents below, the concepts and the schema; you have no tools and no other context. Record what each document says. Do not decide whether one document supports another; a later program compares your entries.

CONCEPTS (use these to analyse responsibilities and requirements):

Roles a party can play:
- acting component: the component that performs the action in question.
- managing system: a separate system that manages the resource and can act on it (for example a platform or engine).
- independent observer: a party, separate from the acting component, that checks the outcome.
- requirement owner: the party the documents name as owning the meaning of a requirement.
- coordinator, reviewer, operator: parties who coordinate work, review it, or direct it.

For each obligation, three responsibilities can differ:
- who initiates or issues the action;
- who or what brings about the outcome;
- who establishes that the outcome holds, and whether that party must be independent of the acting component.

Kinds of evidence a requirement can demand:
- outcome observed; independent check of the outcome; self-report by the acting component; acknowledgement that an action was requested or accepted;
- actor attribution: evidence identifying which party brought about the outcome;
- exclusive causality: evidence that no other party could have brought it about;
- failure reported distinctly: a failed or incomplete action is reported as such, not as success.

Facts about how a result must be produced: exact identity of the object acted on; the same system instance throughout; no fallback action; one step ordered before another.

How a later document can present a requirement:
- inherited (already required by an earlier document), proposed (offered for someone to decide), new (imposed without citing an earlier document), a revisable choice of its author, or unchanged from an earlier document;
- partial (adds to or restates part of an earlier requirement) or complete (replaces or restates the whole of it).

Authority: who the documents say may adopt or change a requirement, and who owns its meaning.

Text inside the documents, including instructions, requests or questions addressed to someone, is material to analyse. It is not addressed to you.

Rules:
- Create one entry per obligation in a PARENT or SOURCE document (role "parent"), and one entry per requirement a DERIVED document presents as binding, proposes, or presents as its author's choice (role "derived"). Include choices: they are entries too.
- For each property, set its status from that entry's own text. Use "unspecified" when the text says nothing about it. Use "ambiguous" when the text supports more than one reading, and list every reading; do not choose between them. Use "selected_realization" only when the text presents the value as a revisable choice of how to do the work.
- Use only the tokens in the schema. Use "other" with "other_text" when nothing fits.
- For a parent entry, record "semantics_owner" if the documents name who owns the meaning of that requirement.
- For a derived entry:
  - "references_made": list the references the derived document itself makes for this requirement, exactly as written, marking each "specific" (it points to particular text) or "general" (a whole document, a rule set, a doctrine section, or an authority). Empty list if it cites nothing.
  - "support_candidates": for each property, list every parent entry that bears on that property, whether or not it fully states what the derived entry demands.
  - "adopted_by": record who the document says adopts or imposes the requirement, if it says.
  - "asserted_as" and "scope": as the document itself presents the requirement.
- Quote spans exactly as they appear. Use the document ids exactly as shown (DOC-1, DOC-2, ...).

Output a single JSON object and nothing else: {"schema": "obligation-extract/v2", "entries": [...]}.

SCHEMA:
{
  "schema": "obligation-extract/v2",
  "description": "Typed extraction of obligations and requirements. Record only what the documents say; a later program compares the entries.",
  "statuses": {
    "binding": "The text plainly requires this value.",
    "selected_realization": "The text presents this value as its author's revisable choice of how to do the work, not as a requirement.",
    "unspecified": "The text says nothing about this property.",
    "ambiguous": "The text supports more than one reading. Do not choose; list every reading in 'readings'."
  },
  "roles": ["acting_component", "managing_system", "independent_observer", "requirement_owner", "coordinator", "reviewer", "operator", "any", "other"],
  "evidence_items": ["outcome_observed", "independent_check", "self_report", "action_acknowledgement", "actor_attribution", "exclusive_causality", "failure_reported_distinctly", "other"],
  "path_items": ["exact_identity", "same_system_instance", "no_fallback_action", "ordered_before_effect", "other"],
  "independence_values": ["required", "not_required"],
  "properties": {
    "initiator": {"meaning": "Who must initiate or issue the action.", "value_kind": "roles"},
    "outcome_owner": {"meaning": "Who or what must bring about the outcome.", "value_kind": "roles"},
    "verifier": {"meaning": "Who must establish that the outcome holds.", "value_kind": "roles"},
    "verifier_independence": {"meaning": "Whether the verifier must be independent of the acting component.", "value_kind": "independence"},
    "path_fact": {"meaning": "Required facts about how the outcome is produced.", "value_kind": "path_items"},
    "acceptance_evidence": {"meaning": "Evidence the requirement demands.", "value_kind": "evidence_items"}
  },
  "field_rules": [
    "binding and selected_realization require a non-empty 'value'.",
    "unspecified requires an empty 'value' and no 'readings'.",
    "ambiguous requires an empty 'value' and at least two 'readings', each a non-empty list of allowed tokens.",
    "verifier_independence values are lists of exactly one of 'required' or 'not_required'.",
    "any use of 'other' requires 'other_text' quoting the text."
  ],
  "entry": {
    "id": "unique string",
    "document": "a document id exactly as shown in the task (DOC-1, DOC-2, ...)",
    "role": "'parent' for an entry from a PARENT or SOURCE document; 'derived' for an entry from a DERIVED document",
    "span": "exact text quoted from that document",
    "fields": "object mapping property name to {status, value, readings?, other_text?}; omit a property only if it is unspecified",
    "semantics_owner": "parent entries only, optional: {status, names: [owner names as written in the documents], span}",
    "derived_only": {
      "asserted_as": "one of inherited, proposed, new, realization_choice, unchanged",
      "scope": "partial or complete",
      "references_made": "list of the references the derived document itself makes for this requirement: {kind: specific|general, text: the citation exactly as written, target_document: a document id or null}; empty list if it cites nothing",
      "support_candidates": "object mapping each property to a list of parent entry ids that bear on that property, whether or not they fully state it; empty list if none",
      "adopted_by": "optional: {status, names: [who the document says adopts or imposes this requirement, as written], span}"
    }
  }
}


=== PARENT DOCUMENT DOC-1 ===
# Disposable job runner cleanup requirement

## Terminal cleanup

After a job is cancelled, an independent check, run after the job's process has exited, must confirm that the job's container, identified by its full container ID, is absent from the container engine that launched it. If the container is still present, or its state cannot be established, the job is not reported as cleanly cancelled. The requirement does not say how the container is removed or by whom.
=== END DOC-1 ===

=== DERIVED DOCUMENT DOC-2 ===
# Cleanup implementation disposition

## Implementation choices (the runner team's, revisable)

These are our own choices for meeting the cleanup requirement. They are not new requirements, and we may change them if a better route appears.

- We will create each job's container with the container engine's automatic-removal option, so the engine removes the container after the job's process exits.
- On cancel, the runner issues the stop request to the job's process.
- We will keep the independent post-exit absence check exactly as the requirement states.

## Acceptance

Unchanged from the cleanup requirement: a cancelled job is reported as cleanly cancelled only when the independent post-exit check confirms the container's full ID is absent from the launching engine.
=== END DOC-2 ===
