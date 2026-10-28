# Astra review of a Claude builder

The Codex caller launches and collects a separate top-level `gpt-6-astra`
reviewer through [manual orchestration's Codex host](../../manual-orchestration/references/codex-host.md),
with the effort from [model selection](../../../docs/model-selection.md).
This keeps the reviewer outside the caller's child tree and explicitly selects
the standard tier even when `/fast` changed shared configuration.
This is the review pairing selected for this skill; an explicit
owning task contract may select otherwise. The reviewer has a fresh entry and
does not inherit the caller's or Claude builder's conversation.

Use manual orchestration's reviewer/worker role. Give it the accepted objective,
constraints, exact immutable subject or integrity-bound snapshot, attributed
change manifest and baseline overlaps, relevant source/instruction paths,
validation evidence, and selected specialist contracts. The reviewer directly
inspects the subject and its dependencies as needed; a builder summary is not
substitute evidence. Do not send hidden reasoning or the builder transcript.

The review assignment permits observation and its assigned review artifacts,
not repository mutation, fixes, commits, or publication. Constrain capabilities
to the extent the owning review claim requires and preserve actual grant
evidence. A read-only prompt is a behavioral instruction, not an independently
observed read-only capability grant. If a stronger independence claim is
required, use an execution realization that can establish it; otherwise return
the exact unmet condition to its owner instead of certifying it from a prompt.

Require findings with stable IDs, severity, source evidence, consequence,
limitations, and a clear blocking/nonblocking disposition against the assigned
acceptance criteria. For normative skill/instruction changes, load
`skills/agent-instruction-review/SKILL.md` and its finding contract, preserving
that specialist's advisory authority. The caller judges findings and acceptance;
Claude performs valid in-scope repairs and reruns affected checks.

Retain the Astra session through remediation. After the orchestration context
check, send the exact new subject/delta, prior finding IDs, repair summary and
new gate evidence with `role_host.py deliver` to the same top-level reviewer.
Record resolved,
open, or uncertain findings. A changed subject invalidates a prior acceptance
unless its continued applicability is established. Replacement follows the
owning handoff/review-state contract and preserves prior findings and lineage.

When a workflow configures authority-bound review state, use its exact episode,
subject, writer generation and publication contract. This manual adapter does
not create an independent-review-state episode implicitly or replace campaign
receipts with its own result.

Record observed builder and reviewer identities, providers, fresh-entry
isolation, capability evidence, subject bindings, continuation/replacement, and
limitations in the existing result. Claude/Astra is a different-provider pairing
when those identities/routes are observed; that fact alone is not proof of
statistical or reasoning independence. Unknown identities remain unknown.

The current `codex-adversarial-review` skill is scoped to Codex builders and
same-provider evidence classes. Do not load it unchanged for this pairing or
mislabel a Claude builder as Codex merely to fit its schema. This reference
supplies the manual review adapter; campaign support needs the campaign's
actual provider and evidence-class contracts to admit the pairing.
