# H1 R1 remediation validation

## Result and exact subject

`J2H1-R1` is fixed against `c4952af0542763de365ead11ba69d70a1347d831`.
`TrustedStartup::load` validates both derived native begin/result request and
grant identifiers with the actual episode `Request::parse` envelope contract
before returning startup authority. The IDs come from `dispatch_command`, so
suffix lengths are validated after derivation. No independent 121-byte constant
or competing ASCII rule was introduced. Root-selection placeholders are valid
protocol syntax only and create no grant, root or dispatch authority.

The permanent CLI now refuses 122/128-byte base operation IDs and internal spaces
before any of the four owner roots exists. Existing tab/non-ASCII rejection is
also tested. A 121-byte ID including previously permitted punctuation completes
one real controlled peer execution, returns the truthful EvidenceUnestablished
outcome, and recovers the exact original receipt without another peer entry.
Ordinary historical IDs retain existing behavior. Existing stranded roots are
neither rewritten nor re-dispatched; this change supplies no repair authority.

Owned files:

| Path | SHA-256 |
| --- | --- |
| `rust/crates/review-workflow/src/startup.rs` | `b3969c61e492df7e7b1dbda062fe26596d7223d6d8ee114e92867f266af880d4` |
| `rust/crates/work-engine-review-host/tests/controlled_host.rs` | `89c1c851fac6549e069ed5b268bcce97ac5dcddf9debb10edc3050e1efdda1a4` |

The third owned path is this validation report. No Cargo, campaign/CE reducer,
provider, SC3 or planning-file edit belongs to this remediation. No staging,
commit, publication, activation or credential operation was performed.

## Validation

All commands ran serially with an independent target under
`/home/bline/code/.work-engine-tmp/h1-r1-20261013/`. The frozen snapshot contains
the committed baseline plus only the two R1 source/test changes. It uses the
published Cargo configuration, excluding unrelated unpublished S8 additions;
concurrent R2 source edits are not part of this isolated qualification. Parent
owns binding the combined remediation subject and retained-reviewer reevaluation.

| Gate | Result / raw log |
| --- | --- |
| Baseline reproduction with new CLI test | Expected failure: original startup admitted 122 ASCII bytes; `baseline-reproduction.log` |
| Release, both host/workflow packages, all features, serial | 21 passed, 0 failed, 0 ignored; `release-regressions.log` |
| Debug permanent-host focused native boundary/recovery regressions | 3 passed, 0 failed; `debug-focused.log` |
| Debug workflow default features including authority compile-fail cases | 8 passed, 0 failed; `debug-workflow.log` |
| All-target/all-feature Clippy, `-D warnings` | Passed; `clippy.log` |
| Package formatting | Passed; `format.log` |

Release includes all 11 existing actual-process crash cuts and independent peer
entry witnesses: recovery adds zero entries at every cut. All five rejected
startup variants leave campaign, episode, claims and evidence roots absent and
the external peer-entry witness absent. The accepted 121-byte case checks the
original outcome receipt after reopen and repeat initialization.

The existing 700 external status probes passed during cold reconstruction,
actual capture, campaign contention and four simultaneous requests. Worst p99
was 0.198 ms, maximum 1.033 ms, versus existing 50/250 ms gates. The successful
operation took 4,019 ms including the 3-second controlled peer; one outcome,
three busy refusals, no timeouts/unresolved outcomes, one peer entry. Raw probe
samples and operation timing are in `release-evidence/`; memory was not observed.

A stale-build qualification attempt is retained as
`release-superseded-stale-host.log`. Copying the fixed source with preserved
mtime while the baseline build completed left the host linked to baseline bytes;
the new regression detected the mismatch. No test ran concurrently with a
rebuild. After that suite exited, only snapshot mtimes were advanced and local
crates rebuilt. `stale-host-diagnostic.json` records the old binary and cause;
`release-identity.json` records the rebuilt binary. Final tests above ran that
new binary, and all 49 captured release startup records bind its exact hash and
the exact snapshot Cargo.lock (`release-startup-binding-check.json`). The failed
attempt is not included in the passing gate counts.

## Identity and evidence

Release host SHA-256: `83ec5ef28791f913399484873a9b5ce2a40b543d440a9cad4c6ee4e1153c7f1a`.

Published Cargo.toml: `4d7a13318ae89695d4b320770b83a2469fd5534a5ce203c28b009a68e6a67fc0`; Cargo.lock:
`aed46285aacbcaf89f4ac6122c947d8c6fc944ce4a109d8b0514ba78b0cb58b9`. Rust/Cargo 1.92.0, x86_64-unknown-linux-gnu.

`snapshot-source-manifest.json`, `frozen-owned-source.json`, `baseline.json`,
`release-identity.json`, `final-command-results.json` and `validation-receipt.json`
retain exact source/build/log identities. Final shared source bytes equal the
qualified snapshot bytes. Both preexisting dirty Cargo files are byte-identical to the task baseline.
The Git index binary metadata changed during concurrent work, but all 2,509
mode/blob/stage entries match the reconstructed publication baseline (preserved
prepublication entry list plus the exact 29-path c4952af delta); all 70 unrelated
staged paths remain. `index-preservation-check.json` records that distinction.
No staging or index-content mutation was performed; unrelated work is preserved.

Tier 2 graph verification used project `home-bline-code-work-engine`;
startup `load` was traced both directions, depth one, and the exact native parser
read. Initial six-path coverage and final five-path coverage had no recorded gaps
and verified content (final generation `2026-10-13T04:59:33Z`). Direct source reads
verified envelope construction and test behavior; graph edges are best-effort,
not a completeness claim. The final implementation is one private startup helper;
no native owner schema, grant contract, marker or recovery semantics changed.

This is provider-disabled qualification. It does not establish a real provider,
enforced read-only reviewer realization, acceptance of R2, or combined review
acceptance. The retained reviewer and parent own those separate decisions.
