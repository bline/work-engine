# M4 S2: H2 decision packet and the concrete execution artifacts

2026-11-17, revision 1. **This is a planning candidate for root review. It implements
nothing, selects no native build, installs nothing, runs no inference and grants no
authority.** Packet:
`/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/006-s2-plan/PACKET.md`.
Root and collector: Pi session `01a1e56d-7ceb-71fc-9c46-c9fcd9618234`. Planner: Pi
session `01a1e71f-8e21-724a-b6e9-94cdbd4892f7`. Shared branch HEAD when read:
`f596f4eb`. S1 standing: `O/ROOT-ACCEPTANCE.json` (`O` =
`/home/bline/cbm-work/research-lifecycle-tool-registry-20261116`).

**Short answer.** No engineering unit can be done offline now that moves M4 much
closer to a real authorized H2 execution. Every remaining gate on the path is waiting
for an operator or owner decision first, or it is not on that path at all. Section 3 is
the decision packet. Section 4 plans one small offline unit that does not wait for those
decisions. It produces the exact draft bytes the operator would be approving (audit
request, snapshot, rubric, paired protocol, grant and installation skeletons) and a
source-verified trace of every production refusal between today and one real H2. The
operator cannot decide H2 authority today partly because none of these bytes exist.

A second fact matters for the operator. **Under the current full protection claim, real
H2 cannot be reached within the remaining ~$64.** Section 2 explains why. Getting there
means either more budget over several slices, or an explicit operator decision to narrow
the claim for this one use (D1).

## 1. Where M4 stands

The M4 exit is one actually authorized bounded execution, with its basis and the
required verification
([roadmap M4](research-lifecycle-rust-implementation-plan-20261111.md)). The first use
is still the bounded repository audit of `context-boundary.h2-levels`. An advisory A5
task (proposed locator `research-lifecycle-h2-advice-1`) comes first, then the audit
task `research-lifecycle-h2-audit-1`. M0 LC-7 also requires a paired exploratory
baseline to be frozen before either arm runs
([admission packet](research-lifecycle-m4-admission-and-a5-packet-20261112.md) §§3, 7;
[M0](research-lifecycle-m0-contract-and-first-use-20261111.md) LC-6, LC-7).

These parts are accepted at their bounded private scopes: M4b preparation and adoption,
Q10/Q11 startup and restart qualification, the 008 strict transport-input producer (the
producer half of C1), and S1. S1 delivers per-step router snapshots and dispatch
citation, with refusal of use, in private native build `77fbfa4b…` (patched official
`83086188`).

These production gates still refuse. Section 4's trace re-verifies each one.

| Gate | Current refusal | What would release it |
| --- | --- | --- |
| Selected native startup | `lifecycle-service/src/native_startup.rs:121–125` `publish` returns `Unavailable` unless both hold and observation are controlled ("no qualified production contract exists for this build") | A qualified tool-installation semantics (S1 evidence exists for the private build only), qualified protected-surface observation (004 route), and a selected native build (D3) |
| A5 delivery (C1) | The strict producer exists only in the private build. Production consumption needs that build selected | D3, then production use of the existing `compiled_input` / fence chain |
| Research admission | Default construction is unavailable (`Installation::synthetic` is feature-gated). M4c-b installer result was "changes required", and no manifest has been approved | O1 channel rebinding or reuse (D8), exact approved manifests |
| Grants | `grant_for_context` selects only trusted configured grants. None exist for the H2 tasks | Operator grant decisions (D8) bound to exact task, scope and budget |
| Live inference | No API allowance. Codex quota is exhausted until 2026-11-21. The installer channel names a Codex conversation | D6, D8, D9 |

## 2. Candidate units compared

Costs are API-equivalent dollars. They are rough, and none of them is authorized here.

| Candidate | Consequence it closes | Dependencies | Decision needed before engineering | Rough cost |
| --- | --- | --- | --- | --- |
| **A. Protection/access-profile evidence** (overnight 004 route: per-FD kernel witness, closed topology, selected proxy boundary) | Protected set, sockets, proxy and descendants: the protection half of the startup refusal | Four owner prerequisites (004 §7). 005 showed only a plain-child pidfd case; bwrap rejected `--preserve-fds`. The actual selected containment is bwrap plus a Unix proxy (`isolation.rs`), not the Docker confinement S1 used | **Yes:** D2 (FD wording) and D6 (credential-free readiness). Helper/proxy provenance is an owner input | 004's conditional estimate is 11–22 proxy units *after* the prerequisites, and unknown before them. More than the remaining budget allows safely |
| A′. Kernel-witness feasibility probe only (004 prerequisite 2, under the actual bwrap containment) | Shows whether observation is even permitted on this host | Actual isolation launch | Formally no. In practice it is wasted if D2 or D6 rule the profile out | $6–10 |
| **B. Lawful initial rehydration and early native ingress** | Original repair qualification row | 006 found no lawful pre-Bind owner route; one would be a new owner mutation route | **Yes:** a new route is an architecture decision, or D7 excludes it from the first use | $15–30, uncertain |
| **C. Substitution and ticket experiments** | Original repair qualification row | Needs an admissible owner-local realization | **Yes:** an owner disposition (D7) | $10–20 |
| **D. Harden the seven native utilities** | Narrows the protected-use tool surface (S1 open item) | Whether the config can disable them is unverified. If not, it needs a native patch and rebuild (~20 min) | **Yes:** D5 (approve or disable), which depends on the protection claim (D1, D2) | $4–8 if config only; $12–18 with a native patch |
| F. Production-candidate startup composition (feed S1 and 008 evidence into `native_startup` as typed prerequisites, protection still unavailable) | Turns the tool-installation half of the refusal into a checked predicate | Private build selected | **Yes:** D3. Without it, this is speculative scaffolding | $15–25 |
| **E. H2 execution artifacts and production-gate trace (recommended S2)** | "No exact completed I/O/S/selection approval or authorization supplied" (M4 completion table): gives the operator exact bytes to decide on, and a verified list of what still refuses | None | **No** | expected $8–11, cap $15 |

B and C are not on the path to one fresh first-use execution if the first use runs
without restart and the existing restart and rehydrating refusals stay in place. Whether
that is acceptable is D7. A, D and F depend on decisions that could make them
unnecessary or different. E is useful whichever way D1–D9 go, because every route ends
at the operator approving exact bytes.

**Why real H2 does not fit in ~$64 under the full claim.** The minimum chain is: A
(prerequisites plus 11–22 units), F ($15–25), production C1 consumption, an approved O1
installation, and then three live runs (advisory, audit, LC-7 baseline) plus
verification. Live inference has no allowance today. Even the low end of A plus F, with
review, uses most of the budget before any live run. This is an estimate from recorded
plans, not a proof.

## 3. Decision packet for the operator

Each decision lists its options and their consequences. The planner recommends no
option where the choice is the operator's preference or authority. Decisions marked
**gates S3** need an answer before any further engineering slice. None of them gates S2.

**D1. Route and claim scope for the first real H2** (gates S3). This is the main
decision.
- (a) *Full original claim.* Real H2 waits until the protected-surface route (A), a
  production native selection (D3), production C1 consumption and an approved O1
  installation are each qualified. Consequence: several slices beyond the current $100
  objective, with uncertain feasibility (004 prerequisites, credential-free readiness).
- (b) *Operator-scoped first use.* For this one analytical audit, the operator accepts a
  named narrower protection claim. The repair plan §4.1 and M4c-a §3.3 conditions would
  be met by the selected bwrap and proxy isolation's existing checks plus S1 refusal of
  use plus 008 capture. Socket protected-surface coverage would be recorded as an
  unestablished exclusion, next to the already recorded same-UID exclusion. This is a
  contract change for that use. It needs the operator's explicit authority; a model may
  not make it. Consequence: the path becomes shorter and the guarantee weaker. The
  exclusion must travel with every receipt and must not be generalized.
- (c) *Close M4 as an unmet condition.* The roadmap allows this: "report the unmet
  condition rather than widening this into a runtime rewrite". Consequence: M4 closes
  without execution, and M5 is not reachable.

**D2. Wording of the capability-FD condition** (004 prerequisite 1; gates A). Does
"No capability-bearing FD or activation channel crosses into the native namespace/bridge"
(repair plan §4.1) forbid the declared proxy bridge?
- *Intended meaning is protected capabilities only.* The proxy is an explicit crossing
  with its own witness. Protected FDs and activation channels stay excluded.
- *Literal meaning.* The selected profile cannot satisfy it, so H2 needs a different
  access profile.

**D3. Production selection of a modified native build** (gates F and any production
C1). C1 and the tool-registry evidence exist only in private build `77fbfa4b…`.
Options:
- Approve the private build, or a successor, as the production candidate for this first
  use. Its provenance would be local build custody, not official attestation, and the
  operator owns the fork's upkeep.
- Require an upstream or official build that carries equivalent capabilities. This has
  no timeline.
- Decline. Then C1 stays unavailable and real H2 is impossible through this host (D1(c)).

**D4. Refusal of use versus pre-execution prevention for unlisted tools.** S1 makes a
turn with unexpected tools unusable, but an unlisted tool could still run before the
refusal. Options:
- Accept refusal of use for an analytical audit with no shell, network or write tools.
- Require native allowlist enforcement. That is a change to native trust semantics: a
  new native slice and an owner decision.

**D5. The seven approved native utilities** (`list_mcp_resources`,
`list_mcp_resource_templates`, `read_mcp_resource`, `request_user_input`,
`request_user_input_async`, `clock.curr_time`, `clock.sleep`). Options: approve them for
protected use as they are, or disable them. `request_user_input` routes to the
app-server client. S2 records read-only how Work Engine's client handles it and whether
a config knob disables each utility. The protection owner decides.

**D6. Readiness qualification and provider for live runs** (004 prerequisite 4). The
choices:
- Either define a credential-free, provider-disabled readiness fixture that is
  equivalent to the actual profile, or permit a real provider lease for readiness.
- Name the provider, model, effort and tier for the advisory, audit and baseline arms.
  Codex quota is unavailable until 2026-11-21. The S1 fixtures' requests named
  `gpt-6-sol`; S2 confirms which provider the selected profile actually binds.

**D7. Scope of the original repair before the first H2.** Options:
- Rehydration, early ingress and substitution/ticket experiments must close first.
- The first use runs from a fresh start only, with the existing restart and rehydrating
  refusals in place. Those rows stay open for whole-repair acceptance, which remains
  separate.

**D8. Authority for the H2 tasks** (needed before any live run).
- O1 channel: the installer channel record names "this Codex conversation" as operator
  installer. Rebind it to a named Pi or Claude root, or wait for Codex.
- Research owner: qualification of the `context-boundary.h2-levels` research basis.
- Decision owner: applicability of the exact audit use.
- Execution and inference grants for both tasks.
- Acceptance of LC-7's limited baseline estimand ("the user must accept…").
- Who verifies and accepts the audit result.

Each of these binds to exact bytes that S2 drafts. None can be approved before those
bytes exist.

**D9. Budget.** Live inference spend for three runs plus verification (the API
allowance is zero today), and whether the $100 objective is extended for D1(a).

## 4. Recommended S2: H2 execution artifacts and production-gate trace

### 4.1 Claim and consumer

**Claim.** At a named shared-branch revision, the S2 package (a) lists every production
refusal between the current accepted state and one real H2 execution, each with a
source locus verified at that revision, its current predicate, the evidence that would
release it, its owner and the decision (D1–D9) it waits on; and (b) supplies draft bytes
for every artifact the governing contracts require an operator or owner to approve for
the first use. Each artifact field is either exact (content plus SHA-256) or an explicit
named slot for a decision. Nothing in the package is an approval, grant, installation
or attestation, and each artifact says so.

**Consumer.** The operator, deciding D1–D9 and later approving exact manifests through
the O1 channel. Root uses the gate trace to choose S3. Required field sets come from:
LC-6's "a future execution packet must supply…" list; admission packet §3's record
table (delivery, recommendation, prepared basis, adoption); M4c-a §3.2's `I` / `R_I` /
`C` / `O` / `R_O` records; and LC-7's frozen fields.

### 4.2 Alternatives

| Route | Why not chosen |
| --- | --- |
| Any engineering row (A–D, F) now | Section 2: each waits on a decision or is off the path |
| The decision packet alone (section 3), no artifacts | The operator's authority decisions (D8) bind to exact bytes. Without them D8 cannot be answered, and decisions D1–D7 have no concrete first-use subject |
| Produce real installations or grants in configured paths | Outside authority. It would also look like an approval |
| Project the real A4 card with the production constructor | Unavailable by design. A synthetic-feature preview is allowed only through an existing test or CLI path with no code change, and must be labelled controlled (§4.3 item 6) |

### 4.3 Ownership and outputs

One worker owns:

- the new external directory `O/007-h2-artifacts/`;
- one new planning document,
  `planning/research-lifecycle-h2-execution-packet-draft-20261117.md`, which is the
  index and readable summary.

It writes nothing else in the repository. It must not touch `metrics/register/*`,
configured grant or installation locations, or the shared branch's uncommitted work.
It commits nothing; root decides whether to commit.

Artifacts, all marked `DRAFT — not approved`:

1. `gate-trace.md`: one row per production refusal on the path, re-verified at the
   named HEAD and, for the native side, at the S1 private trees. It also covers
   `request_user_input` handling and config knobs for the seven utilities, read only (D5).
2. `audit-request.json`: objective, engineering question, target decision with exact
   register revisions and digests from `metrics/register/decisions.json` and
   `applicability.jsonl`, scope and output limits, prohibited actions (LC-6).
3. `snapshot-manifest.json`: commit plus path list plus SHA-256 for the approved audit
   snapshot, and the reasons for the selection. Ordinary-policy baseline contents are
   recorded separately.
4. `rubric.md`: the acceptance rubric (supported / unsupported / unknown parts, cited
   evidence, proposed wording or justified no change, further measurement), the
   verifier role and the blinding limits.
5. `lc7-protocol.json`: common objective, snapshot, model/effort/tier slots (D6),
   budgets, tools, exposure manifest, contamination statement, cost-allocation rule,
   and treatment of refusal or failure as outcomes.
6. `advisory-input.md`: the exact list of bounded canonical material the A5 task
   receives, and the delivery observation (008 strict capture and the `compiled_input`
   chain). Optionally a synthetic-labelled A4 card preview, only through an existing
   tested path.
7. `authority-skeletons.json`: `I`, `C` and `O` field skeletons and grant scopes, with
   every authority-bearing field left as a named D8 slot. No receipt, signature or
   approval-looking content.
8. `completeness.md`: every required field from the §4.1 contract sources mapped to an
   artifact field or a named slot.
9. `check.py`: recomputes every SHA-256 in the artifacts from the repository at the
   named commit (`git show <commit>:<path>`), and checks that every slot names an
   existing decision ID. It runs under `custody.py run`.

### 4.4 Decisive signal and controls

**Decisive signal.** `check.py` passes under custody, and a separate reviewer confirms
four things: every required field in `completeness.md` is satisfied or slotted; a
sample of at least eight gate-trace loci, chosen by the reviewer, matches the source; no
artifact presents approval or authority; and no D-decision is silently pre-decided.

Controls:

| Control | Expected |
| --- | --- |
| Corrupt one listed path's digest in a scratch copy | `check.py` fails, naming the path |
| A slot naming a nonexistent decision ID (scratch copy) | `check.py` fails |
| Reviewer's own gate-locus sample, including at least one native-tree locus | All match, or each mismatch is reported |
| Omission probe: reviewer removes one field from a copy of `completeness.md` and checks that its source contract requires it | The source contract does require it |
| `git status --porcelain` before and after; `metrics/register/*` digests | Only the new planning document is added; register unchanged; nothing staged |

### 4.5 Reuse

- Register records and the codecs in `research-register` (read only).
- S1 and 008 private trees for native loci (read only).
- `custody.py` for `check.py` runs.
- The gate table in section 1 is the starting list, to be verified rather than trusted.

No build, native run, inference or credentials.

### 4.6 Validation and review

The worker runs `check.py` and the two scratch negative controls under custody, and
records the context reading at its phase boundaries. Fable at xhigh then reviews,
same-provider and read-only, against the four points in §4.4. Root collects.

Model: worker `claude-sdk/claude-sonnet-5-5` at `high` through Pi, matching S1's builder
for a fully specified packet. If root judges source-tracing precision more important,
Opus at medium is an option.

### 4.7 Budget and stop conditions

Remaining overall budget is about $64.

| Part | Expected | Cap |
| --- | --- | --- |
| Worker (trace, artifacts, check) | 4–6 | 8 |
| Separate review (Fable xhigh) | 2–3 | 4 |
| Repair and recheck | 1 | 2 |
| Root collection | 1 | 1 |
| **Total** | **8–11** | **15** |

This leaves at least about $49 for S3 after the decisions.

The worker stops and reports if any of these happen:

- a required artifact cannot be drafted without deciding a D-item;
- a gate locus contradicts section 1 in a way that changes a decision's options (it
  reports that to root before continuing);
- producing a card preview needs a code change;
- any write outside the owned paths becomes necessary;
- the worker reaches about $8.

Reaching a cap is not acceptance.

### 4.8 What remains open after S2

Everything in section 3 is the operator's to decide. Engineering rows A–D and F remain
open; production startup still refuses; there is still no live run, real H2, M5 or
whole-M4 acceptance.

Likely S3, depending on the answers:
- D1(b) with D3: F plus D (~$20–30). Within the remaining budget.
- D1(a) with D2 and D6 answered: A′, then a re-plan of A.
- D1(c): a closure record.

## Evidence used

`O/ROOT-ACCEPTANCE.json`; `O/003-build/RESULT.md`; `O/004-impl-review/REVIEW.md`;
`O/005-repair/RESULT.md`; the S1 plan and amendments; the M4 completion plan;
admission packet; M4c-a §§1, 3; M0 LC-6, LC-7; roadmap M4; records for the m4cb result,
installer channel, overnight result and native capture result; overnight 004
`RESULT.md` and `PLAN.md` §§1, 2, 7, 005 `RESULT.md` (head) and 006 `LEGAL-SEED.md`.
Source reads at `f596f4eb`: `native_startup.rs` (full) and `grep` of `isolation.rs`.
Line numbers for other gates come from the cited records and were not re-verified here;
S2's trace verifies them. No build, test, native run or inference took place.
