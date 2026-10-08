# CE1 and SC0 validation

Completed 2026-10-07 under the user's acceptance of parallel CE1 implementation
and SC0 characterization. The [claims plan](rust-claims-hp2-plan.md),
[campaign plan](rust-campaign-hp2-plan.md) and
[shared contract](rust-host-hp2-contract.md) remain the accepted scope.

## Completed behavior

CE1 adds a permanent `claim-evidence` Rust library with the historical claims
codec, bootstrap and registered-grant checks, a private SQLite store, finding
creation/revision and exact operation replay/reconciliation. It checks the final
stored representation before mutation, preserves historical finding UTF-16
text, rejects unsupported numeric identity inputs and retains exact recovery
identity for uncertain outcomes. Store/admission internals remain private.

CE1 uses new private roots and a controlled campaign-admission peer for domain
qualification. Newly host-issued root/grant/actor/scope/operation coordinates are
UTF-8 in this profile; historical finding subject, statement, reference and free
revision text preserve UTF-16 code units. No existing grants or databases are
imported. CE2 reliance/projection, CE3 production-path evidence and the real HP3
campaign admission join remain unimplemented.

SC0 supplies a frozen candidate/profile input handoff, pinned historical
validator and producer identities, raw compatibility vectors and real Git
objects. The legacy gate receipt does not bind its own subject. The selected
controlled profile composes that receipt with a trusted producer-side capture
of isolated execution and exact tested source, then verifies the later
candidate's tree and receipt binding. This proves one exact-path source-tree
check; it does not establish syntax checks, arbitrary gates or checks depending
on later candidate commit metadata. Earlier marker-only evidence is preserved
and excluded from the passing gate claim.

## Review and final coordinator checks

One fresh Astra/xhigh reviewer assessed both lanes in a context separate from
the two Sol/high workers. This is same-provider review, not a provider-independence
claim. One remediation cycle per lane resolved all six findings:

- CE1: final stored-state depth/readability, historical UTF-16 finding strings,
  and checked public numeric hashing boundaries; one P1 and two P2 findings.
- SC0: raw codec vectors, internal observation-consistency coverage and isolated
  Python gate execution mode; three P2 findings.

The retained reviewer verified every finding closed. The UTF-16 finding fixture
was also reproduced byte-for-byte by a pinned, pure in-memory legacy Node
`applyOperation` invocation; its source/runtime/invocation evidence is retained.

| Final check | Result |
| --- | --- |
| Locked CE1 default tests | 9 passed: 1 unit and 8 integration |
| Locked CE1 fault-feature tests | 14 passed: 1 unit and 13 integration entries, including 3 helper entrypoints |
| Locked CE1 optimized tests | 9 passed: 1 unit and 8 integration |
| Default and fault-feature Clippy, all targets, warnings denied | Passed |
| Workspace formatting | Passed |
| SC0 isolated-interpreter replay | 39 legacy oracle cases, 11 codec cases and 5 integration negatives passed; 78 unique content hashes and 3 source pins verified |
| SC0 source/gate/tree/bundle and predecessor evidence bindings | Passed |
| Frozen source and worktree scope | All 16 product hashes unchanged; no unexpected paths or executable bits |
| Main workspace/index preservation | Shared Cargo files still equal the baseline; main index bytes unchanged |

Fault tests include actual process interruption around initialization and commit,
reopen/reconciliation and competing SQLite writes. These are focused domain
gates; they do not qualify S4, the integrated native-review host, a live provider
or host responsiveness. The fault helper entries are not additional substantive
cases. Optimized/default runs exercise the same cases and are not added into a
single unique-test count.

All build/test temporary files use `/home/bline/code/.work-engine-tmp/` with
explicit Cargo target and temporary directories. Coordinator gates used Rust
1.92.0, one build job, incremental compilation disabled and no debug information.
No final gate failed. Token/cache usage is unavailable; no Claude calls were
made. A completed planning owner provided one bounded SC0 contract interpretation.
Thread capacity required archiving completed workers; no model substitution or
product failure resulted.

## Exact handoff

CE1 remains in `/home/bline/code/work-engine-hp2-ce1`, based on published S2/S3
commit `b2efdbfa0354a997b23ac4e183b89ca0dbf36cc2`. Its 16 product paths include the
new crate and private Cargo membership/lock changes. The lockfile adds only the
new package entry; no third-party version changes. Two copied planning documents
are support material and excluded from the product subject.

Evidence root:
`/home/bline/.local/state/work-engine/rust-hp2-ce1-sc0-20261007/`.

| Artifact | SHA-256 |
| --- | --- |
| `ce1/candidate2/completion.json` | `4728b7bf8208688dc44390759c250f76316ccc2346906bed0f0df2b36fb1d54e` |
| CE1 product aggregate | `9c14cc258ff9da3bd74fb487a56fa4b588775efff7145cc28951f8331b8f2721` |
| `ce1/candidate2/source.tar` | `67e5551a893212fbe05d3c835a758a62754b801460605f1fbf3e641942ee2a0f` |
| `ce1/candidate2/ce1-product.patch` | `e695a477a88ba9c526278b8a1b281084e7fc159b8c5afb57517a2c32e672f4f5` |
| `review/ce1-candidate2-review.json` | `c2e1d4775654f13ed77e3f075468ff70e12cd7b44f0285ee445f42b2f849cf4c` |
| `review/sc0-candidate2-review.json` | `53b4bf95e22f926e2ec9e5ca569f008ec9f9ea331c57d78d3a9b367747005b3b` |

The CE1 aggregate hashes manifest-order concatenation of each path, NUL, lowercase
file SHA-256 and LF. `handoff-metadata.json` records the algorithm and actual
non-executable modes. Individual file hashes remain authoritative. Candidate 1
archives and original review receipts remain preserved.

SC0's handoff is
`/home/bline/.local/state/work-engine/rust-hp2-sc0-20261007/candidate-2/subject-handoff.json`,
SHA-256 `409985af70e233377e39d0303063db184597b5fa9898b39adf07da87af835a0e`.
Its custody receipt binds the corpus and verifier. Coordinator evidence is in
`parent-ce1/results.json`, its raw logs, `parent-sc0-validation.json` and
`source-scope-audit.json` under the evidence root above.

## Next boundary

SC0 settles the bounded input handoff prerequisite for SC1; it does not implement
SC1. The next proposed parallel slices are campaign SC1 and claims CE2, using
Sol builders and the accepted domain boundaries. Shared workspace integration
remains serialized with its S4 owner from exact validated patches. CE1 has not
been merged into S4 or the main checkout. Lifecycle/UI work remains separate.

No ordinary commit, push, provider call, live-state migration, runtime activation
or cutover occurred. No main runtime source or shared Cargo file was edited.
The only main-tree changes in this task are this receipt and its roadmap link.
