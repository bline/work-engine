# Slice 1b packet: spec/structural author — sole-C ownership and schema/number domains

Owner/root: Pi session `01a1e569-2805-72ff-b837-03d1577c405d`. You are the spec/structural
author for this slice. Root alone joins, reviews and accepts. An independent Fable review of
your exact diff follows.

## Workspace

Your git worktree (branch `spec-r1`, based on the accepted predecessor `5e594cac`):
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/spec-r1`
Commit your work on this branch (author `pi-accounting-spec-author`). Use cargo target dir
`/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/spec-r1-target`.

Exclusive writes: `spec/**`, `tooling/spec-check/src/**`, structural/schema/wire test files
under `tooling/spec-check/tests/` (not `independent_raw_axes.rs` or `independent_r8_controls.rs`,
which belong to the oracle), crate `Cargo.toml`/`Cargo.lock`, `tooling/spec-check/README.md`,
and regenerated `contract/CK-GENERATED.md`, `contract/TRANSITIONS.md`, `corpus/SCHEMA.md` only
if your canonical changes require regeneration. Do not touch `oracle/**`, `validator/**`,
`corpus/cases/**`, raw/historical bytes, or financial expectations.

## Required consequences

### R8-OWNER-01 (P2; report C1-R8-01)
C-validity has a sole owner. Today `tooling/spec-check/src/lib.rs` (~line 589–597) rejects only
S constructing `SettlementDecision`; the same competing authority placed in V (exact
counterexample: replace `spec/ck.json` rule `V-PRESENCE-1.outputs` with
`SettlementDecision.valid` using its declared fields `basis`, `charged`, `settlement` and empty
bindings) passes `canonical`. Required: canonical/static checking rejects any non-C stage
constructing a C-owned decision (all writable constructors and stages, not only V), with a
controlled error naming the owner violation. Valid C construction and S's typed forwarding of
C's reference continue to pass.

### R8-SCHEMA-DOMAIN-01 (P2; report C1-R8-03)
Published Draft 2020-12 schemas and the Rust canonical/wire/evidence representations agree on
every numeric and string domain, and unsupported input gets a controlled validation error, never
a panic. Known counterexamples: `ck.types[0].revision = 4294967296` is schema-valid but Rust
needs u32; input origin `original_position = 18446744073709551616` is schema-valid and Rust
`evidence` panics (exit 101) at `as_u64().unwrap()` (`src/accounting.rs` ~line 94). Required:
- agreement at exactly 4294967295 / 18446744073709551615 (accepted) and +1 (rejected
  normally by both);
- audit all numeric/string domains in `spec/*.schema.json` against Rust and fix every
  disagreement and unchecked conversion/overflow you find (checked arithmetic returns a
  controlled error; no wrap, saturation or panic);
- representation agreement, not just bounds: integral decimal/exponent JSON numbers
  (`1.0`, `1e0`), non-integral fractions, negatives, booleans, strings, and precision around
  the u64 frontier. Keep standard schema integer semantics with exact checked conversion — do
  not silently reject a representation the published schema accepts, and do not round via f64.
  If a parser/library cannot support this, verify its capability and say so; dependency or
  feature changes need exact version/features/lock/license evidence;
- `spec/ck.json` MONEY defines Nano as checked i128 and SERIALIZATION defines nonnegative
  canonical decimal strings: publish the corresponding maximum string domain in the standard
  schema (i128::MAX accepted, max+1 rejected) rather than inventing a new financial limit.

## Evidence required in your result

- Discriminating controls added to the structural/schema tests: each negative names the
  predicate it is meant to hit (an unrelated digest/schema failure does not count); positives
  prove no blanket rejection.
- The predecessor's nine gates still pass on your branch:
  `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/tools/replay_gates.sh <worktree> <target> <outdir>`
  (outdir under `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/spec-r1-evidence/`),
  plus `cargo test --offline` for the crate and `tooling/spec-check/tests/schema_agreement.py`.
- If you change protected canonical nodes, run the `impact` command for them and report
  dependency/digest consequences. Do not regenerate or re-acknowledge the 19 null
  acknowledgements to make digests match; report them.
- Write `RESULT.md` in `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/spec-r1-evidence/`:
  commits, files changed, each consequence with its controls and exact command outcomes, every
  domain you audited and its disposition, and anything left open.

## Limits

No network, no paid APIs, no child agents, no edits outside the worktree and evidence dir.
Stop and report if a fix would require a new financial policy or touching oracle-owned files.
