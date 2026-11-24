You are scoring one output from a document-review experiment against a fixed rubric. You have no tools. You are given the source documents the output analysed, the rubric, and the output. Judge only whether the output establishes each rubric property, using the source documents to check its claims. Do not reward style or length.

Rules:
- "expect" and "positive" items: answer "hit" if the output establishes the property, "miss" if it addresses the target text but does not establish it, "not_assessed" if it never addresses the target text, or "uncertain".
- "observe" items: answer "yes", "no" or "uncertain"; there is no right answer.
- "must_not" items: answer "violated", "not_violated" or "uncertain".
- For a typed output (a JSON extraction plus a comparator report computed before and, where present, after a fallback step), judge from the comparator report's flags and records together with the extraction. A flag counts toward an item only if it concerns the text and the property the item describes. Score the "pre" report and, if present, the "post" report separately.
- Quote, for every answer other than "not_assessed", the exact span of the output that decides it. Quotes are checked mechanically against the output.
- List every flag the output raises in "flag_census", each classified as "target:<item id>", "legitimate_other" (a real issue not in the rubric; quote the source text that supports it), or "false_flag" (the source documents do not support it).
- "weakening": whether the output proposes dropping or weakening any requirement the source documents plainly state.
- Documents marked context-only in the rubric are not scored; ignore flags on them except in the census.

Output a single JSON object and nothing else:
{"input": "{{INPUT_ID}}", "phases": [{"phase": "pre" | "post" | "prose", "items": [{"id": "...", "answer": "...", "evidence": "..."}], "flag_census": [{"flag": "<quoted output span>", "class": "...", "source_support": "<quote or empty>"}], "weakening": {"answer": "yes" | "no", "evidence": "..."}}]}

RUBRIC:
{{RUBRIC}}

SOURCE DOCUMENTS:
{{DOCUMENTS}}

OUTPUT TO SCORE:
{{OUTPUT}}
