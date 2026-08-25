# App Server Runtime Evidence

## Evidence status

This document records a bounded local runtime experiment performed on
2026-08-24 against the unexpectedly available `codex-cli
0.150.0-alpha.5`. It is evidence about that observed build and configuration,
not a permanent Codex App Server contract.

The recovered evidence available to this intake contains the suffixes, rather
than the complete values, of the two context-window identifiers. Those suffixes
are recorded exactly below. The raw protocol transcript and full identifiers
must be retained in a future reproducibility artifact before they are used as
exact machine subjects.

## Configuration and procedure

The experiment:

- started Codex App Server with `token_budget` enabled;
- created one ephemeral App Server thread;
- forced a 12,000-token model context;
- consumed 11,033 tokens during the first turn;
- observed native lifecycle instructions for `notes`, `history`,
  `get_context_remaining`, and `functions.new_context`;
- started a second turn after the first context exhausted its configured
  budget; and
- compared the thread and context-window identities before and after the
  transition.

The user subsequently confirmed a second bounded experiment with
`token_budget` enabled:

- invoked manual `thread/compact/start` on an existing durable thread;
- observed a fresh context-window transition;
- observed that the durable thread identity remained unchanged; and
- did not observe legacy summarizing-compaction behavior.

## Observations

- The App Server thread identity remained unchanged across the transition.
- The original context-window identity ended in `8d6fd4664ac6`.
- The replacement context-window identity ended in `a5300e564917`.
- The second window reported itself as a fresh context and identified the
  previous window.
- App Server emitted an item named `contextCompaction` during the transition.
- The observed semantic result was a fresh model context window, not continued
  reliance on an opaque summary inside the original window.

The evidence therefore supports this bounded claim:

> With `token_budget` enabled in the observed alpha build, one durable App
> Server thread survived an automatic transition between two distinct model
> context windows.

The user-confirmed manual experiment additionally supports:

> With `token_budget` enabled in the observed runtime, manual
> `thread/compact/start` triggered a fresh context-window transition on the same
> durable thread and did not exhibit legacy summarizing-compaction behavior.

It also falsifies this inference:

> An App Server item named `contextCompaction` is sufficient evidence that the
> provider performed summarizing compaction.

## Required transition classification

The runtime adapter should classify observed transitions using multiple
signals:

- stable or changed thread identity;
- prior and current context-window identities;
- preservation and readiness state bound to the prior revision;
- event ordering;
- whether continuation relies on an opaque generated summary; and
- post-transition runtime and rehydration evidence.

At minimum, the normalized classifications should distinguish:

- `fresh_context_transition`;
- `summarized_compaction`; and
- `unknown_transition`.

Provider event names remain evidence, not the semantic classification owner.

## Consequences for the Context Manager direction

- App Server is a credible first runtime scaffold rather than only a later
  richer realization.
- A logical role can retain one thread binding across successive context
  windows.
- Token pressure should trigger preservation before the emergency limit is
  reached.
- Clearing remains a separate semantic-safety and optimality judgment.
- Native fresh-context replacement is allowed after continuation readiness.
- Manual `thread/compact/start` is an observed host control for requesting that
  fresh-context transition under `token_budget`; its provider event still
  requires post-transition classification.
- Opaque summarizing compaction is prohibited as the sole continuation
  mechanism.
- Snapshot binding and a write fence remain necessary because meaning created
  after an inspection revision must not be retired by a stale decision.
- Transition classification and post-transition reconciliation require an
  external lifecycle ledger rather than reliance on the observer's context.

## Limitations and unresolved experiments

- The build is alpha and does not establish behavior for other Codex versions.
- The raw protocol transcript and complete window identifiers are not present
  in the recovered intake evidence.
- The confirmation does not establish whether a separate direct `new_context`
  host method exists; manual `thread/compact/start` is the observed host
  initiation surface.
- The exact intervention and fencing mechanism before automatic exhaustion is
  unresolved.
- The supported source for context-window identity remains to be established
  without relying on unstable internal response details.
- Thread-local skill isolation and experimental dynamic-tool behavior require
  versioned integration tests.
- Manual fresh-context behavior must be reverified for every supported Codex
  version rather than inferred from the method name.
- Recovery after replacement that occurs before a readiness receipt remains
  unresolved.
