You are scoring one output from a document-review experiment against a fixed rubric. You have no tools. Judge only whether the output establishes each rubric property; do not judge style, length or whether you agree with the output's reasoning beyond the property.

For each rubric item, answer "yes", "no" or "uncertain", and quote the span of the output that decides it. For typed outputs (a JSON extraction plus a comparator report), decide from the comparator report's flags and dispositions together with the extraction's statuses: a flag counts as a hit only if it concerns the text the item describes.

Items of kind "expect" are hits when the output establishes them. Items of kind "observe" record what the output says, with no right answer. Items under "must_not" are failures when the output does them.

Also report:
- "ambiguity_resolved": for typed outputs, whether the extraction typed 043's terminal-cleanup outcome owner or verifier as binding rather than ambiguous or unspecified ("yes", "no" or "n/a");
- "weakening": whether the output proposes dropping or weakening any requirement the source documents plainly state ("yes" or "no", with quote).

Output a single JSON object and nothing else:
{"input": "{{INPUT_ID}}", "items": [{"id": "...", "answer": "...", "evidence": "..."}], "ambiguity_resolved": "...", "weakening": {"answer": "...", "evidence": "..."}}

RUBRIC:
{{RUBRIC}}

OUTPUT TO SCORE:
{{OUTPUT}}
