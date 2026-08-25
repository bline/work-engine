# Implementation review-selection contract

The slice supervisor owns specialist selection. The builder owns the exact
implementation subject and supplies a bounded projection; it does not decide
whether its own work receives a specialist review. A configured review provider
executes the supervisor's selection, and the builder remains the only role that
may evaluate findings and mutate the implementation.

This boundary lets the supervisor select reviews without inspecting source,
diffs, or test details. It does not make the projection authoritative evidence
about the implementation or let selection substitute for the specialist's own
applicability judgment.

## Builder projection

After deterministic checks are ready for review, the builder returns a
candidate-ready projection:

- the exact attributed manifest, gate identity, task-patch identity, and other
  inputs needed for the supervisor to create or identify the immutable candidate;
- the task-owned artifact manifest with each artifact's semantic role;
- the implemented consequence and changed ownership, authority, loading,
  precedence, lifecycle, interface, or other contract surfaces;
- whether materially normative agent-facing text is `present`, `absent`, or
  `uncertain`, with a concise reason and the relevant artifact references; and
- unresolved uncertainty that may make another specialist perspective useful.

The supervisor binds that projection to the resulting immutable candidate
revision before selection. The projection describes the subject. It does not select a panel, declare a
review applicable, summarize prospective findings, or authorize acceptance.
When the projection is missing or cannot support selection, the supervisor
returns to the builder for the missing bounded consequence instead of reading
the repository itself.

## Supervisor selection

Only after the candidate revision is immutable, the supervisor compares the projected consequences with the review
capabilities available through the configured provider. It records each
selected perspective and each materially plausible omission with a reason.
Availability never makes a specialist applicable, and omission is not a pass.

`agent-instruction-review` must receive one explicit selection disposition for
every implementation subject:

- select it when materially normative agent-facing text is present or when the
  projection is uncertain about an instruction, loading, precedence, or exact
  mandatory-route consequence; or
- omit it with the observed absence reason when the projection establishes that
  the implementation contains no present agent-instruction consequence.

This trigger is deliberately conservative. Selection only opens the specialist
review; the specialist itself returns `applicable` or `omitted` under its own
finding contract after inspecting the exact text and the minimum placement
context. The supervisor does not diagnose instructions.

## Execution and remediation

The supervisor sends the selection plan and immutable subject binding to the
same retained builder. The builder invokes the configured review provider for
each selected specialist in a fresh role-scoped context that does not inherit
builder reasoning or another specialist's conclusions. Provider identity and
specialist identity remain separate provenance.

The builder preserves each specialist's result and finding ownership, decides
which advisory findings are valid and in scope, performs authorized fixes, and
returns exact deltas to the same retained specialist. A material repair creates
a new immutable candidate subject. The supervisor reconsiders selection only
when the projected consequences or review premise changed; ordinary
remediation does not make the builder a selector or require a fresh reviewer.

## Terminal receipt projection

Every new slice-builder terminal receipt includes
`worker_metrics.review_selection`:

```json
{
  "selection_owner": "slice-supervisor",
  "state": "decided",
  "state_reason": null,
  "subject": {
    "revision": "<immutable candidate identity>",
    "references": ["<integrity-bound subject or artifact reference>"]
  },
  "specialists": [
    {
      "skill": "agent-instruction-review",
      "selection": "selected",
      "selection_reason": "The subject changes normative builder instructions.",
      "execution": "completed",
      "applicability": "applicable",
      "result_ref": "<revision-bound review artifact>",
      "finding_ids": ["AIR-001"],
      "unresolved_finding_ids": []
    }
  ]
}
```

When a stopped or failed slice terminates before an immutable review candidate
exists, preserve that lifecycle distinction instead:

```json
{
  "selection_owner": "slice-supervisor",
  "state": "not_reached",
  "state_reason": "The slice stopped during planning before candidate creation.",
  "subject": null,
  "specialists": []
}
```

When a stopped or failed slice has an immutable candidate but terminates before
the supervisor can disposition specialist perspectives, use `state: undecided`,
a nonempty `state_reason`, the bound subject, and an empty specialist list.

`state` is `decided` only after the supervisor has dispositioned the available
specialist perspectives. In that state, `state_reason` is null, the subject is
nonempty, and `agent-instruction-review` is present exactly once. `not_reached`
requires a nonempty reason and contains neither a subject nor specialist
dispositions; `undecided` requires a nonempty reason and subject but contains no
dispositions. Both are valid only for stopped or failed terminal receipts. An
accepted slice must record `decided`.

`selection` is `selected` or `omitted`. A selected specialist records
`execution` as `completed`, `failed`, or `unavailable`; an omitted specialist
uses `not_run`. A completed specialist records `applicability` as `applicable`
or `omitted` and names its revision-bound result. Other execution states use a
null applicability and may name a failure artifact when one exists. Selection
omission uses null applicability and result reference with empty finding arrays.
Unresolved finding IDs must be a subset of the recorded finding IDs.

For a decided selection, the `agent-instruction-review` entry is always present
exactly once. This makes selection not reached, candidate-bound selection not
decided, truthful selection omission, specialist applicability, execution
failure, and finding closure distinguishable without copying findings or source
into the receipt. Historical receipts produced before this contract may lack
the field; the active supervisor must not use that compatibility allowance for
a new slice.
