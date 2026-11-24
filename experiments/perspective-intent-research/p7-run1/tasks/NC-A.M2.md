You are extracting obligations from documents into a fixed schema. You have only the documents below and the schema; you have no tools and no other context. Do not judge whether requirements are good or justified; record only what each document says.

Rules:
- Create one entry per obligation in a PARENT document (role "parent") and one entry per requirement a DERIVED document presents as binding or proposes (role "derived").
- For each property, set its status from the text alone. Use "unspecified" when the text says nothing about it. Use "ambiguous" when the text supports more than one reading, and list the readings; do not choose between them. Use "selected_realization" only when the text presents the value as a revisable choice of how to do the work.
- Use only the actor and item tokens in the schema. If nothing fits, use "other" and quote the text in "other_text".
- For derived entries, set "asserted_as" and "cites" from how the document itself presents the requirement. Cite a specific parent entry only when the derived document points to specific parent text that states it; a citation of a whole document, a rule set, a doctrine section or a general authority is "general".
- Quote spans exactly.

Output a single JSON object and nothing else: {"schema": "p7-obligation-extract/v1", "entries": [...]}.

SCHEMA:
{
  "schema": "p7-obligation-extract/v1",
  "description": "Typed extraction of obligations and asserted requirements for P7 run 1. The extractor fills these fields from the given documents only. The comparator (comparator.py) reads them; the extractor never sees the comparator's rules.",
  "statuses": {
    "binding": "The text plainly requires this value.",
    "selected_realization": "The text describes this value as a chosen way of doing the work, revisable by its author, not as a requirement.",
    "unspecified": "The text says nothing about this property.",
    "ambiguous": "The text supports more than one reading. Do not resolve it; record the readings in 'readings'."
  },
  "properties": {
    "initiator": {
      "meaning": "Who must start or issue the action.",
      "value_kind": "permitted_actors",
      "comparison": "A larger set permits more actors and is weaker."
    },
    "outcome_owner": {
      "meaning": "Who or what must bring about the outcome.",
      "value_kind": "permitted_actors",
      "comparison": "A larger set permits more actors and is weaker."
    },
    "verifier": {
      "meaning": "Who must establish that the outcome holds.",
      "value_kind": "permitted_actors",
      "comparison": "A larger set permits more actors and is weaker."
    },
    "path_fact": {
      "meaning": "Required facts about how or by which route the outcome is produced.",
      "value_kind": "required_items",
      "comparison": "A larger set requires more and is stronger."
    },
    "acceptance_evidence": {
      "meaning": "Evidence the acceptance condition requires.",
      "value_kind": "required_items",
      "comparison": "A larger set requires more and is stronger."
    }
  },
  "actors": [
    "operator",
    "campaign_owner",
    "research_lifecycle",
    "dev_vertical",
    "server",
    "host",
    "collector",
    "container_engine",
    "fixture",
    "independent_observer",
    "termination_actor",
    "component",
    "any"
  ],
  "actor_notes": {
    "any": "No restriction on who. Use only when the text plainly permits anyone.",
    "component": "Generic: the single component the obligation is about, when no other actor token fits (used by fixtures)."
  },
  "path_items": [
    "no_name_fallback",
    "exact_id_only",
    "same_engine",
    "no_force_or_fallback_stop",
    "fresh_admission_before_first_write",
    "pending_entry_fenced",
    "other"
  ],
  "evidence_items": [
    "outcome_observed",
    "independent_absence",
    "removal_failure_reported_as_failure",
    "actor_attributed_removal",
    "exclusive_causality",
    "self_report",
    "independent_termination_verification",
    "action_acknowledgement",
    "other"
  ],
  "item_notes": {
    "other": "Something not in the list. Put the quoted span in 'other_text'. The comparator cannot compare 'other' and reports it as not established."
  },
  "entry": {
    "id": "string, unique in the output, e.g. 'F-043#1'",
    "document": "input document id, e.g. 'F-043'",
    "role": "'parent' (a source of requirements) or 'derived' (a later document asserting requirements)",
    "span": "exact quoted text the entry is extracted from",
    "fields": {
      "<property>": {
        "status": "one of statuses",
        "value": "list of actor tokens or item tokens; empty when status is unspecified",
        "readings": "for ambiguous only: list of alternative value lists",
        "other_text": "for 'other' items only"
      }
    },
    "asserted_as": "derived entries only: 'inherited' (presented as already required by a parent), 'proposed' (offered as a new requirement for someone to decide), 'new' (imposed as binding without citing a parent), 'realization_choice' (the author's own revisable choice), 'unchanged' (presented as unchanged from a parent)",
    "cites": "derived entries only: {'kind': 'specific', 'parent_entry': '<id>'} or {'kind': 'general', 'text': '<quoted citation>'} or {'kind': 'none'}",
    "replaces_parent": "derived entries only: true if the entry presents itself as the complete requirement replacing the cited parent's"
  }
}


=== PARENT DOCUMENT NC-A ===
# Scratch-file hygiene contract (synthetic fixture NC-A)

Status: constructed negative control for P7 run 1. Not a real Work Engine contract.

## Scope

The report renderer writes one temporary file per render, under its own private scratch directory, and nothing else writes there.

## Terminal cleanup

The renderer owns its temporary file for its whole lifetime. Before it returns, the renderer must delete the temporary file it created and record in its own result whether the deletion succeeded. A render whose deletion failed reports `cleanup_failed`; it is not reported as a clean render. No other component removes or inspects the file, and no independent check is required: the file is private to the renderer and has no consumer.
=== END NC-A ===

DERIVED DOCUMENTS: none
