## Applicability

`applicable` — checkpoint `6ea42984a87c32084743f6938311f3590ac23e59` contains materially normative agent-facing text in the skill entrypoint, classifier contract, and generated authority metadata. Reviewed under the exact agent-instruction finding contract.

Subject bindings verified:

- Tree: `40ec4057aa66a3682e6b29c97b68ec6b86f95855`
- Binary task-patch SHA-256: `61ed4da2fb8903be89fb8d29f8a24d4078b843ed971ce7279b0c95551f6b1b9a`
- Plan/scope: `profile-recognizability-precheck-v1` / `skills-linguistic-register-pilot-only`
- Commit metadata binds gate receipt `ad64c0b3646851fe0b278420ea16c83cd53e2ca2433ca425897bbd644f642b58`
- All reviewed workspace blobs match the checkpoint.

## Findings

### LRP-001 — HIGH — Unvalidated blinding key can produce a false passing gate

Evidence: the score path checks only the key’s outer fields and plan/packet digests at [recognizability_artifacts.py:478](/tmp/tmp.c8VHBVJjtC/skills/linguistic-register-pilot/scripts/recognizability_artifacts.py:478), then trusts its mapping at [recognizability_artifacts.py:343](/tmp/tmp.c8VHBVJjtC/skills/linguistic-register-pilot/scripts/recognizability_artifacts.py:343). Candidate completeness is calculated over whatever identities remain after inversion, and `all()` drives the gate at [recognizability_artifacts.py:374](/tmp/tmp.c8VHBVJjtC/skills/linguistic-register-pilot/scripts/recognizability_artifacts.py:374). The report also omits the blinding-key digest from its bindings at [recognizability-report.json:4](/tmp/tmp.c8VHBVJjtC/skills/linguistic-register-pilot/pilot/profile-recognizability-precheck/recognizability-report.json:4).

An in-memory probe duplicated one candidate identity in `anonymous_mapping`; every implemented CLI check passed, the report omitted Leveson, and returned:

```json
{"candidate_count":2,"candidate_ids":["gelman-model-criticism","shaw-engineering-judgment"],"gate_passed":true}
```

Consequence: corrupted, incomplete, or swapped unblinding can misattribute scores or pass without all three preregistered candidates, invalidating the gate evidence.

Remediation: deterministically reconstruct the expected packet and key from the frozen plan and require exact equality; validate schema, experiment, bijection, candidate set, neutral identity, profile-digest map, and card correspondence. Bind the blinding-key SHA-256 into the report. Add duplicate, missing, swapped, and packet/key mismatch tests.

### AIR-001 — MEDIUM — Packet authority calls the pre-check a manipulation check

Observation: generation emits `"blinded manipulation-check input"` at [recognizability_artifacts.py:262](/tmp/tmp.c8VHBVJjtC/skills/linguistic-register-pilot/scripts/recognizability_artifacts.py:262), reproduced at [blinded-packet.json:3](/tmp/tmp.c8VHBVJjtC/skills/linguistic-register-pilot/pilot/profile-recognizability-precheck/blinded-packet.json:3). The owning skill explicitly says this is not the rendered C0/C1/C2 manipulation check at [SKILL.md:32](/tmp/tmp.c8VHBVJjtC/skills/linguistic-register-pilot/SKILL.md:32), and the preregistration repeats that boundary at [preregistration.yaml:70](/tmp/tmp.c8VHBVJjtC/skills/linguistic-register-pilot/pilot/profile-recognizability-precheck/preregistration.yaml:70).

- Owner/consumer/reach: pre-check contract; classifier and downstream artifact readers; loaded with every packet.
- Protected distinction: retained-profile recognizability evidence versus section 9.5 rendered-artifact evidence.
- Exact wording is not causally necessary and conflicts with the higher-precedence skill contract.
- Advisory outcome: `restate` as “blinded retained-profile recognizability pre-check input.”
- Confidence: high.

The report and summary themselves are correctly limited; the problem is the packet’s normative authority label.

## Verified properties

- Current packet contains no direct candidate, author, source-title, source-ID, institution, or URL leakage. Preparation copies only retained layer/category/abstract description/weight fields.
- Current raw classifier output normalizes exactly into the committed result; launcher provenance is absent from the raw result and added separately in the normalized artifact.
- All three separability reports, the blinded packet/key, normalized result, and recognizability report reproduce exactly in memory. Current gate arithmetic is correct.
- Limitations clearly disclose same-family classification, constructed neutral decoy, small corpora, and non-equivalence to section 9.5.
- The parent diff contains only the 26 files under `skills/linguistic-register-pilot/**`; `app-server` is untouched.

## Test observations

- Scoped suite discovered 11 tests: 1 passed; 10 could not execute because the enforced read-only environment provides no writable temporary directory. They errored before test logic.
- Existing recognizability tests cover the happy path, pair completeness, and stale profile binding, but not corrupted packet/key mappings.
- The adversarial in-memory probe above confirms LRP-001.

Omissions/limitations: external corpus bytes are intentionally absent, so their content digests were not independently reproduced. The gate receipt’s digest binding was verified, but its underlying receipt was unavailable. Codebase graph coverage calls were approval-blocked; all 26 slice files were instead read directly and matched against immutable Git blobs.

## Verdict

**BLOCK** — LRP-001 permits a mechanically passing but incomplete or misattributed recognizability report.

Provenance: model `gpt-5.6-sol`; reasoning `xhigh`; fresh process; no inherited builder context; same-model relationship; evidence class `accepted_same_model_review`; `independence_claimed: false`.