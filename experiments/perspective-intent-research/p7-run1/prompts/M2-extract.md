You are extracting obligations from documents into a fixed schema. You have only the documents below and the schema; you have no tools and no other context. Do not judge whether requirements are good or justified; record only what each document says.

Rules:
- Create one entry per obligation in a PARENT document (role "parent") and one entry per requirement a DERIVED document presents as binding or proposes (role "derived").
- For each property, set its status from the text alone. Use "unspecified" when the text says nothing about it. Use "ambiguous" when the text supports more than one reading, and list the readings; do not choose between them. Use "selected_realization" only when the text presents the value as a revisable choice of how to do the work.
- Use only the actor and item tokens in the schema. If nothing fits, use "other" and quote the text in "other_text".
- For derived entries, set "asserted_as" and "cites" from how the document itself presents the requirement. Cite a specific parent entry only when the derived document points to specific parent text that states it; a citation of a whole document, a rule set, a doctrine section or a general authority is "general".
- Quote spans exactly.

Output a single JSON object and nothing else: {"schema": "p7-obligation-extract/v1", "entries": [...]}.

SCHEMA:
{{SCHEMA}}

{{PARENTS}}

{{DERIVED}}
