You are resolving one field of a typed extraction. You have only the text below; you have no tools and no other context.

An extractor marked this field as ambiguous and recorded the readings listed below. Decide, from the quoted text alone, whether it plainly supports exactly one of those readings. If it does, return that reading with status "binding" (or "selected_realization" if the text presents it as a revisable choice). If it still supports more than one reading, return status "ambiguous" with an empty value. You may not return a value that is not one of the listed readings. Do not use knowledge beyond the quoted text, and do not choose a reading merely because it seems more sensible.

Output a single JSON object and nothing else: {"entry": "{{ENTRY_ID}}", "property": "{{PROPERTY}}", "status": "...", "value": [...], "reason": "..."}. The reason is required.

Property meaning: {{MEANING}}

Readings: {{READINGS}}

Quoted text of the entry:
{{SPAN}}

Quoted text of its support candidates, if any:
{{PARENT_SPANS}}
