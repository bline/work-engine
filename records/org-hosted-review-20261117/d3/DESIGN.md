# D3: a real read-only review on the hosted Pi path (design)

Standing: a read-only design by one worker (Opus 5.5) for the Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`.
It is a proposal. It grants no implementation, issuance, consumption or live-spend authority beyond the
operator's 2026-11-17 D3 authorization. Source: `/home/bline/code/work-engine`, branch
`rescue-detached-20260911`, HEAD `3c1fc206`. Read with Read/Grep only. **Bash was denied in this session**, so
no `git show --stat`, no chatboard read and no context check were run. Commit statements below come from the
dev vertical's own record (`records/dev-vertical-h2-route-20261117/README.md`), not from `git`. Paths are
relative to `rust/crates/` unless they start with `records/`, `docs/` or `planning/`.

## 0. Verdict

1. **The hosted analysis profile already is the read-only hosted profile that RECONCILIATION §3 asked for**
   (`records/org-r2-core-20261117/reconciliation/design/RECONCILIATION.md:191-198`). §3 predates slice 3. The
   remaining gaps are on our side: preparing the job, delivery evidence, and result validation. **The live
   route needs no change to any claimed crate** (§2).
2. **The frozen candidate cannot be realized on this path. This needs an operator decision before any run.**
   Both acceptance conditions of the frozen requirements allow `provider_family: [anthropic]` and
   `execution_confinement: [guarded_sandbox]` (`work-engine-organization/tests/fixtures/frozen/requirements.json:37-49`, `:69-81`).
   Their proposition is "a fresh guarded Claude reviewer" (`:27`). The hosted profile admits only `gpt-6-luna`
   (`work-engine-manual-host/src/pi_coding_provider.rs:443`). A truthful realization claim therefore produces a
   `KnownIncompatible` check (`work-engine-review-preparation/src/realization.rs:93-96`, `:189-203`). Under
   evidence-and-claims §3.9 that is an unrealizable condition, which must be reported to its owner rather than
   run as if executable. I recommend **option A** (§4, OQ-1): run D3 on the R2-S **assumption world**, which is
   the frozen requirements with `realizationConstraints` removed, compiled by the unchanged compiler
   (`work-engine-review-delivery/tests/fixtures/assumption-world/DERIVATION.md:5-26`). It must be labelled. The
   frozen world stays the reported negative.

## 1. Route

```text
candidate (assumption world) + PreparationRequest{obligation, subject, hosted realization claim}
  -> prepare()                                  work-engine-review-preparation/src/prepare.rs:381
  -> FrozenDelivery::freeze(prep, transport)    work-engine-review-delivery/src/lib.rs:144-169   (contract bytes + sha256)
  -> [NEW review-hosted] render task = template v1 + contract;  materialize snapshot from git subject;
     write job/authority/packet (budget 700000)
  -> analysis-qualification --execute job.json  pi-pressure-context-host/src/bin/analysis-qualification.rs:58-188 (unchanged)
     Host: profile work-engine.pi-analysis-hosted.v1, snapshot tools, body retention, ledger, lifecycle grammar
  -> state/analysis-result.json {answer, answer_sha256}   analysis.rs:293-302; written analysis-qualification.rs:174-178
  -> [NEW review-hosted] check: parse_json(answer) -> ReviewResult::parse -> subject == request.subject
     + delivery / no-mutation / grounding checks  -> review-result.json, check.json (owner_pending)
```

**Who builds the job input.** A new crate builds it from the candidate and request. It takes the same inputs as
`ExperimentBinding::new` (`work-engine-review-delivery/src/binding.rs:153-170`) but does not use the binding.
The binding is gated behind the `fixture` feature (`lib.rs:44-49`) and requires a `FixtureDeclaration` and a
`synthetic_result` (`binding.rs:134-147`). The hosted adapter calls the public `prepare` and
`FrozenDelivery::freeze` directly. It carries the contract bytes and `contract_sha256` (`lib.rs:170-175`), and
the transport is `{attempt_id: <job_id>, generation: 1}`.

**How the contract enters.** The task bytes are a versioned, digest-pinned instruction template followed by
the contract bytes verbatim:

- The template states the snapshot layout. It requires the final message to be exactly one
  implementation-review/v1 JSON object with no code fence, the `subject` copied exactly, and evidence `sha256`
  values taken from `snapshot_list`.
- The Host plan admits the task by digest (`authority.task_sha256`, mirroring `records/dev-vertical-h2-route-20261117/analyze.py:90-95`).
- `AnalysisConfig::prompt` appends a fixed read-only suffix (`pi-pressure-context-host/src/analysis.rs:42-47`).
  The prompt is bounded at 32 768 bytes (`:26`, `:74-81`).
- The prompt is sent as `bridge_prompt` and recorded as lifecycle native input (`analysis.rs:186-211`).
- `ModelContext` adds no instruction (`work-engine-review-preparation/src/prepare.rs:109-112`). The template is
  therefore the host's own instruction layer, and it is recorded separately by digest.
- If the rendered prompt exceeds 32 768 bytes, the adapter refuses and stops. It never moves the contract into
  the snapshot silently, because that would change the delivery-evidence class.

**What the reviewer reads.**

- The snapshot is the job workspace, pinned by its manifest and never mounted in the container. The Host serves
  `snapshot_list` and `snapshot_read`, and checks every member's digest per read
  (`work-engine-manual-host/src/pi_coding_analysis.rs:1-9`, `:93-165`).
- Bounds: at most 1 MiB, 256 regular UTF-8 files and 256-character paths (`:23-24`, `:35-49`, `:149`). There
  are at most 64 reads and 32 requests (`:11-12`), and the answer is at most 64 KiB (`:28`, `:51-68`).
- The materializer reads git objects only:
  - post-change contents of the changed files at their repository paths;
  - `.review/patch.diff`, the diff from the first parent to the commit;
  - `.review/subject.json`, holding commit, tree, patchIdentity, parent and member list.
- It recomputes `patchIdentity` as sha256 of `git diff-tree --binary --no-renames --no-ext-diff <parent>^{tree} <tree>`.
  This is the only definition I found (`app-server/tests/supervisor-native-review-hosting-integration.test.mjs:253-256`,
  n=1, owner `not established`).
- It refuses on a mismatch, a binary member or an exceeded bound. The model never sees the repository.

**How the result leaves and is validated.**

- `analysis-qualification` writes `analysis-result.json` and prints `acceptance: owner_pending`,
  `domain_effect_settlement: not_claimed` (`analysis-qualification.rs:62-65`). It fails unless the Host receipt
  is `component_passed` and settlement is `CompletedSettled` (`:179-183`).
- The checker verifies `answer_sha256`, then runs `review_episode_core::codec::parse_json(answer)`
  (`review-episode-core/src/codec.rs:223`), then `ReviewResult::parse` (`implementation_review_v1.rs:31-124`).
- **Subject check.** `result.subject_digest()` (`:126-128`) must equal `digest(request.subject.to_js_value())`
  (`prepare.rs:30-37`). The three fields must also be equal exactly.
- The parse is strict: no fence stripping and no repair. A failure is recorded as `result_contract`.

## 2. Per touched crate

| Crate / file | Change | Owner | Size | Outside claimed crates? |
|---|---|---|---|---|
| **new `work-engine-review-hosted`** (lib + 2 bins `review-job-prepare`, `review-result-check`) | render task (template v1 + contract), materialize snapshot from git and recompute patchIdentity, write the job (Job JSON mirroring `analyze.py:41-112`, `budget_microusd` 700000, model `gpt-6-luna`), check (§1, §3) | ours | ~650–850 lines incl. unit tests (`unverified` estimate) | **yes.** Deps: organization, review-preparation, review-delivery (non-fixture), review-episode-core, serde_json, sha2. No build dependency on manual-host, lifecycle-* or pi-pressure-context-host; it invokes `analysis-qualification` as an external binary pinned by path and sha256 |
| `work-engine-review-delivery` | none (uses public `FrozenDelivery`) | ours | 0 | n/a |
| `pi-pressure-context-host` (`analysis-qualification`) | none; binary built unchanged, digest pinned in `authority.binary_sha256` (`analysis-qualification.rs:78-87`) | dev vertical | 0 | not touched |
| `work-engine-manual-host` profile/Host | none | dev vertical | 0 | not touched |
| `work-engine-manual-host/examples/pi_hosted_analysis_observer.rs` | **optional:** `HOSTED_OBSERVER_ANSWER_FILE` env var, the final answer read from a file instead of the fixed sentence (`:57-73`, scenario switch `:173-178`); default unchanged | dev vertical (example, not production) | ~10 | no. It is the only claimed-crate touch, and only for a positive offline end-to-end run (DQ-1) |
| `lifecycle-store`, `work-engine-server` (`job.rs`), receiver | none. The analysis profile has no server class (`docs/dev-vertical-open-items.md:36-38`); the producer runs directly | dev vertical | 0 | not touched |

The job is written by our crate, not by `analyze.py`:

- `analyze.py` hard-codes `BUDGET` at 5 000 000 µUSD (`:25`). D3's per-run cap is 700 000, which is within the
  analysis budget range of `1..=10_000_000` (`pi_coding_provider.rs:446`).
- One crate binds the contract, snapshot and job digests in one record.
- The Host revalidates every field (`pi_coding.rs:370-427`, `pi_coding_provider.rs:429-510`), so schema drift
  fails closed.
- The runtime, image and receiver inputs are the same ones `analyze.py:18-22`, `:69-71` uses (builder reads them;
  I did not, per packet).

## 3. Evidence plan

**Offline first.** No provider and no spend. The loopback observer is on synthetic routes; the producer uses
`LoopbackTransport` when no credentials are given (`analysis-qualification.rs:156-163`).

- O-1, unit tests (pure):
  - render is deterministic, and the task digest equals the authority's `task_sha256`;
  - the prompt-bound refusal fires;
  - the materializer reproduces the frozen vertical's `patchIdentity` and refuses binary or oversize members;
  - the checker accepts the fixtures `app-server/tests/fixtures/implementation-review/{acceptable-as-is,remediation-required}.json`
    with their subject rewritten;
  - the checker refuses: a fenced answer, a subject mismatch, an `answer_sha256` mismatch, a schema violation,
    and ungrounded evidence (grounding reported, not refused).
- O-2, offline end-to-end with the existing observer scenario `ok`:
  - the job is admitted by `Host::admit`, settles `CompletedSettled`, and delivery and no-mutation checks pass;
  - the checker records `result_contract` **refused**, because the observer's fixed sentence is not JSON. This is
    the negative world.
- O-3, positive offline end-to-end: only if DQ-1 lands. The observer answers a valid implementation-review/v1
  object for our subject, and the checker accepts.
- If DQ-1 is declined, O-1 plus O-2 stand. The hosted transport and ledger are already qualified by run sets
  a3/a4 (`records/dev-vertical-h2-route-20261117/README.md:82-99`, `:174-176`).

**Live runs** (operator-authorized): `analysis-qualification --execute`, with `WORK_ENGINE_HOSTED_CREDENTIALS`
named by the operator. Primary is the subscription route, with API failover (`analyze.py:72-79`). Runs live under
`~/cbm-work/org-hosted-review-20261117/runs/<run>/` and the record goes in `records/org-hosted-review-20261117/`.

**Retained per run:**

- **Inputs:** candidate requirements bytes and digest, `PreparationRequest`, preparation JSON (blockers and
  realization evaluation), contract bytes and `contract_sha256`, template digest, task, snapshot plus manifest,
  job, authority, packet, `analysis-qualification` sha256, and the HEAD the binary was built from.
- **Delivery:**
  - every request body, retained create-only with `request_body_sha256` on each attempt (README:74-76, :159);
  - checker assertion: request 1's user input text equals `prompt(task)` exactly, so the contract bytes reached
    the provider-bound body;
  - Host snapshot deliveries, each a create-only artifact matched natively (`pi_coding_analysis.rs:221-246`, `:251-288`).
- **Result:** `analysis-result.json`, the exact answer bytes as `review-result.json`, and `check.json`
  (parse/subject/grounding outcomes).
- **No-mutation:**
  - container containment: exact mount set, `NetworkMode=none`, read-only rootfs, `CapDrop ALL`
    (`pi_coding.rs:680-720`);
  - no workspace, snapshot or repository mount (README:91-92, :132-134);
  - zero Host tool requests: completion requires `tool_requests` empty (`pi_coding_analysis.rs:296`), and
    `tool_request` is refused by policy (README:152);
  - the snapshot manifest re-verified at finish (`work-engine-manual-host/src/pi_docker.rs:1238`) and at
    admission (`:301`);
  - `git rev-parse <commit>^{tree}` equal to the subject tree after the run;
  - a digest listing of the only writable bind, `state/exports` mounted at `/export` (`pi_coding.rs:605-608`).
- **Cost:** per-request `settled_microusd` and attempts from `pi-state.json`. These are catalog-rate proxy
  values (R-c, open-items:13). The paying route is recorded per attempt. Builder and review model costs come
  from the Pi ledger.
- **Monitoring:** `metrics/probes/hosted_live_observations.py` is appended after each live run (open-items:46-51).

**Claimed:**

1. A compiled implementation-review/v1 contract from organization candidate X (sha), prepared and frozen
   (`contract_sha256`), was delivered byte-exact in a provider-bound request on the hosted analysis profile.
2. A live model produced an answer that `ReviewResult::parse` accepts, with a subject equal to the request subject.
3. During the run the reviewer had no route to mutate the subject. This is supported by the Host's own
   recording and containment checks.

**Unclaimed:**

- No consumption. Both conditions say `unestablishedRoute: hold-consumption-for-owner-disposition`
  (`requirements.json:52`, `:84`).
- Standing is `owner_pending` and the grants are self-issued: `analysis-root` / `operator:analysis`
  (`analysis-qualification.rs:114-136`), accepted by the operator for single-operator use (open-items:61-66).
- No org admission: the fixture gate is not used and D6 applies. Preparation blockers are recorded verbatim, and
  none is discharged.
- The proposition "guarded Claude reviewer" is **not met** (gpt-6-luna).
- `execution_evidence_custody` and `independent_execution_evidence` are not established, because the Host
  belongs to the operator's own process.
- The review's quality and correctness are unassessed.
- For n=1 or n=2 runs, nothing is generalized.

## 4. Slice plan, cost, stops, questions

| Slice | Content | Model (per `docs/model-selection.md`, not re-read) | Est. USD |
|---|---|---|---|
| D3 (this) | design | Opus 5.5 | ≤ 5.5 cap (actual not observable by me) |
| H0 preflight | read-only. Checks, in order:<br>(a) `git cat-file -e 96808de3…` and the diff size against the snapshot bounds;<br>(b) patchIdentity recompute;<br>(c) the frozen contract size for the assumption-world candidate (a test print);<br>(d) gpt-6-luna `full_reservation` from the catalog (`pi_coding_provider.rs:546-555`), which must leave room under 700000 (`:635-645`);<br>(e) `git diff 22487ec6..HEAD -- work-engine-manual-host pi-pressure-context-host lifecycle-store` for drift;<br>(f) a chatboard read and claims | Sonnet 5.5 | 1.5 |
| H1 build | new crate, O-1 and O-2 (and O-3 if DQ-1 lands), custody runs | Opus 5.5 high | 12 |
| H2 review | Fable: evidence and claim boundaries, delivery check, strict parse | Fable 5.2 | 8 |
| H2r | remediation (if needed) | builder context retained | 4 |
| H3 live | run L1. If `result_contract` fails, one fresh job (same contract, new job id; never a repair of the answer). Optional L2 for n=2 | root | proxy cap 3 × 0.70 = 2.10, coordination 2 |
| H4 record | `records/org-hosted-review-20261117/README.md` | root | 1.5 |
| **Total** | | | **≈ 36.6, reserve ≈ 13 to $50** (`unverified` estimates) |

**Stops** (report to the root; no route change without it):

- the cumulative step total passes $45;
- H0(a) or H0(b) fails, meaning the subject cannot be reconstructed or does not fit;
- the prompt exceeds 32 768 bytes;
- H0(d) shows that reservation plus the expected settled amount exceeds 700000;
- H0(e) shows profile drift;
- `Host::admit` refuses the job offline;
- any live `provider budget exhausted`, Uncertain attempt or receipt that is not `component_passed`. Retain
  custody and do not retry by uncertainty;
- two `result_contract` failures. That is a template/model question for the root and the operator;
- any access token found in run files (`live_check.py`, README:131).

**Operator questions** (root to post):

- **OQ-1 Candidate.**

  > The frozen candidate requires provider_family anthropic and guarded_sandbox, and the hosted Pi path admits
  > only gpt-6-luna. Choose:
  > (A, recommended) run D3 on the labelled R2-S assumption world (frozen requirements without
  > realizationConstraints) and report the frozen world as unrealizable on this path;
  > (B) run the frozen candidate with its KnownIncompatible blocker recorded, as route qualification only;
  > (C) defer D3 to a Claude realization (dev vertical's Claude Host core, d6960f9a, readiness not established).

- **OQ-2 Subject.**

  > Use the frozen vertical's subject (commit 96808de3…, tree 9c5d94ba…, patchIdentity 5212141d…,
  > `work-engine-review-delivery/tests/common/mod.rs:73-80`) if H0 shows it exists and fits; otherwise which
  > commit?

**Dev-vertical questions** (exact wording, ready to post):

- **DQ-1.**

  > May the org vertical add an optional `HOSTED_OBSERVER_ANSWER_FILE` to
  > `work-engine-manual-host/examples/pi_hosted_analysis_observer.rs` (final answer text read from that file;
  > default behaviour unchanged; ~10 lines, example only), or would you rather add it? It lets us qualify an
  > implementation-review/v1 answer end to end offline. If neither, we qualify with the existing `ok`
  > scenario (negative) plus unit tests.

- **DQ-2.**

  > Which `analysis-qualification` binary should review jobs pin: the a4 binary `150384ad…` built at
  > `22487ec6`, or one we build unchanged from a commit you name? Is anything on `claude-realization-20261117`
  > about to change `work-engine.pi-analysis-hosted.v1`, its grammar, `TOOLS_SHA256` or the Job schema?

- **DQ-3.**

  > We plan 1–3 live review jobs through `analysis-qualification --execute` on the analysis profile:
  > `gpt-6-luna`, `budget_microusd` 700000, subscription primary with API failover, no server class, results
  > `owner_pending`, grants self-issued as in your runs. Any objection? Should we append them to
  > `metrics/dev-vertical-live-observations.jsonl` with your probe?

- **DQ-4.**

  > If a review needs more than the 300 s fresh-job deadline (open item "Job deadline"), is a profile-specific
  > bound on your roadmap, or should review jobs stay within 300 s?

## 5. Not established

- Whether commit `96808de3…` exists in this repository, its diff size against the snapshot bounds, and which
  baseline its `patchIdentity` was computed against. No baseline appears in the frozen fixtures (grep over
  `work-engine-organization/tests/fixtures/frozen/`). The owner of the `patchIdentity` definition is also not
  established.
- The byte size of the frozen contract for the assumption-world candidate, against the 32 768-byte prompt bound.
- gpt-6-luna's `full_reservation` against the 700000 budget. The coding profile ran under that cap, which
  suggests the reservation is ≤ 700000 per request (`unverified`).
- Whether gpt-6-luna returns strict, schema-valid implementation-review/v1 JSON, and whether a review fits in 32
  requests, 64 reads and 300 s. The one live analysis run (n=1) took 25 s and 3 requests (README:124-127).
- Whether HEAD `3c1fc206` (or the claimed branch) changed the analysis profile after `22487ec6`. The commits are
  known only from their subjects in this session's git status.
- Whether the hosted container belongs to any `execution_confinement` class (no owner maps it), and whether
  Host recording counts as `independent_execution_evidence`.
- Active board claims and the dev vertical's current position: the board was not read (Bash denied).
- Crash and cancel for the analysis profile offline (open-items:39), inherited.
- The content of `work-engine-server/src/job.rs`: not read. It is not on this route.
- The context-pressure reading for this session: `usage_unavailable` (Bash denied).
