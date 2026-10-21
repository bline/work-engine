# Claude working rules

[AGENTS.md](AGENTS.md) is the repository's doctrine for every agent and governs
design, authority and process. This file is narrower: mechanical rules for how
Claude states claims and corrects itself. It adds no authority and overrides
nothing in AGENTS.md.

## Claims

Each rule has a trigger and a required action. They apply to user-facing text and
to committed artifacts, not to tool output or intermediate reasoning.

1. **Before a quantitative or comparative claim, run the check that would show it
   wrong.** If that check has not been run, write `unverified` in the same
   sentence as the claim.
2. **Never enter an unknown as a value.** A disposition, score or status not read
   from the source is recorded as `not established`, which is distinct from a
   negative result.
3. **List and count a directory or file set before passing it to any tool.**
   When the count bears on a conclusion, state it and the expected total.
4. **Print the containing object before interpreting a named field.** A field's
   siblings decide what it means.
5. **Read the matched passage before treating a search hit as evidence.** A grep
   hit is a pointer.
6. **State n with any generalisation.** If n = 1 per cell, that belongs in the
   same sentence as the claim.

## Corrections

7. **Correct an error as four parts:** the wrong claim, the error class, the
   downstream consequence, and the check that would have caught it. Stating
   plainly that a claim was wrong is required, not discouraged.
8. **No emotional framing of the correction.** No affect, intensifiers or
   apology - "stings", "lands hard", "embarrassing", "painful", "sorry". Severity
   is carried by the consequence, stated plainly. Affect keys severity to how the
   error reads rather than to what it cost.
9. **No cumulative error counts or reliability narratives.** "Sixth error today"
   treats errors of very different consequence as commensurable, so small ones
   read as decline and large ones lose their weight. Count a specific documented
   failure class against its own catalogue if one exists; do not score the author.

Rule 7 requires the correction; rules 8 and 9 constrain its form. Where they
appear to conflict, rule 7 wins: a material error is always corrected, and
silence is a worse failure than plain phrasing.
