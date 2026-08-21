# Review Bench Experiment Contract

Status: frozen pilot contract  
Contract version: `evidence-calibrated-review/1.0`  
Frozen: 2026-08-20

## Decision being evaluated

The bench does not ask for one universally best reviewer. It estimates which
reviewer configuration supplies sufficient, evidence-grounded protection for a
class of change at proportional cost.

A reviewer configuration is the complete tuple of:

```text
provider
model
harness
inference family
reasoning effort
review protocol
fresh-context state
tool/evidence access
pass count
aggregation strategy
aggregation member configurations
```

Changing any element creates a different configuration. Results must not be
pooled across configurations merely because the model name is the same.

## Initial routing classes

Cases use one coarse primary route plus multi-label characteristics.

1. `deterministic_local`: local behavior where static source and focused tests
   can establish most acceptance-critical claims.
2. `runtime_cross_boundary`: behavior crossing modules, processes, rendered
   interfaces, or runtime state owners.
3. `persistent_state_identity_integrity`: persistence, restoration, identity,
   mutation ordering, durable history, or corruption-sensitive behavior.
4. `novel_architecture_high_consequence`: new ownership or placement decisions,
   security-sensitive behavior, irreversible behavior, or unusually broad
   architectural consequences.

Sparse classes inherit the safer parent route. The taxonomy may be revised
only by a new contract version; existing results retain their original labels.

## Controlled arms

The pilot should include, where available:

- Claude in the established CLI review harness;
- one fresh Sol review in the Codex harness;
- repeated Sol reviews in the same Codex harness;
- Sol in the web attachment harness; and
- grounded aggregation of independently produced findings.

Change one experimental variable at a time. A reasoning-effort comparison must
hold model, harness, snapshot, prompt, tools, pass count, and aggregation fixed.
A harness comparison must hold model family, snapshot, prompt, and nominal
reasoning effort fixed as closely as the products permit. Record uncontrolled
differences as limitations.

## Case admission and truth sealing

- Qualification cases require an immutable snapshot or a reconstruction whose
  digest and limitations have been independently verified.
- Historical forensic cases may calibrate the harness and discover taxonomy,
  but cannot determine production routing policy until promoted through
  reconstruction and adjudication.
- Truth is sealed before new reviewer results are inspected.
- Prior reviewer findings, builder rationale, and truth identifiers remain
  absent from blinded packets.
- Review reconstructed checkpoints from the exported archive in an isolated
  workspace. Do not give a reviewer access to the source repository's Git
  history when a synthetic checkpoint parent or neighboring commit could reveal
  the later repair.
- Clean cases are required to measure false blocking and plausible evidence
  theater.

## Review protocol

Reviewers act as read-only independent verification reviewers. Material claims
are hypotheses until supported by direct artifact, executable, runtime, or
persisted-state evidence. For each acceptance-critical conclusion, reviewers
perform a bounded search for the most plausible contradictory path. They stop
when the material claim reaches high confidence or honestly return
`blocked_unverified`; they do not recursively prove ambient language,
operating-system, or standard-library behavior.

Every retained finding must identify:

- the violated contract or invariant;
- concrete snapshot evidence;
- a plausible failure mechanism;
- relevant confirming test or runtime evidence, when available; and
- the material contradictory path checked.

An aggregation pass is adjudicative rather than additive. It must ground and
filter findings; raw union is not an acceptance mechanism.

## Primary measurements

- defect recall and acceptance-critical blocking recall;
- finding precision and evidence validity;
- false acceptance, false blocking, and unverified blocking;
- pairwise joint miss rate and union recall;
- conditional recall of each reviewer given the other reviewer's misses;
- findings unique to each configuration;
- marginal true detections relative to additional false positives;
- wall-clock time, cost, input tokens, and output tokens; and
- cost, time, and token use per adjudicated truth finding when resource
  observations are complete; and
- the same metrics stratified by route class.

All pilot results are descriptive. Repeated attempts on one case are correlated,
and small per-class samples do not justify equivalence claims or automatic
routing changes.

## Research basis and limits

- SWRBench supports real-PR evaluation, structured issue matching, and both
  same-model and cross-model multi-review experiments. Its largest reported
  relative F1 gain came from repeated same-model review and aggregation, not
  proof that provider diversity is intrinsically superior:
  <https://arxiv.org/abs/2509.01494>.
- The 2026 comparative evaluation reports a severe synthetic-to-real gap and a
  cross-model ensemble that added noise rather than useful recall, so diversity
  remains an empirical configuration property:
  <https://arxiv.org/abs/2606.15689>.
- ContextCRBench supports including task/contract context rather than supplying
  only a diff: <https://arxiv.org/abs/2511.07017>.
- CRScore motivates explicit claim/evidence adjudication instead of lexical
  similarity: <https://aclanthology.org/2025.naacl-long.457/>.
- ReviewGrounder supports separating review drafting from evidence grounding:
  <https://aclanthology.org/2026.acl-long.1477/>.

The controlled same-model/same-harness versus same-model/different-harness
versus different-provider comparison remains the Work Engine-specific question.
