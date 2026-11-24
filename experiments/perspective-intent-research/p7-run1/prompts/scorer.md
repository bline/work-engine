You are scoring one output from a document-review experiment against a fixed rubric. You have no tools. You are given the source documents the output analysed, the rubric, and the output's material for each phase to score. Judge only whether the output establishes each rubric property, using the source documents to check its claims. Do not reward style or length.

Rules:
- Score each phase listed below separately, using only that phase's material. Phases: {{PHASES}}.
- Answer every rubric item ("items" and "must_not") in every phase, once.
- "expect" and "positive" items: answer "hit" if the material establishes the property, "miss" if it addresses the target text but does not establish it, "not_assessed" if it never addresses the target text, or "uncertain".
- "observe" items: answer "yes", "no" or "uncertain"; there is no right answer.
- "must_not" items: answer "violated", "not_violated" or "uncertain".
- Typed material (a JSON extraction plus a comparator report): an item counts as a hit only through a comparator flag or record that concerns the text and the property the item describes. A field typed in the extraction without a matching flag or record is not a hit. A "deficient_provenance" flag concerns citation only: it is not evidence that a requirement is unsupported, changed or strengthened.
- Evidence: for every answer other than "not_assessed", quote the exact span of that phase's material that decides it. Quotes are checked mechanically against that phase's material. When an answer of "miss", "not_violated", "no" or "uncertain" rests on the absence of something, write exactly NO SUPPORTING SPAN.
- "flag_census": list every flag the phase's material raises (for typed material, every entry of "derived_flags" and "plan_flags"), each with a quoted span of the material and a class: "target:<item id>" (an expect or positive item), "legitimate_other" (a real issue not in the rubric; quote the source-document text that supports it in "source_support"), "false_flag" (the source documents do not support it), or "context_only" (a flag on a document the rubric marks context-only).
- "weakening": whether the material proposes dropping or weakening any requirement the source documents plainly state; quote the span if "yes".

Output a single JSON object and nothing else:
{"input": "{{INPUT_ID}}", "phases": [{"phase": "<a listed phase>", "items": [{"id": "...", "answer": "...", "evidence": "..."}], "flag_census": [{"flag": "<quoted span>", "class": "...", "source_support": "<quote or empty>"}], "weakening": {"answer": "yes" | "no", "evidence": "..."}}]}

RUBRIC:
{{RUBRIC}}

SOURCE DOCUMENTS:
{{DOCUMENTS}}

OUTPUT TO SCORE:
{{OUTPUT}}
