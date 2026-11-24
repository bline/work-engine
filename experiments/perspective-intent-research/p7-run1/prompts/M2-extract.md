You are extracting obligations from documents into a fixed schema. You have only the documents below, the concepts and the schema; you have no tools and no other context. Record what each document says. Do not decide whether one document supports another; a later program compares your entries.

{{CONCEPTS}}

Rules:
- Create one entry per obligation in a PARENT or SOURCE document (role "parent"), and one entry per requirement a DERIVED document presents as binding, proposes, or presents as its author's choice (role "derived"). Include choices: they are entries too.
- For each property, set its status from that entry's own text. Use "unspecified" when the text says nothing about it. Use "ambiguous" when the text supports more than one reading, and list every reading; do not choose between them. Use "selected_realization" only when the text presents the value as a revisable choice of how to do the work. Use the role "any" only when the text deliberately leaves the party open.
- Use only the tokens in the schema. Use "other" with "other_text" when nothing fits.
- For a parent entry, record "semantics_owner" if the documents name who owns the meaning of that requirement, listing every name the documents use for that party.
- For a derived entry:
  - "references_made": list the references the derived document itself makes for this requirement, quoted from it, marking each "specific" (it points to particular text) or "general" (a whole document, a rule set, a doctrine section, or an authority), and listing the properties each is offered for. Empty list if it cites nothing.
  - "support_candidates": for each property, list every parent entry that bears on that property, whether or not it fully states what the derived entry demands.
  - "candidate_alignment": mark a property "same_condition" only when its several candidates state requirements on the same thing under the same condition.
  - "baseline": for a complete or unchanged entry, the parent entries it replaces or restates.
  - "adopted_by": record who the document says decided, approved or adopted the requirement, if it says. A party that only states, reviews or recommends it is not an adopter.
  - "asserted_as" and "scope": as the document itself presents the requirement.
- Quote by the schema's "quoting" rule. Use the document ids exactly as shown (DOC-1, DOC-2, ...).

Output a single JSON object and nothing else: {"schema": "obligation-extract/v3", "entries": [...]}.

SCHEMA:
{{SCHEMA}}

{{PARENTS}}

{{DERIVED}}
