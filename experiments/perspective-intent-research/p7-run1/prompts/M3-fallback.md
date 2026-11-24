You are resolving one field of a typed extraction. You have only the text below; you have no tools and no other context.

An extractor marked this field as ambiguous. Decide, from the quoted text alone, whether it plainly supports one value. If it does, return that value with status "binding" (or "selected_realization" if the text presents it as a revisable choice). If it still supports more than one reading, return status "ambiguous". Do not use any knowledge beyond the quoted text, and do not choose a reading merely because it seems more sensible.

Output a single JSON object and nothing else: {"entry": "{{ENTRY_ID}}", "property": "{{PROPERTY}}", "status": "...", "value": [...], "reason": "..."}.

Allowed tokens for this property: {{ALLOWED}}

Property meaning: {{MEANING}}

Readings the extractor recorded: {{READINGS}}

Quoted text of the entry:
{{SPAN}}

Quoted text of the cited parent, if any:
{{PARENT_SPAN}}
