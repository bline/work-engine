# Evidence and review providers

`<skill-dir>` is the slice-builder skill's directory.

Use $repo-search for the durable `orient`, `find`, `trace`, `impact`, and `audit`
intents. Treat Codebase Memory as the current primary indexed-structure
capability and use $codebase-memory to learn and apply the interface it currently
exposes; do not copy individual operation names or today's preferred query
sequence into this durable role contract.
Select indexed structural evidence or targeted direct source observation for
each claim from the objective, repository state, available capability classes,
coverage, and observed uncertainty. Prefer indexed evidence for relationships
the current interface can establish. Use direct source observation for literal
content, runtime or presentation details, stale or incomplete coverage,
unresolved graph ambiguity, or claims the current index cannot establish.
Record the actual evidence mode and fallback reason. Current interface limits
are observed constraints, not permanent workflow law.

`claude-codebase-memory` and `claude-filesystem` remain explicitly selectable
retrieval providers backed by $claude-recon-implementation. Provider and skill
identity must resolve consistently. Record repository retrieval and independent
review as separate provenance. Do not silently substitute providers, count
retrieval as review, or weaken configured acceptance requirements. Auto provider
selection is deferred.

Version 2 may instead explicitly configure `adversarial_review` through
$codex-adversarial-review when a human has accepted Codex review. Launch it
with `fork_turns: "none"`, its configured model and reasoning effort, and no
builder-session history. Record matching builder and reviewer models as
`accepted_same_model_review`; record distinct Codex models as
`accepted_same_provider_different_model_review`. Preserve the actual model
relationship and observed isolation. Neither class establishes independent
reasoning, cross-provider, or statistical independence. Exactly one of
`independent_review` and `adversarial_review` may be configured, and accepted
Codex adversarial review cannot satisfy a requirement that explicitly demands
independent review.

Do not confuse provider selection with evidence capability. A configured Claude
role may expose Codebase Memory and bounded read-only filesystem tools in the
same process. Keep indexed structure primary for repository code understanding,
and use direct filesystem observation only for exact non-code content, reported
coverage gaps, generated or unindexed artifacts, or required host state outside
the index. That evidence-mode change does not rewrite provider identity or
require a configuration amendment. Record the actual mode and transition reason;
mere tool availability is not a fallback event. Selecting `claude-filesystem` as
the repository provider remains a distinct explicit configuration choice.

When a placement or review clause predictably depends on an external or
unindexed artifact, require the configured Claude call to provision bounded
read-only filesystem access from its first attempt while retaining indexed
access for repository structure. A reviewer must observe the material clause in
its own fresh process; a retrieval packet alone does not create independence.
