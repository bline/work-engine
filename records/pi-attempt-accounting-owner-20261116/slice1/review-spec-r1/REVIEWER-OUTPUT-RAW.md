I now have everything needed; nothing further to request. Below is the completed review artifact.

---

**Artifact — to be persisted at** `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/review-spec-r1/REVIEW.md`
(no write-capable tool was available in this session; the runtime should persist this content verbatim)

```
# Independent review — spec-r1 commit 2729107 (R8-OWNER-01, R8-SCHEMA-DOMAIN-01)

verdict: ACCEPT
architecture_assessment: bounded_repair_credible

Reviewer: Fable (claude-fable-5-1, xhigh), standing in for Astra by operator direction.
SAME-PROVIDER REVIEW: reviewer, root and builder share one provider. Read-only; I did not
author or modify anything. No shell was available to me, so I executed no binaries;
every "executed" result below is the builder's recorded output, which I cross-checked
for internal consistency against the source, or my own reasoning from the code, labelled
as such. Base compared by reading the `s1-base` worktree
(`successor/subject`, HEAD = refs/heads/s1-base) against the clean commit checkout
(`/tmp/rootv-spec`).

## 1. What must be true

### 1.1 R8-OWNER-01 / C1-R8-01 — HOLDS
Source seam (`tooling/spec-check/src/lib.rs`):
- 340-357: `C_OWNER_STAGE="C"`, `C_OWNER_BOUNDARY="C-validity"`, `C_OWNED_OUTPUT=
  "settlement_validity"`; `c_owned_types` = fixed point of {SettlementDecision} ∪ every
  type with a variant field whose `named_type` (decisions.rs:63-71, strips nested
  `List<`/`Option<`) resolves into the set.
- 569-585: every stage ≠ C and every boundary ≠ C-validity declaring
  `settlement_validity` in `outputs` → `C-validity owner violation: {id} declares C-owned
  output …`.
- 604-610: every rule with `stage != "C"` whose `outputs[].type_id ∈ c_owned` →
  `C-validity owner violation: stage {S} rule {R} constructs C-owned {T}; only stage C may
  construct it`. Runs before read-set, formula, replacement-binding and constructor-field
  checks, so the diagnostic is the owner predicate, not an incidental failure (the
  predecessor rejected the F mutant with "replacement result/reason disagreement" and
  accepted the V mutants — `c_owner_cli.predecessor.out`).
- 636-643: S-DECISION still requires `SettlementForward` with
  `bindings.validity_ref == "C-validity.validity_ref"` (unchanged positive path).
Writable-constructor census (so the check is complete over the model): the only typed
constructors in `Spec` are `rules[].outputs: Vec<Constructor>` (lib.rs:76-83).
`Branch.result` is `Option<FinancialResult>` of strings (decisions.rs:15-32),
`ObligationBinding` is strings (35-41), `Projection` maps type variants to wire shape and
is forced total (decisions.rs:316-370; one projection per variant, orphan check), and
boundaries/stages carry output *names* only (covered by the name check). Base vs subject:
the old S-only test `r.stage=="S" && type_id=="SettlementDecision"` (base lib.rs:589-591)
is gone.
Controls (recorded): `c_owner_cli.slice1b.out` PASS 8/8 through the real `canonical` CLI
— exact C1-R8-01 counterexample rejected with `…stage V rule V-PRESENCE-1 constructs
C-owned SettlementDecision…`; S/F/V/S-TIME rules each rejected naming stage+rule; stage V
declaring `settlement_validity` rejected; unmodified spec and C-KEY-1 constructing
`SettlementDecision.valid` accepted. `c_owner_domains.rs` (11/11 in cargo-test.out):
every non-C rule × all five variants rejected naming stage+rule (≥30 rules), C-KEY-1 × all
five accepted, carrier type (`ValidityEnvelope{inner: SettlementDecision}`) rejected for V
/ accepted for C with its total projection, stages E,S,V,F,X,K and boundaries
C-primary,C-auxiliary,S-consume rejected for the output declaration, S forwarding positive
+ wrong-ref negative (`forward immutable validity_ref`).

### 1.2 R8-SCHEMA-DOMAIN-01 / C1-R8-03 — HOLDS
Published domains vs Rust (all verified by reading both sides):
- `ck.schema.json`: every `type: integer` site — Boundary(20-24), Branch(79-83),
  Decision(149-153), Formula(236-240), Node(273-277, shared by invariants and
  owner_policies), Projection(339-343), Rule(461-465), Stage(514-518), TypeDef(545-549) —
  is `minimum 1, maximum 4294967295`; Rust fields are `u32` (lib.rs, decisions.rs:7,17,45).
  Schema validation runs before `from_value` (read_spec lib.rs:133-145; `canonical`
  main.rs:58-65; `validate` re-checks `json(s)` at 364-369), so revision 0 and 2^32 are
  Draft2020-12 rejections, never serde errors.
- `accounting.schema.json:974-976` `original_position` `0..=18446744073709551615`; the
  former panic `o["original_position"].as_u64().unwrap()` (base accounting.rs:94) is now
  `exact_u64(..).ok_or("origin position outside u64 domain")?` (accounting.rs:94).
- Nano strings: pattern at accounting 559/925/941/945/1148 and wire 32/55/191/203/245/285,
  each with `type: string`. I verified the generated pattern digit-by-digit against
  i128::MAX = 170141183460469231731687303715884105727 (39 digits): `0`,
  `[1-9][0-9]{0,37}` (1–38 digits), one alternative per position whose digit > 0 with
  class `[0..digit-1]` and the right free-digit count (e.g. `1[0-6][0-9]{37}`,
  `1700[0-9]{35}`, …, `17014118346046923173168730371588410572[0-6]`), and the exact
  maximum. That is exactly the canonical-decimal interval 0..=i128::MAX, identical to the
  Rust `Nano` predicate (lib.rs `validate_value`: digits only, no leading zero, `i128`
  parse). No new financial limit (MONEY/SERIALIZATION). Generator `tests/nano_domain.py`
  matches; `numeric_domains.py` additionally probes 391 frontier integers against the
  compiled pattern independently of any validator (recorded PASS).
- `U64` constructor values: `exact_u64` (lib.rs `validate_value`); `corpus.rs:131`
  `u64::try_from(i+1)`, `:140` `exact_u64`; `applicability.rs:157-158` exact string
  comparison `pos == p.to_string()` (replaces `as_u64()==parse::<u64>().ok()` which
  matched `None==None`), `:293-296` `exact_u64` + `usize::try_from`, `:301`
  `checked_sub`; lib.rs:380-384 zero-revision via `exact_u64`; `product()` lib.rs:322
  `saturating_mul` only feeds a bounded-limit *error*, not a value.
Representation (numbers.rs): `classify_literal` (41-78) does exact decimal
classification — strips sign, clamps |exponent| at 2^40 (unparseable exponent digits →
clamp), concatenates integer+fraction digits, strips leading zeros, scale = exp −
fraction_len, strips trailing zeros; `scale<0` ⇒ NonInteger (correct: last digit nonzero
after the point); >40 digits ⇒ LargeInteger; else exact digits. I traced `1.0`, `1e0`,
`1E+0`, `10e-1`, `-0`, `-0.0e5`, `0e999999999999`, `100.00e-2`, `0.5e1`,
`4294967295.0000000000000001`, `18446744073709551614.99999999999999999999`,
`1.8446744073709551615e19`, `1e-400`, `1e400`, `1e99999999999999999999` — all classify
correctly without f64. `number_value` (125-146): exact `u64`/`i64` (with the i64::MIN
special case), `±1e40` for over-wide integrals, native f64 for representable fractions,
`0.5` for fractions f64 would collapse (incl. `1e-400` → 0.0 → 0.5). Every published
numeric constraint is `type: integer` + `minimum` + `maximum`; jsonschema's integer test
(`is_u64 || is_i64 || f64.fract()==0`) classifies `±1e40` as integers that fail the
bound and `0.5` as a type failure, so both sentinels are controlled Draft2020-12
rejections — confirmed by recorded tests (`1e400`, `4294967296.0`, `18446744073709551616`,
`…615.5`, `1e999999999999` all `Draft2020-12`). `exact_u64` (84-92) never casts out of
range; stock-parser `Value`s are read at face value (f64 2^64 → None, controlled).
`canonicalize_integers` (98-117) is applied in `validate_accounting_bundle` so continuity
compares integers, not spellings (`1` vs `1.0` equal; `2` → `continuity drops or rewrites
origin`; recorded).
Parser faithfulness (Reader, numbers.rs:152-347): JSON whitespace = SP/TAB/LF/CR;
`-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?` grammar with `01`, `1.`, `.5`, `1e`, `+1`, `-`
rejected; strings reject raw control chars, validate UTF-8 of raw bytes, handle all nine
escapes, require a trailing surrogate after a leading one and reject lone trailing
surrogates via `char::from_u32`; objects require string keys, last duplicate wins
(`Map::insert`, same as serde_json's Value visitor); depth bound 127 (= serde_json's 128
limit: 128 nested `[` fail, 127 pass — unit-tested both ways); trailing non-space fails;
BOM fails. Unit tests (3) and the differential test over >300 repository JSON files
(`exact_parser_agrees_with_serde_json_on_every_repository_json_file`) passed. Intended
divergences from serde_json on *valid* JSON are numeric only: `1e400` parses (serde_json
errors), `-0` is integer 0 (serde_json f64 -0.0 — observably identical downstream),
`1.0` is an integer. No non-numeric divergence found.
Consumer paths: `parse_json` is used by `canonical` (main.rs:58), `evidence`/`wire`
(main.rs:77) and `read_spec` (lib.rs:139). It is deliberately NOT used by `corpus.rs:11`
(corpus/oracle files) or `applicability.rs:67` (referenced payload files): those feed
`serde_json::to_vec`/`to_string` digests (`applicability.rs` `source_digest`,
`corpus.rs canonical(&c["inputs"])`) that must keep the stock f64 formatting to match the
Python/recorded digests; switching them would change digests. I checked every
mixed-parser `==` between a bundle value (parse_json) and a payload value (stock): the
compared objects (`InitialReferenceContract`, accounting.schema.json:829-864) and fields
are strings/enums only — no numeric equality crosses parsers.
Panic audit: remaining `unwrap()` sites reachable after schema validation
(accounting.rs:253, 271, 286, 319, 350, 362, 368, 586-587, 620, 629) are each guaranteed
by the schema (Base requires boolean `atomize`/`break_copies` 334-376; DefiniteLink
from/to/link_id required strings 583-609; Grant.grant_id required 718-756; Origin
.initial_contract_ref required 965-997; SlotBinding.origin_ref required 1195-1216;
Unit.successor_ref string and guarded by `get` 1321-1324) or by a preceding
`refs()`/`len()>1` check; note `validate_result` can run before `validate_input` when a
result document precedes its input in `documents`, but every document is
schema-validated in the first loop of `validate_accounting_bundle` before either runs.
lib.rs:624 is guarded by `fs.contains(f)` (621-623); applicability.rs:66 by
`paths.len()==1` (65). Recorded `no_panic_mutations.py`: 1308 runs, no exit 101/signal
(seeded sample, not exhaustive).
Controls: `numeric_domains.py` 983/983 PASS (expectations computed with exact
Decimal/Fraction, Rust must exit 0/1, every negative must carry `Draft2020-12` or the
named continuity predicate; Python jsonschema second oracle only where Python's float
parse is exact: 46+46+162). Same script on the predecessor binary: 214 failures incl.
`invalid value: integer 4294967296, expected u32`, `invalid type: floating point 1.0,
expected u32` and exit 101 for `original_position = 18446744073709551616` — the exact
C1-R8-03 counterexamples. `c_owner_domains.rs` pins u32::MAX/+1, u64::MAX/+1,
i128::MAX/+1 in-process for schema, `validate`, wire, `validate_constructor`, bundles.

### 1.3 No regression — HOLDS within what I could verify
- Nine gates rc=0 (`gates/results.jsonl`); `cargo test --offline` all suites pass
  (`cargo-test.out`: lib 18, c_owner_domains 11, independent_r8_controls 2,
  independent_raw_axes 11, r9_spec_applicability 7, r9_spec_payloads 6, structural_r8 11).
- Oracle-owned files untouched per builder; `independent_*.rs` pass unchanged;
  `validator/` not modified (pycache restored).
- `spec/dependencies.json`: 19 `"acknowledgement": null` at identical lines in base and
  subject; `check ROOT --generated` → `structurally_valid`, 211 nodes, 19 unestablished
  consumers (`gates/generated.out`). Impact for SCHEMA-ACCOUNTING/SCHEMA-WIRE names 16
  consumers (`impact-changed-nodes.json`); digest spot-check: SCHEMA-ACCOUNTING,
  SCHEMA-WIRE, consumer-T, view-contract changed; C-validity, MONEY, SettlementDecision,
  USAGE-PRESENCE unchanged — consistent with "exactly 18 changed".
- `ck.json`: only SCHEMA-ACCOUNTING rev 3→4 and SCHEMA-WIRE rev 1→2 plus their embedded
  text (lines 2684-2700 both trees); `publish_domains.py` is a mechanical pass (maximum on
  every bounded integer, nano pattern replacement). No financial policy changed.
- `Cargo.toml` read: pins unchanged, `jsonschema default-features=false`, no serde_json
  feature (Cargo.lock not read).
- `tests/structural_r8.rs` diff: `s_cannot_construct_or_replace_validity` message
  `"S cannot construct"` → `"C-validity owner violation"`; fixtures gained `link_id`,
  `admitted_payload`, `primary_memberships`, `usage_origin_refs`, retained components,
  and the "rejected-dollar" negative gained a matching membership row so the intended
  `dollar primary evidence` predicate is reached. Every assertion fragment is unchanged
  (`interchanged grant`, `complete valuation`, `non-valuation`, `whole-unit`, `invalid
  primary`, `computed unbudgeted applicability`, `dollar primary evidence`, `incomplete
  physical-credit`, `rewrites origin`). The claim that three tests already failed on the
  base is corroborated structurally: base fixtures' `exact_links` lack `link_id` while base
  `accounting.rs:137/149` require `link_id`/`admitted_payload` via `keyed(..)?`, so
  `validate_accounting_bundle(&b).unwrap()` panicked. Not executed by me.

## 2. Findings (none blocking)

P3-1 — Sentinel collapse is observable at one published `uniqueItems` location
(introduced form; collision class inherited).
Location: numbers.rs:119-146 (`sentinel`, `number_value`) × accounting.schema.json:296-305
(`Assertion.expected_fields_or_relations: {type: object}`, free-form) inside
`AccountingExpectedAssertion.assertions` (`uniqueItems: true`, line 11).
Reasoning (not executed): two assertions identical except
`expected_fields_or_relations: {"n": 1e40}` vs `{"n": 1e41}` (or any two >40-digit
integers, or any two fractions f64 collapses, e.g. `1e-400` vs `1e-401`) both become the
same sentinel, so jsonschema `uniqueItems` fails → `Draft2020-12 … non-unique`, while
standard Draft 2020-12 (exact) accepts the document. Predecessor: distinct f64 for the
first pair (accepted) but collapsed other pairs (`18446744073709551616` vs `…617` → both
2^64) and errored on anything beyond f64 range — so the class is inherited, the instances
moved. Expected: schema and Rust agree; real oracle assertions use string amounts
(`A3.expected.json`), the differential test over every repository JSON file passed, and
no financial domain is involved. The builder's residual risk #1 describes this as needing
a *future* schema; it is reachable now at this one free-form location. Smallest fix
options: (a) document the exact location/limitation in README/RESULT; (b) for over-wide
integrals store the literal's finite f64 when finite (sentinel only when infinite) to
restore f64-resolution distinctness for integrals — fractions ≥ 2^53 cannot be
represented non-integrally in f64, so the `0.5` sentinel must remain there.

P3-2 — Python `re` `$` accepts a trailing `\n` (inherited; report only).
Location: nano pattern `^(…)$` at all 11 sites (and the former `^(0|[1-9][0-9]*)$`).
Rust/ECMA reject `"1\n"`; Python `jsonschema` (search semantics, `$` before final newline)
accepts it. Affects Python reference validators only; builder excluded such probes from
the Python oracle and asserted Rust-rejected. Expected: reference validators use
fullmatch semantics or the pattern gains `(?!\n)`-free equivalent; outside this slice.

P3-3 — Documentation inaccuracies (introduced; RESULT.md/README only).
RESULT.md names the nine `ck.schema.json` revision locations as
"types/stages/rules/formulas/invariants/owner_policies/decisions/projections/boundaries";
the actual nine `$defs` are Boundary, Branch, Decision, Formula, Node (shared), Projection,
Rule, Stage, TypeDef — i.e. `decisions[].branches[].revision` is bounded (good, matches
Rust `Branch.revision: u32`) but unnamed. README §slice 1b lists only
"type/boundary/rule/formula/projection revision" and says "All Rust integer reads use
`exact_u64`/`exact_u32`" — `exact_u32` is unused in `src/` (revision range is enforced by
schema + serde's checked `u32`). Harmless; correct the text.

P3-4 — Inherited non-discriminating negatives in the `evidence_controls` gate (oracle-owned;
outside the diff).
`gates/evidence_controls.out`: `VC07-missing-completion.wrong.json`,
`VC07-relabel-dollar*.wrong.json` and `VC07-unlinked-release.wrong.json` are rejected by
schema shape (`exact_links[]` without `link_id`; completion without `usage_origin_refs`)
— `… is not valid under any of the schemas listed in the 'oneOf' keyword` — not by their
named predicates. The gate only compares expected/actual validity, so it passes. Not
caused by this commit (the `link_id` requirement predates it). Report to root/oracle
owner.

P3-5 — New controls are not gate-protected (process; root-owned tooling).
`tools/replay_gates.sh` builds with `cargo test --no-run` and runs only filtered
`independent_raw_axes` tests plus Python scripts; `c_owner_domains.rs`, `structural_r8.rs`,
`c_owner_cli.py`, `numeric_domains.py`, `no_panic_mutations.py` ran only by hand (outputs
recorded). README already names structural_r8 as "meaningful" yet un-gated — which is how
three of its tests silently failed on the accepted predecessor. Recommend root add
`cargo test --offline` and the three scripts to the replay.

P3-6 — Observation on the ownership model (inherited design boundary, not a defect).
Opaque `Json`/`List<Json>` fields exist (`ck.json` 5901, 5916, 6083, 6126-6139); a non-C
rule's `bindings` could embed a decision-shaped blob in such a field. The checker treats
`Json` as opaque by design (`"Json" => true`); typed construction of C-owned types is
fully covered. Noted in answer to the "alias" concern; no change requested.

## 3. Builder claims challenged
| Claim | Disposition |
|---|---|
| Owner check across every stage/boundary/constructor | Verified from source + recorded CLI/unit outputs |
| `parse_json` used by canonical/evidence/wire/read_spec | Verified (main.rs:58,77; lib.rs:139); corpus/payload paths intentionally stock — no mixed numeric equality (checked) |
| Exact semantics never via f64; `1e400` cannot panic/hang | Verified by trace of numbers.rs; exponent clamp 2^40, no allocation ∝ exponent |
| Grammar/dup-key/depth equal serde_json | Verified by reading + unit/differential tests; only numeric divergences, all intended |
| 9 revision sites bounded | Verified — but mis-described (P3-3) |
| original_position panic removed | Verified (accounting.rs:94) |
| Nano pattern = 0..=i128::MAX, no new limit | Verified digit-by-digit |
| No dependency/feature/lock change | Cargo.toml verified; Cargo.lock not read |
| 19 null acks untouched; 18 digests changed | 19 verified in both trees; digests spot-checked (8 keys) |
| structural_r8: 3 tests pre-failing, fixtures-only repair | Structurally corroborated; assertions unchanged (full file diff read); not executed |
| Residual risk #1 is future-only | Not accurate — reachable now at one free-form `uniqueItems` location (P3-1) |

## 4. Coverage
Read in full: PACKET, RESULT.md, PLAN rows + numeric paragraph, C1-REVIEW C1-R8-01/03,
SPEC-PACKET; subject `numbers.rs`, `main.rs`, `accounting.rs`, `lib.rs` 1-950,
`corpus.rs`, `applicability.rs`, `decisions.rs` 1-120/300-480, `ck.schema.json`,
`wire.schema.json` 20-295, `accounting.schema.json` numeric/`uniqueItems`/required
sections, `ck.json` stages/boundaries/SettlementDecision/schema nodes, `README.md` (both
trees), `tests/structural_r8.rs` (both trees), `c_owner_domains.rs`, `c_owner_cli.py`,
`nano_domain.py`, `numeric_domains.py`, `no_panic_mutations.py`, `Cargo.toml`,
`publish_domains.py`, `replay_gates.sh`; evidence: cargo-test.out, both c_owner_cli
outputs, both numeric_domains outputs (head), no_panic output, gates/results.jsonl and
gate outputs, impact-changed-nodes, digest spot-checks.
Executed: nothing (no shell). Not covered: byte-level `git diff` (replaced by base/subject
reads of every file the builder listed as changed, except `contract/*.md`,
`corpus/SCHEMA.md`, `Cargo.lock`); independent re-run of any gate or counterexample;
jsonschema 0.58.6 internals (relied on recorded tests exercising them); the three
unchanged Python gate scripts' source; full 228-entry digest diff.
```

---

**Verdict: ACCEPT** (architecture: `bounded_repair_credible`; six P3 report-only notes, none blocking). Full report above is the content for `/home/bline/cbm-work/pi-accounting-s1-owner-20261116/slice1/review-spec-r1/REVIEW.md`. This was a same-provider review with no execution capability; all executed evidence is the builder's recorded output cross-checked against source.