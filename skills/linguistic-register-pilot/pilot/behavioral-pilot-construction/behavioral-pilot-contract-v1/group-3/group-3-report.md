# Behavioral pilot construction — implementation group 3

Group 3 reached a truthful preoutcome stop. Six fresh, opaque full-role
renderings were generated and bound, but the frozen gate did not pass. No
behavioral task was presented to the subject and no outcome or score was
created.

## Rendering receipt

- Group-3 ID: `linguistic-register-behavioral-pilot-group-3-v1-2026-08-26`
- Six render jobs completed under OpenAI `gpt-5.6-sol`, medium reasoning,
  Codex CLI `0.149.1`.
- Each job used a fresh ephemeral process and a condition-independent opaque
  UUID. The mapping remains in the separately bound `condition-key.json`.
- All six raw results satisfied the exact single-key `text` wrapper contract.
- Normalized prose lengths were 738–780 words.
- Raw responses, events, attempt markers, and receipts remain outside Git;
  `evidence-summary.json` binds their digests and provenance.

The first batch shell launched only one job because that child inherited and
consumed the loop's stdin. The other five had no marker or receipt and were
therefore mechanically distinguishable as unattempted. Exactly those five
immutable jobs were then launched with stdin detached. Nothing was retried or
substituted.

Before packet construction, the integration context's broad contract search
returned task/key file paths and one isolated repeated key sentence. No task or
key file was opened, and none of that material entered a render packet. Each
renderer saw only the frozen canonical role and its licensed realization
contract in an isolated temporary process. Renderer-level task/key blindness is
therefore directly supported; complete builder-context non-exposure is not
claimed.

## Review transport and publication history

The first Claude review launch was rejected locally because Claude's schema
surface did not accept the declared 2020-12 dialect. A fresh draft-07 manifest
was frozen. Its first attempt was also rejected before inference because bare
mode disables OAuth/keychain authentication. A bounded safe-mode manifest then
executed successfully.

That review correctly found patch-boundary contamination in three publication
copies. The original raw render evidence was clean; repository publication had
concatenated patch payloads. The contaminated packets and reviews remain
preserved. Publication copies were reconstructed from the retained raw strings
and a wholly fresh v4 blinded packet was sealed. Four strings without a
terminal newline acquired an incidental terminal `+` during repository
publication. The v4 reviewers observed and excluded that cue, and the final
gate qualifies the overstated LF-only normalization sentence in
`rendering-manifest-v2.json`. No role was rerendered. The v1 cue/gate files are
retained separately from the corrected terminal-plus qualification.

## Frozen gate results

Canonical coverage and advisory speech-act equivalence passed for all six.
The different-provider semantic reviewer accepted semantic equivalence and
salience control for only the two neutral artifacts. Four treated artifacts
introduced or heightened wording strongly enough to fail the all-artifact
gate. No forbidden identity, domain, task, source, or profile leakage was found
in the standalone v4 artifacts; the terminal plus was retained as incidental
publication metadata rather than treatment evidence.

The actual-artifact manipulation gate also failed:

- Sol: 4/6 exact assignments;
- Claude: 2/6 exact assignments;
- both perspectives separated treated surface realization from neutral prose;
- neither perspective established the preregistered C2 > C1 discourse uptake.

The shallow baseline found six perfect one-scalar condition rules, involving
colon, line, paragraph, question-mark, and semicolon counts. This independently
fails cue concentration.

The source audit completed all 48 render/source comparisons. Maximum direct
exact overlap was five normalized tokens, with zero material exact and zero
near-verbatim matches. Nine comparisons crossed the frozen five-token
profile-mediated threshold, so the profile-mediated quotation gate fails even
though direct quotation checks are clean.

Local subject fields pass: Codex CLI `0.149.1` and the frozen model, reasoning,
service, and no-tools contract are recorded. Effective model/backend receipt,
exposed sampling identifiers, usage meter, and fresh-context/no-tools proof
remain mandatory runtime checks before T001 and were not invented.

## Disposition

`overall_gate: false`. The root `group-3-preoutcome-gate.json` is consumable by
`behavioral_pilot_contract.py`, whose launch check must refuse calibration.
Human audit remains unresolved; model reviews are evidence only.

The next scientifically coherent action is a separately authorized rendering
repair experiment. It should reduce semantic drift and condition-level shallow
structure, then regenerate all six artifacts under a fresh preregistration.
The current outputs must not be repaired in place or carried into T001.
