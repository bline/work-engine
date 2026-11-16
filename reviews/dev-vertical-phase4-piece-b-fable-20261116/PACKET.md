# Independent implementation review packet: phase 4 piece (b), hosted Pi provider transport

## Reviewer authority and constraints

- Reviewer: `claude-fable-5-2` at `high` effort, fresh context. The operator
  gave Fable the independent review authority intended for the inherited
  Astra/xhigh review (2026-11-16). This is a same-provider review; a Claude
  Opus session wrote the implementation.
- **Read-only.** Do not modify any file. Do not run builds, tests, Docker or
  network operations, and do not access credentials. Reading files, `git
  show`, `git diff` and `git log` are fine.
- **Subject:** commits `9131a20d`..`c66f2baf` on the current branch (base
  `0933a91e`; `9d658689` is the admission-owner change that owner already
  accepted, included for context). Exact blob ids are in
  `subject-manifest.txt`. Use `git show c66f2baf:<path>` wherever the working
  tree differs from those blobs.
- **Scope:** review the implementation against its design and the operator
  contracts below. Do not redesign the subsystem; prefer the smallest change
  that closes a real gap.

## Read first

1. `planning/dev-vertical-phase4-hosted-transport-r2-20261116.md`: the design
   (revision 2, after your design review).
2. `reviews/dev-vertical-phase4-design-fable-20261116/REVIEW.md`: your
   earlier design review (R1–R5, K1–K5). Check how each item was resolved.
3. `planning/dev-vertical-hosted-gate-contract-20261116.md`: the lifecycle
   gate contract, including its amendment.
4. `records/dev-vertical-hosted-offline-run-20261116/README.md`: the offline
   Docker run and the defects it found.

## Operator contracts (binding)

- The hosted profile is built on the lifecycle bridge composition. The
  lifecycle gate (`rust/crates/pi-pressure-context-host/src/coding.rs`),
  not `PiOwner`, fences provider entry. `Effect::ProviderAttempt`
  (`9d658689`) is deliberately unused by this profile.
- The lifecycle grant must cover the permitted failover policy. A grant for
  one logical request must not silently become permission for unlimited
  provider dispatches or unauthorized payer changes. The grant binds the
  allowed routes and payers, the attempt limit and the deadline, and
  currentness is checked before each dispatch.
- Logical request versus attempts: each attempt has its own identity and
  reservation under the request. Only the financial owner (the provider
  ledger) releases, replaces or grants money.
- Complete screened SSE retention is required, distinct from terminal-event
  accounting.
- The inference container has no network and no access material. Access is
  held on the Host and used only at dispatch.

## Code

- `rust/crates/work-engine-manual-host/src/pi_coding_hosted.rs` (new): route
  table, `HostedProviderTransport`, `LoopbackTransport`, Host-loop stream
  state, attempts, settlement, retention.
- `rust/crates/work-engine-manual-host/src/pi_coding_provider.rs`: attempt
  records, `reserve_attempt`, `settle_attempt`, `settle_completed`, plan and
  admission binding (`hosted`, `hosted_routes_sha256`), `provider_complete`.
- `rust/crates/work-engine-manual-host/src/pi_coding.rs`: frame routing
  (`provider_fetch`/`pull`/`cancel`/`ack`), refusal of container
  `provider_observation`, grant augmentation in `coding_bridge_dispatch`.
- `rust/crates/work-engine-manual-host/src/pi_docker.rs`: Host loop tick
  (search for `coding_hosted_tick`).
- `rust/crates/work-engine-manual-host/receiver/pi-coding.mjs`: hosted
  receiver (fetch injection, zstd decompression, marker credentials).
- `rust/crates/pi-pressure-context-host/src/coding.rs`: `HostedProviderPolicy`,
  policy-bound grant ids, `provider_attempt`, hosted report.
- `rust/crates/pi-pressure-context-host/src/bin/coding-qualification.rs`:
  embedder (policy from the route table, grant installation, transport).
- Unchanged piece-(a) code it relies on: `pi_stream.rs`, `pi_route.rs`,
  `receiver/pi-native-tools.mjs`.
- Tests: `pi_coding_hosted.rs` (tests module), `pi_coding_provider.rs`
  (`attempt_tests`, profile admission cases 10–16),
  `pi-pressure-context-host/tests/coding.rs` (hosted tests),
  `tests/pi-coding-hosted.test.mjs`.

## Focus areas

1. **Financial correctness.** Durable attempt reservation before any
   upstream byte; release only on a definitive refusal or a confirmed
   not-dispatched cancel; an `Uncertain` attempt never authorizes another;
   budget accounting across attempts; settlement of the completed attempt
   (`provider_complete` path); the router's allowance being disabled.
   Restart: what does reconcile do with an unsettled attempt?
2. **Grant coverage.** Does every upstream write pass through
   `provider_attempt` with currentness held during the write? Can routes,
   payers or the attempt count escape the policy bound into the grant ids?
   Is the deadline the minimum of grant expiry and the job deadline?
3. **A deliberate deviation to judge.** The gate originally recorded
   `pi.provider_attempt` in the turn ingress, which made settlement fail
   (the settled event grammar has no such event). The fix keeps attempts in
   memory for the continuity report and relies on the Host ledger's durable
   attempt records. Is this adequate evidence of gate-admitted attempts, or
   is a durable lifecycle-side record required?
4. **Host-loop integration and races.** Non-blocking progress; the
   `pre_dispatch`-equivalent scan of unprocessed container lines for
   `cancel(1)`; cancel before reservation (`StreamPhase::Fetched`); crossing
   `response-unavailable`; frames interleaving while a stream is open;
   `coding_hosted_before_complete` drain; ordering between direct stream
   frame writes and lifecycle command writes in `pi_docker.rs`.
5. **Evidence and isolation.** Host-produced status, backend model and raw
   terminal; refusal of container observations; retained SSE and refusal
   bodies bound to attempt digests; credential header stripping and the
   Host's refusal of `content-encoding`; the receiver's bounded zstd
   decompression; container arguments (network none, no `/auth`).

## Required output

Write your review as your final answer, in Markdown:

1. A **disposition**: `accept`, `accept with required changes`, or `reject`,
   for proceeding to piece (c), qualification.
2. **Findings.** For each: severity (P1/P2/P3), evidence (`path:line` at
   `c66f2baf`), why it matters, and the minimal change. Separate confirmed
   defects from risks and open questions.
3. A short answer for each focus area, and the status of each earlier design
   review item (R1–R5, K1–K5).
4. Anything you could not verify, and why.

Be concise. Do not restate the design.
