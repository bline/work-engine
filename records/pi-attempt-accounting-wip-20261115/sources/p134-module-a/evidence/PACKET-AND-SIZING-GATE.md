# P1.34 preflight, packet derivation and sizing gate (written before launch, 2026-11-13 ~16:57Z)

## Preflight (2026-11-13T16:48–16:57Z)

- **Release:** `current` = `92a03789`, unchanged since 2026-11-12T06:53Z.
- **Producer scripts:** `skills/manual-orchestration/scripts` unchanged from `92a03789` to HEAD `df991c81`.
- **Board** (seqs 5157–5176):
  - other roots active: research-lifecycle M4, org-pi-review-vertical, U1 operator interface;
  - no claim on pi-accounting, S1 or this workspace;
  - the other roots consume Codex subscription quota, so quota readings are account-wide.
- **Frozen revisions:**
  - revision 5 `s1/` manifest `c2017453…`, 350 files, intact;
  - revision 4 `3af24453…` intact;
  - revisions 1–3 (`04f8df60`, `e44b11b0`, `e278f66c`) intact;
  - the P1.33 and P1.32 round manifests are intact, including the sealed P1.32 response `be7ec326…`.
- **Key digests:** CONTRACT `f087d4a2…`, TRANSITIONS `f30a6b23…`, MONEY `83a6b5f8…`, validator `e542e729…`, sabotage
  `b0dbc900…`, all as reported in P1.33.
- **Validator, rerun read-only:** check 173 cases, 0 failures; self-test 34 of 34 (rc 0); sabotage ALL CAUGHT.
  Revision 5 is still intact afterwards.
- **Design-only status:**
  - no crate or Rust source mentions `amount_source`, `ReservationBound` or the contract name;
  - the working-tree changes are other workstreams' (terminal-ui, operator-observation).
- **P1.33 packet:** `a60ce900…`, VERIFY PASS.

## Why a derived packet

The P1.33 packet's brief was blind, but its sources disclosed the earlier review:
- **CONTRACT.md:3–4:** "repairs of the P1.32 Module A findings A-01 to A-09 … exact change log";
- **DECISION-REGISTER.md:126, :135–136:** "P1.32 Module A findings"; "repaired as P2 corrections";
- **INVARIANTS.md:19, :28:** "(P1.32 A-01)";
- **corpus:** `findings` tags such as `P1.32-A-04`, `group` "P1.33", and provenance "P1.33 Module A repair".

The P1.34 instruction forbids revealing the earlier verdict, the dispositions and the implementer's conclusions. The
brief also predates the P1.34 questions.

## Derivation (`packet/build_packet_p134.py`)

**Source:** the 14 frozen P1.33 source files, each checked against the P1.33 manifest digest.

**Declared operations:**
- 5 provenance substitutions, each with an exact count;
- `A-0N` → `R5-0N` (56 labels);
- the corpus fields `findings` and `group` dropped (112 fields);
- 14 provenance origins renamed.

**Asserted invariants:**
- line counts unchanged in every file, so citations map to the frozen files;
- no disclosure pattern remains;
- 6 producer and rate files and MONEY.md are byte-identical.

**Normative equivalence (`normative-equivalence.txt`):**
- 42 changed lines. With the relabel undone, all but 7 equal their source. Those 7 are the 5 declared substitutions
  (one spans two lines).
- In all 56 cases, `inputs`, `expected`, `evaluations` and `historical` are identical.

**Brief (`brief_p134.md`)** adds:
- the three P1.34 judgments;
- the 12 required areas, including service tiers and independence from time placement;
- the three architectural challenges;
- the S2a split (financial core versus later integration);
- minimum revisions;
- a per-finding local/structural class with an overall architecture assessment (the architecture guard);
- 24 derivations: the P1.33 22 plus `VC04-upstream-error-keeps-reservation` and `P133-A06-error-status-null`, for
  status by event type.

No question was narrowed.

**Result:** prompt `c6bf96abc9225b4372a47e15372741a7e8f601377f1d18da5a83605b8087f786`, 506,425 bytes, 14 files, VERIFY
PASS.

## Sizing gate (`sizing.txt`)

- **Calibration:** P1.32 sent 434,602 bytes as 129,094 input tokens (3.367 bytes per token).
- **Input:** 150,428 tokens estimated; 176,765 pessimistic (corpus at 2.5 bytes per token).
- **Context:** Pi model data gives `gpt-6-astra` `contextWindow` 272,000 and `maxTokens` 128,000. The worst case,
  177k + 20k output, leaves about 75k headroom and stays below the 272k long-input price tier.
- **Admission:** subscription mode uses `SubscriptionProxyBudget`, a serial threshold with no reservation
  (`pi_container.py:868–957`). Prompt size cannot cause a refusal before sending.
- **Cost proxy** ($10/M input, $50/M output):

  | Output tokens | Estimated | Pessimistic |
  |---|---|---|
  | 9k | 1.95 | 2.22 |
  | 12k | 2.10 | 2.37 |
  | 16k | 2.30 | 2.57 |
  | 20k | 2.50 | 2.77 |

  If the response omits cache counts, the proxy values all input at the cache-write rate (12.5). The pessimistic
  16k case would then be about 3.01. The stop is a threshold, not a ceiling, for this single request.
- **Duration:** the upstream cutoff observed at about 900 s is reached at about 25k output tokens at P1.32's rate, or
  17k at the slowest observed rate (19 tokens per second). Expected output is 10–16k, bounded by the output contract.
- **Decision:** proceed. The risk of hitting the cutoff is low but not zero.
