# Parent validation — 2026-10-07 UTC

## Current outcome

The bounded live proof passed using exactly all three authorized entered native
turns on `gpt-6-sol`: first input on A, readiness on successor B, and second
input on B. One coordinator (PID `16324`) remained alive throughout. A used
thread `01a114ce-4f25-78c3-bfe5-f5ad5ef833b0`; B used
`01a114ce-5ed8-7832-b55e-7dbb896d67ab`. The second input did not contain the
carried value, and returned `SECOND:ORION-7` correctly.

Post-close readback found coordinator revision 38 with three successful
`CodexTextTurnCompletion` effects, three successful deliveries, and one
transition. The adapter recorded exactly three commands/turn records and two
generation/thread bindings, with no text-contract violations. This establishes
the bounded text-only experiment, not arbitrary external-effect settlement,
network-wide exactly-once execution, or production readiness.

The receipts, selected durable readback, authenticated preflight, and coherent
SQLite backups are in `evidence/live-text-vertical-2026-10-07/`. Both backups
passed `integrity_check`; they include the committed WAL-visible state rather
than merely hashing the uncheckpointed source database files. Provider PID
`15240` and coordinator PID `16324` were stopped and reaped (exit 143). Their
native child PID `15340` was confirmed absent; its exit status was not directly
observed. Cleanup is not the basis for effect settlement. The separate
disposable profile was successfully logged out; the normal profile was not used
or changed. All three live turns are consumed.

An earlier setup attempt was rejected before entry because the example used
Unix seconds while the engine requires Unix milliseconds. Durable readback
proved zero effects, deliveries, provider commands, or bindings. That attempt
remains preserved. The recipe was corrected, a pre-command deadline check and
regression were added, and fresh coordinator state was used after validation.
See `evidence/live-admission-rejection-2026-10-07/diagnosis.json`.

## Provider-disabled validation

The parent ran the frozen standalone Rust crate's validation after the retained
design review identified and the implementation corrected native text/reasoning
stream handling.

- Formatting: passed.
- Default tests after the deadline correction: 30 passed (8 unit/binary tests,
  17 integration, 5 compile-fail doctests);
  7 opt-in native probes were excluded from the default run.
- Clippy, all targets, warnings denied: passed.
- Explicit actual-CLI controlled text vertical: passed.
- Explicit native asynchronous-input adverse case: passed, preserving
  `UserInputRequested` and strict `Unknown`.
- Explicit disabled-host execution rejection case: passed.

The parent's latest controlled vertical kept coordinator PID `11071`. Native thread
A was `01a114cd-31cc-7612-9b9e-79788964cf53`; successor B was
`01a114cd-3232-75e3-8413-07da03d253e8`. Exactly three simulated inference requests
were observed. The continuation-package digest was
`e6dfe045167b2853676cdc9e4dda3ba47101d98c9bb21b45f9a73faf25b6f63b`.
The second input did not repeat the carried fact. The coordinator accepted
three scoped text receipts; this is not arbitrary external-effect settlement
or a network-wide exactly-once claim.

Controlled probe directories under `evidence/` are mutable latest-run evidence.
The live result has its own separately named directory. Do not combine run
identities with earlier coordination-board reports. The retained design
reviewer inspected the streaming correction and found no remaining blocker
for the bounded trial, subject to live preflight. That was a narrow retained
review, not fresh independent review or production approval.

## Live trial boundary

The user authorized at most three isolated text-only native turns, including
readiness and uncertain entered attempts. No production state, external tools,
user work, publication, activation, or implicit retries are authorized.

The user separately agreed to a fresh disposable Codex sign-in. The parent
prepared `/tmp/context-lifecycle-live.yyp1P6`, verified login completion, and
ran no-inference account/config/model preflight before the proof. No credentials
were read, copied, or included in repository artifacts. The allowance is now
fully consumed; the prepared profile has been logged out. The exact validated
source/build inputs are bound in `parent-validated-sources.sha256`.

## UI handoff

Controlled and live native sequences are available for read-only historical
lifecycle views. The separate UI session reported that it built an illustrative
view, retaining distinct native observation and lifecycle-outcome fields and
leaving commands disabled. No UI files were changed by this lifecycle work.

The public experimental `Observer`/`Client` interfaces are tested, but no
versioned UI wire contract has been announced. A versioned read-only projection
is the next running-engine integration handoff. The live proof now establishes
the bounded text-only case, not production command integration.
