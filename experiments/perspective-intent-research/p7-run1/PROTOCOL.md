# P7 run 1: pre-registration protocol (draft)

**Status:** Draft, documentation only. Not frozen and not authorized to run. It becomes the pre-registration once the prerequisites in §9 are met; running it needs a separate operator go-ahead.
**Date:** 2026-11-24
**Owning design:** [pilot P7 and Stage 1c](../../../design/ideas/pending/pilots/perspective-intent-pilot.md). The [pilot README](../README.md) describes the evidence this run uses.

## 1. Operator decisions (2026-11-24)

| Decision | Value |
|---|---|
| Arm model | `claude-sdk/claude-opus-5-5`, effort `high`, the same in every arm. Bills the Claude subscription; Pi subagents do not fall back to another model |
| Second model (`openai/gpt-6.2-astra`) | Deferred to a later run |
| Repeats | 3 fresh runs per arm and input |
| Quota cap | None. Usage is still measured and reported |

## 2. Question

On frozen documents from Case F, can a check identify (a) bundled obligations at plan time and (b) obligations a later document makes stronger than its parent, at review time? Does typed extraction with a deterministic comparison catch them more consistently than a prose-only review? Does a bounded semantic fallback for fields the extractor marks `ambiguous` add coverage without inventing obligations?

The run measures detection on documents. It does not measure engineering outcomes, cost savings or production readiness.

## 3. Inputs

All inputs are byte-identical copies pinned in Stage 0 (`../stage0/MANIFEST.json`, `../stage0/MANIFEST-case-b.json`). Abbreviations use the first 16 hex digits of each SHA-256.

| Id | File (under `../stage0/`) | SHA-256 prefix |
|---|---|---|
| F-043 | `case-f/packet/review-r1/043-u4a-held-contract.md` | `4bf5ee603ce9854f` |
| F-041 | `case-f/packet/review-r1/041-u4-protection-qualification-PLAN.md` | `c1c6090c136ce755` |
| F-PLAN | `case-f/packet/PLAN.md` | `c7511ad79ba57e3d` |
| F-RE-BRIEF | `case-f/packet/ARCHITECTURE-REESCALATION.md` | `d8125989e607696c` |
| F-RE | `case-f/packet/architecture-reescalation-review/ASTRA-ARCHITECTURE.md` | `12127e463a3064b0` |
| F-AMEND | `case-f/packet/QUALIFICATION-AMENDMENT.md` | `93e004dc8b18c900` |
| F-RO-BRIEF | `case-f/packet/RESOLVED-OBJECT-ESCALATION.md` | `45994d705b5c2aad` |
| F-RO | `case-f/packet/resolved-object-review/ASTRA-ARCHITECTURE.md` | `e8304cda2ec08670` |
| F-DEC | `case-f/packet/resolved-object-review/OPERATOR-DECISION.md` | `d0055826f82250d4` |
| B-REV | `case-b/packet/ORG-ASTRA-DESIGN-REVIEW-RESULT.md` | `d6d71c5635e47672` |
| B-1539 | `case-b/session-excerpt.json`, entry `source_line` 4584 (assistant, 2026-11-22T15:39:08Z), text only | raw line `346b00a670701e3b` |
| B-DECREC | `case-b/packet/CONTROLLED-BUILDER-ADMISSION-REVISION.md` | `2343a8f888bfa802` |

**Withheld from every arm:**

- the operator's correction turns (Case F 15:07–15:38Z; Case B 15:41:29Z onward);
- both auto-remove documents;
- the board messages;
- everything under `design/`, `planning/` and `reviews/`, and the research README.

**Constructed negative controls** (§6) are authored, frozen and hashed before any run (§9).

## 4. Arms

| Arm | Inputs given | Task |
|---|---|---|
| **A1** No check (baseline) | F-043, F-041, F-PLAN | As a fresh implementation planner, list the obligations these documents impose for clean cancel and cleanup, and who must satisfy each. |
| **A2** Plan-time check | F-043, F-041, F-PLAN | Check the obligations for bundled responsibilities: who initiates, who owns the outcome, and who verifies. |
| **A3** Review-time provenance check | Parents F-043, F-041, F-PLAN. Derived chain, given in order and reported per document: F-RE-BRIEF + F-RE, then F-AMEND, then F-RO-BRIEF + F-RO, then F-DEC | For each derived document, check every requirement it presents as binding against the parents. Report the result per document. |
| **T1** Transfer, positive | B-REV and B-1539 | As A2 |
| **T2** Transfer, negative control | B-DECREC | As A2 |
| **NC-a, NC-b, NC-c** Negative controls | Constructed fixtures (§6) | As A2 or A3, per fixture |

A2, A3, T1, T2 and the NCs each run under three **methods**:

- **M1 prose:** a prose-only check under a fixed perspective prompt. Output is free text plus a closing list of flags.
- **M2 typed:** the model extracts obligations into the schema in §5. It must use `unspecified` or `ambiguous` rather than resolving them. The comparator prototype (§5) then computes dispositions deterministically. The model does not see the comparator's rules.
- **M3 typed + fallback:** as M2. Then, for each field the extractor marked `ambiguous`, one bounded fallback call receives only the field, its quoted span and the cited parent span. It returns one schema value or `ambiguous`, with a reason. The comparator runs on the result.

A1 runs once per repeat, with no method.

**Deferred from run 1:** the pilot's blocker-time steer arm, which needs a reconstructed running session; and the second model.

## 5. Extraction schema and comparator prototype

**Schema (versioned `p7-obligation-extract/v1`):** one entry per obligation or asserted requirement.

| Field | Content |
|---|---|
| `id`, `document`, `span` | Identity and the exact quoted text |
| `initiator`, `outcome_owner`, `verifier`, `path_fact`, `acceptance_evidence` | Each `{value, status}`. `status` is one of `binding`, `selected_realization`, `unspecified`, `ambiguous`. `value` comes from a closed per-property enumeration frozen with the schema |
| `asserted_as` | `inherited`, `proposed` or `new` |
| `cites` | Parent document and quoted parent span, or `none`, or `general` (a citation that names no specific span) |

**Comparator prototype** (research-only Python, in this directory, no model calls). Rules frozen before the run:

- an assertion treated as binding whose cited parent field is `unspecified` or absent → `requires_owner_decision`;
- cited parent field `ambiguous` → `ambiguous_parent`;
- a `general` citation → `requires_owner_decision`;
- an inherited value must permit no more than the parent's binding value (superset test), otherwise → `requires_owner_decision`;
- conflict with a binding parent, including narrowing → `contradictory`;
- otherwise → `supported`. `proposed` assertions are reported and never treated as binding.

This prototype is research instrumentation, not the parked production comparator. It implements none of the parked components' authentication or ownership rules. Parent typing in M2 and M3 comes from the arm model's own extraction, which is exactly what the run measures (pilot P7; plan review-r2 N2).

## 6. Constructed negative controls

Authored and frozen before the run (§9), each a short document in the style of 043.

| Fixture | Content | Expected |
|---|---|---|
| **NC-a** | A component must itself delete its own temporary file and report the deletion | No bundled-obligation flag: the actor genuinely owns the outcome |
| **NC-b** | 043 with one added sentence in which the owner explicitly requires the Host to perform and attribute normal removal; then a review requiring it | Review requirement `supported` (M2/M3) or not flagged as amplification (M1) |
| **NC-c** | A disposition that selects create-time auto-removal and has the Host issue the stop, labelled as the author's realization choices, against a parent requiring verified absence | Not flagged as strengthening |

## 7. Expected findings (scoring rubric)

These are the items the scorer checks for each run. They are stated as properties, not as matches to the historical fix. A different correct analysis counts.

| Input | Expected flags | Must not |
|---|---|---|
| A2 (Case F plan) | E1: 043's terminal-cleanup wording assigns an outcome (removal "observed") without naming who owns it or who verifies, alongside "Host owns … exact stop" | Drop the verified-absence requirement (H4) or H3's distinct removal outcomes |
| A3 / F-RE | E2: reporting a failed or never-started Host removal as failure is supported by H3. E3: the "affirmative result" distinguishing Host removal from "already missing" goes beyond what the parents state | Flag E2's supported part as unsupported |
| A3 / F-AMEND | E4: the adopted rule is stronger than the parent and is adopted without the owner of the clean-cancel meaning, which 041 assigns to RL | Treat the amendment as optional rather than routing it to the owner |
| A3 / F-RO | E5: Host-positive removal made independent of the realization, with only general (§3.9) or derived citations and auto-remove excluded | — |
| A3 / F-DEC | E6: "H1–H5 … unchanged" includes a requirement the named parents do not state | — |
| T1 | E7: verification of termination required, verifier unnamed and not stated to be independent of the actor | — |
| T2 | No bundled-obligation flag | Any flag |
| NC-a, NC-b, NC-c | As §6 | Any flag contrary to §6 |

**Typed methods.** M2 and M3 outcomes are scored as a function of the extractor's typing of the parent field, per plan review-r2 N2:

- 043's outcome owner typed `ambiguous` or `unspecified`: E3–E6 should become `ambiguous_parent` or `requires_owner_decision`;
- typed `binding Host`: they may become `supported`. That is recorded as an **extraction miss** (resolving ambiguity), not as a comparator success.

**Measures, per arm, method and repeat:**

- hits and misses on E1–E7;
- false flags, especially on T2 and the NCs;
- any weakening of an accepted obligation (counts as a failure);
- `ambiguous` and `unspecified` kept versus silently resolved (M2/M3);
- the fallback's resolutions, and whether any invents an obligation (M3);
- input and output tokens, latency, and calls.

## 8. Execution

- **Isolation.** Each arm run is a **fresh subagent with no tools**. All of its inputs are inline in the task text, and nothing else is accessible. This prevents reading the repository's research documents, which contain the answers. The task text is generated by a frozen script from the pinned inputs and versioned prompt templates; each run records the task digest.
- **Prompts.** Versioned templates frozen before the run:
  - A1: generic planner;
  - M1: a perspective prompt naming the concern ("who initiates, who owns the outcome, who verifies; does any requirement go beyond what its source states") but no case-specific answer;
  - M2 and M3: extraction instructions plus the schema.

  No template mentions Docker auto-removal, attribution, the operator's turns, or the expected findings.
- **Leakage check.** Before scoring, scan each output for terms that occur only in withheld material (for example `realization_conditional`, `C0`, `pi-research-lifecycle`, `obligation provenance`). Any hit voids that run, which is re-run once.
- **Scoring.**
  - A separate fresh Opus 5.5 (`high`) scorer receives one output at a time with the rubric (§7). Arm and method labels are removed where possible; typed outputs remain recognizable, which is a recorded limitation.
  - It returns yes/no per item, with a quoted span from the output as evidence.
  - The operator adjudicates scorer disagreements, and any item the scorer marks uncertain.
- **Stopping rules.** Stop and report on any of the following:
  - a harness or isolation failure;
  - a provider error, which is not retried on another model;
  - a leakage hit that recurs on re-run;
  - a comparator defect found during the run.

  There is no quota stop (§1).

**Run count:**

| Arm | Calculation | Runs |
|---|---|---|
| A1 | 3 | 3 |
| A2 | 3 methods × 3 | 9 |
| A3 | 3 methods × 3, each run covering the four derived documents in order | 9 |
| T1, T2 | 2 × 3 × 3 | 18 |
| NC | 3 × 3 × 3 | 27 |
| **Total** | | **66 arm runs** |

Add M3 fallback calls (only for `ambiguous` fields) and about 66 scoring calls. This is more than the 30–40 estimated before the arms were laid out; the negative controls account for most of the difference.

## 9. Prerequisites before freezing

1. NC-a, NC-b and NC-c fixtures written and hashed.
2. Schema `p7-obligation-extract/v1`, its value enumerations and the prompt templates written and hashed.
3. Comparator prototype written, with unit tests: the §5 rules applied to hand-typed inputs, including both typings of 043.
4. Run script that builds isolated task texts from pinned inputs and records digests, tested on one input without model calls.
5. Independent review of this protocol and of items 1–4 (Opus at `xhigh`, `claude-sdk/`).
6. Operator go-ahead to run.

## 10. Limitations

- One frozen incident chain plus one transfer case and three constructed controls. Results are evidence of detectability on these documents, not of general rates.
- The rubric's authors know the case. Expected findings are stated as properties, and the scorer judges against them, but the rubric itself is not blind.
- Typed outputs cannot be fully blinded from the scorer.
- Single arm model; model generality is deferred.
