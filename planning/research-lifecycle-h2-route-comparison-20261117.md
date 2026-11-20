# H2 execution route comparison: native Codex host vs the dev-vertical hosted Pi path

2026-11-17, revision 1. **Read-only analysis for an operator route decision. It decides
nothing, implements nothing, grants nothing and changes no host.** Packet:
`/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/011-route-compare/PACKET.md`
(`O` = `/home/bline/cbm-work/research-lifecycle-tool-registry-20261116`). Root and
collector: Pi session `01a1e56d-7ceb-71fc-9c46-c9fcd9618234`. Analyst: Pi session
`01a1e8c1-cc80-7316-b53e-257b3439032d`. Source read at shared HEAD `9b289171`. All
dollar figures are API-equivalent **estimates** from recorded plans and source size,
not measurements. Line numbers are from that HEAD.

## Short answer

The hosted Pi path is real, qualified and live for one thing: a fixed coding-continuity
job. It is not yet an execution route for the H2 audit. Three facts decide most of the
comparison.

1. **The qualified Pi live class is a narrow fixture.** The operational server admits
   only model `gpt-6-luna`, exactly 4 provider requests, 1 tool call, the write
   boundary `src/lib.rs`, a context of at most 24 000 bytes, a budget of at most
   700 000 µUSD and a command timeout of at most 30 s
   (`work-engine-server/src/job.rs:207-227`; also `pi_coding.rs:371-379`). The Host
   caps a request body at 45 000 bytes (`pi_coding_hosted.rs:27`). Its only tool is
   `workspace_exec`, a shell (`receiver/pi-coding.mjs:155,180-181`). The proposed H2
   snapshot is about 179 000 bytes (`O/007-h2-artifacts/snapshot-manifest.json`), and
   the audit prohibits shell, edits and network. H2 on Pi therefore needs a **new
   versioned analysis profile**, not reuse of the qualified one.
2. **The Pi path touches no research-lifecycle owner.** `work-engine-server`,
   `work-engine-manual-host` and `pi-pressure-context-host` do not depend on
   `work-engine-task-host`, `retained-task-workflow`, `lifecycle-service` or
   `research-register` (their `Cargo.toml` files). The consumer seam LC-6 names is the
   task host, and `lifecycle-service` drives the Codex app-server JSON-RPC directly
   (`operations/mod.rs:301,479,774`). LC-6 also says: "If that prerequisite is not
   qualified, report it before admission; do not create a new runtime to bypass it."
   So the Pi route needs a new adapter that makes the Pi Host a native realization
   under the existing task host, or an explicit operator change to what M4's first use
   is. The roadmap's M4 exit says: "If the selected host cannot establish required
   evidence, report the unmet condition rather than widening this into a runtime
   rewrite" (`planning/research-lifecycle-rust-implementation-plan-20261111.md:228-230`).
   Both Pi options change a first-use binding, so each needs operator authority: that
   a Pi realization under the task host is not the excluded widening (R7′), or an
   explicit redefinition of the M4 exit (R1(c)).
3. **The largest remaining gate is shared.** S2's G4 (no production basis verifier, no
   Preparer/Adopter installation, host dispatch accepts only `research.invalidate.v1`)
   lives in `retained-task-workflow` and `work-engine-task-host`, and it applies to
   both routes.

What the Pi route removes is real: no modified native build to select (D3), no
seven native utilities (D5), Docker containment that is already inspected per run
instead of a kernel FD witness under bwrap (most of D2), a Host that sees every request
body before dispatch and executes every tool itself (D4 becomes cheap prevention), and
an API-key route whose funding is independent of the exhausted Codex quota (a
funding-source property; it does not make a live run possible sooner, see section 5).
What it adds is the new profile, request-byte retention with card association, and
the task-host adapter.

**Recommendation (section 7):** keep the native Codex host as the M4 first-use route
by default, answer the route-independent decisions now, and buy a small read-only
sizing of the Pi task-host adapter before committing either way. If the operator
declines R7 (owning a native fork), the Pi route under the task host is the only path
to a real H2, and it requires R7′ (the operator decision that a Pi realization is not
the excluded widening, or an amendment of the M4 exit). If R7 is declined and R7′ is
not granted, R1(d) (report the unmet condition and close M4) is the roadmap's
prescribed outcome. Main uncertainty: the size of that adapter.

## 1. The two routes as they stand

| | Native Codex host | Dev-vertical hosted Pi path |
| --- | --- | --- |
| Components | `lifecycle-codex`, `lifecycle-service`, `work-engine-task-host`, `retained-task-workflow`, `research-register`; private native build `77fbfa4b…` (S1 + 008) | `coding-job-prepare` → `work-engine-server` → `work-engine-command` → `work-engine-manual-host pi-docker` Host; lifecycle gate in `pi-pressure-context-host`; store `lifecycle-store` (`pi_coding.rs`) |
| Standing | M4b, Q10/Q11, 008 producer and S1 accepted at private scope; production startup, admission, prepare/adopt, delivery and grants refuse (S2 G1–G7) | Offline matrix 19/19 (r8), live runs `live-1`, `c1`–`c3` through the operational entrypoint; the qualification record is "not owner acceptance" (README:9-12); review 3 remediation `7c103ad2` not re-reviewed. (Per-job results are `owner_pending` by design, open-items:39-42; that is a per-job output policy, not a pending acceptance of the path.) |
| Model | Fixed `gpt-6-sol` (`lifecycle-service/src/lib.rs:228`) | Live class fixed `gpt-6-luna` (`job.rs:213`) |
| Provider | Research install route source-fixed to the ChatGPT access lease (`research_installation.rs:1349-1362`, S2 G3) | Primary `https:chatgpt-codex` (subscription access from Pi's auth store), secondary `https:openai-responses` (API key), failover only on definitive 401/402/403/429 |
| Containment | bwrap plus a Unix proxy (`isolation.rs`); non-controlled observation refuses sockets and protected FDs (G2) | Docker, `--network none`, `--read-only`, `--cap-drop`, `no-new-privileges`, `--pids-limit`, `--user` (`pi_docker.rs:873-886`), verified by inspection (`:1530`); no `/auth` mount; credentials only in the Host process |
| Lifecycle store | `lifecycle-store` native session/turn grammar via `lifecycle-service` | `lifecycle-store` Pi coding grammar via `pi-pressure-context-host` |

Both routes share `lifecycle-core`, `lifecycle-store` and `lifecycle-runtime`; nothing
above them is shared.

## 2. M4 obligations, route by route

Owner names follow the existing crates and contracts. "Missing" lists what is needed
for one real H2, not for whole-M4 acceptance.

### (a) C1: complete final model-request delivery evidence with card/request association

| Route | Today | Missing | Owner |
| --- | --- | --- | --- |
| Codex | 008 strict transport-input producer in the private build captures the final request bytes; `AcquiredNativeInput` checks them (`lifecycle-codex/src/compiled_input.rs:37-50`); `retain_native_input_with_registry` keeps them (`lifecycle-store/src/native_input.rs:88`). The task host's `verify_delivery` checks card reconstruction and request digest against the native causal read (`work-engine-task-host/src/research_delivery.rs:366-400`) | Production use: D3, then a production caller of `acquire` (only the `controlled-proof` `checked_fixture_basis` calls it, G5) and G4 | Native producer (private fork); `lifecycle-codex`; task host |
| Pi | **Verified at source.** The Host forwards the container's fetch body byte-identical: `HostedRequest.body = fetch.body` (`pi_coding_hosted.rs:775-780`) and `HttpsTransport::send` writes the reconstructed head, then exactly `request.body` (`pi_coding_https.rs:268-269`). The head is deterministic from the endpoint, the request/attempt id and an allowlist (`pi_coding_hosted.rs:226-248`), except the access headers, which are never retained. The body is **not** retained per attempt: `AttemptRecord` has `relay_sha256` and `refusal_body_sha256` but no request digest (`pi_coding_provider.rs:72-104`). The body survives only inside the container's stdout fetch frame (base64), which the Host digests at process end and snapshots as a host artifact (`pi_docker.rs:815,1246-1260`). A join attempt → request id → fetch frame is possible by re-parsing stdout; failover attempts reuse the same fetch. Response bytes per attempt are retained create-only and registered (`pi_coding_hosted.rs:1221-1301`) | A create-only per-request body artifact with its digest on `AttemptRecord`, persisted before dispatch (small); a card/request association, which needs a research delivery record naming the card digest and the request-body digest and spans (does not exist on Pi); a production consumer (the task host) | Pi Host (`work-engine-manual-host`) for retention; task host and `research-register` for the card join |

Assessment: the Pi Host is structurally better placed for C1. It holds the exact body
in memory before dispatch, the container cannot send anything to a provider itself,
and a Host-enforced request count makes every request enumerable (the live class's 4
requests will not hold for the analysis profile: P1 must raise the request and body
bounds, so the bound is a profile parameter). It lacks the retention record and any
research association. The Codex route has both pieces only in a private fork.

Sizing note: consumption of C1 is smaller than the full adapter. `verify_delivery`
(`research_delivery.rs:366-400`) works over `RetainedDelivery` bytes and does not
depend on the Codex source; only `acquire` (op `native_causal_read`, via the actor)
is Codex-specific. A request-body artifact with a digest on `AttemptRecord` (the
small piece above) plus a second acquirer is far smaller than P2. The conclusion
stands: the `Reconstruction` (card to request spans) is produced in the task host's
preparation path, so the Host alone cannot associate card and request. P0 should
therefore size C1 consumption separately from turn mapping, fence join and the
install predicate.

### (b) Effective tool set and dispatch evidence (S1's claim)

| Route | Today | Missing | Owner |
| --- | --- | --- | --- |
| Codex | S1: per-step router snapshots and dispatch citation, refusal of use, private build; seven approved native utilities (`tool_registry.rs:34`) | D3; D4 (refusal versus prevention); D5 (utilities); disabling knobs untested (S2 D5 table) | Native producer; profile owner |
| Pi | Pi built-in tools disabled (`noTools:'builtin'`), only custom tools (`receiver/pi-coding.mjs:181`); the ready frame's active tool names are checked against the profile (`pi_coding.rs:1004-1013`); every tool execution is a Host request matched to its completion (`pi_coding.rs:1446-1458`); the Host body policy admits only `type: function` tools (`pi_coding_hosted.rs:576-588`) | An analysis tool set (snapshot read, question open, result publish; no `workspace_exec`) served by the Host, following the existing read-only claims-tools pattern (`pi_coding.rs:1405-1444`); a Host check that each request's `tools` array equals the pinned spec digest before dispatch | Pi Host and receiver; analysis-profile owner |

Assessment: S1 does not carry over, but the Pi equivalent is cheaper and stronger. A
Host check on the request's `tools` array happens **before** any provider byte, and the
Host executes every tool, so D4's prevention option is the natural Pi realization. The
receiver code and image are pinned; whether that pin is enough tool-surface provenance
is an owner question (R8″), as is approval of the analysis tool list itself.

### (c) Protection and access profile; how D2 changes

| Route | Today | Missing | Owner |
| --- | --- | --- | --- |
| Codex | bwrap plus proxy; non-controlled observation refuses sockets, protected FDs and ancestors (G2); no per-FD witness for the actual containment; 004 route estimated 11–22 units after four prerequisites | D1(a): A′ then A; or D1(b) narrowed claim. D2 (does the proxy bridge count as a capability crossing?). D6 credential-free readiness (readiness half of D6: a provider-disabled readiness fixture equivalent to the actual profile, or a real provider lease; needed only under the full claim, see R2) | Protection owner |
| Pi | Every run inspects `NetworkMode=none` and the absence of `/auth`; no credential in the container; secret scans of every run file in live runs; access screening of response bytes; no containers left after a run (qualification record, invariants). Descendants are bounded by the container and `--pids-limit` | A statement by the protection owner that Docker inspection plus host-only credentials meets the research lifecycle's protected-set and descendant conditions (repair plan §4.1, M4c-a §3.3), or the conditions to add; the protected set for an analysis job (no repository mount; snapshot served through Host tools) | Protection owner; Pi Host |

How D2 changes: the only crossing is the inherited framed stdin/stdout IPC of
`docker run`; there is no proxy socket into the container. D2's question becomes "is
the declared framed IPC an acceptable crossing", which the dev vertical already
assumes. It is still the owner's wording decision, but it no longer blocks on a kernel
witness that bwrap could not provide (005).

### (d) Production research prepare/adopt and basis verification (G4)

G4 applies to both routes. `InstalledResearch::dispatch` accepts only
`research.invalidate.v1` (`research_installation.rs:996-998`); every basis verifier is
`controlled-proof` gated; non-controlled `adopt_fresh` is unavailable (S2 G4). These
live in `retained-task-workflow` and `work-engine-task-host`, not in a host.

Host-specific parts: the basis consumes the delivery proof, and the only delivery proof
today is the Codex native causal read (`research_delivery.rs:42-56`, op
`native_causal_read`). The install route also requires `startup.control.is_native()`
and `NativeProviderRoute::OpenaiChatgptAccessLeaseV1` (G3). On the Pi route both
predicates need a Pi equivalent: a delivery/causal record produced from the Pi Host's
retained request bytes, and an install predicate that admits the Pi realization. So G4
is shared, and its inputs are host-specific on either route.

### (e) Research delivery into the task; causal and receipt joins

| Route | Today | Missing | Owner |
| --- | --- | --- | --- |
| Codex | Chain exists under `controlled-proof`: `actor.rs` passes `eligible` to `retain_native_input`; `native_causal.rs` refuses ineligible input; the M4b preparation fence in `actor.rs` (~2712–2724) rejects `!input.eligible` (tool-registry plan §2) | Production caller of the chain (G5) after D3 and G4 | `lifecycle-service`, task host |
| Pi | None. The Pi Host's evidence (attempt ledger, stream files, lifecycle gate turns, receipt with route/payer per attempt) is not joined to any task, preparation or adoption record | A task-host join: the task host must drive the Pi Host as its native, retain the request bytes as native input with an eligibility flag (the shared `lifecycle-store` already has `retain_native_input_with_registry`, but keyed to native session/turn entries the Pi grammar does not create), and pass the same preparation fence | New adapter (section 3) |

### (f) Restart and cancellation

| Route | Today | Missing |
| --- | --- | --- |
| Codex | Q10/Q11 startup and restart qualified privately; rehydration and early ingress unqualified (S2 B); D7 proposes fresh start only | D7 |
| Pi | `pi-docker reconcile` marks a pending attempt Uncertain, keeps its charge and retained bytes, and refuses replay ("reconcile instead of launch replay"); `pi-docker cancel` and mid-stream cancels exercised offline; a live crash or cancel against a real route not exercised (open items) | Restart semantics of the composed task-host + Pi path (new); D7's fresh-start-only choice applies the same way. A crashed H2 arm would be a retained failure outcome under LC-7, not a rerun |

### (g) Authority, installer channel, grants (D8)

Both routes need the same research authority: O1 installer channel (the record names
"this Codex conversation", `records/research-lifecycle-m4cb-result-20261112.md:7`),
research and decision owner approvals of exact bytes, LC-7 estimand acceptance,
verifier. Grants differ:

- Codex: `grant_for_context` accepts exactly one trusted configured grant per context
  and scope; none exists for the H2 tasks (G6).
- Pi: lifecycle grants are **self-issued** by the producing binary for its own job
  (open item K-c), accepted by the operator for single-operator local use. Authority
  rests on the operator-pinned runtime admission and job credential digest
  (`job.rs:1-10`). For a research task this would sit under the task host's grant
  rather than replace it; K-c is acceptable only if the task-host grant is the
  authority and the Pi grant is a derived fence. That is a D8 sub-decision.

### (h) Provider, model, funding (D6) and the Codex quota

- Codex quota is exhausted until 2026-11-21 (settled fact). The Codex research install
  route is source-fixed to the ChatGPT access lease (G3), so the Codex route cannot run
  live before then without a source change to G3.
- The Pi primary route `https:chatgpt-codex` posts to the ChatGPT Codex backend with
  the subscription's access. It very likely draws on the **same** exhausted plan quota
  (inferred, not verified; the completion plan notes a plan-usage-limit error can be
  app-specific). The secondary `https:openai-responses` route is API-key funded, has
  carried live traffic, and is independent of the quota. A Pi live run before
  2026-11-21 would use paid API funds (D9), but no such run is possible before the Pi
  sequence in section 5 completes.
- Settlement and the budget use the pinned catalog rates on every route (R-c), so a
  Pi cap bounds a proxy, not actual API spend.
- Model: the qualified live class pins `gpt-6-luna`; the Codex profile pins
  `gpt-6-sol`. An H2 profile on Pi must name its own model and effort (D6), and LC-7
  requires the same model for both arms.

### (i) LC-7 baseline arm on the same host

Both routes can run the two arms on the same host with the same tools by varying the
snapshot members. Exposure evidence differs. On Codex it is the 008 capture plus S1
snapshots (private build). On Pi, with per-request body retention, the exact bytes the
model saw are retained for every attempt by the Host itself, which is direct exposure
evidence that the baseline did not receive the research bundle. Either way the
baseline must run on the same profile, model, effort, limits and tool set as the
treatment.

## 3. Ownership and placement for the Pi route

Existing owners that would compose, each keeping its semantics:

| Responsibility | Existing owner |
| --- | --- |
| Task, research admission, delivery verification, basis, prepare/adopt, grants | `work-engine-task-host`, `retained-task-workflow`, `research-register` (unchanged consumer seam per LC-6) |
| Task lifecycle and native session/turn records, native input retention | `lifecycle-service` with `lifecycle-store` |
| Containment, attempt ledger, funding failover, request/response retention, tool execution | `work-engine-manual-host` Pi Host (`pi_docker`, `pi_coding_hosted`, `pi_coding_provider`, `pi_coding_https`) |
| Grant-bound provider attempts | `pi-pressure-context-host` lifecycle gate (`CodingEffectGate::provider_attempt`) |
| Financial meaning | `pi_coding_provider` ledger (single financial owner) |

New pieces, and where they belong:

1. **Pi analysis profile** (new versioned profile, e.g. `work-engine.pi-analysis-hosted.v1`,
   name not decided) in `work-engine-manual-host` and its receiver: Host-served
   read-only snapshot tools, question and result tools, no shell, raised and
   re-qualified context, body and request-count bounds (the 64 KiB receiver frame
   bound noted at `pi_coding.rs:372-374` needs chunking or a redesign), a request
   `tools` digest check, and per-request body retention. The provider ledger, gate
   and transport are reused unchanged.
2. **A Pi native realization for the task host.** Today `lifecycle-service` is Codex
   specific: it depends on `lifecycle-codex` and its operations speak app-server
   JSON-RPC (`thread/start`, `turn/start`, `item/tool/call`). The sibling placement
   that avoids duplicating ownership is a `lifecycle-pi`-style adapter beside
   `lifecycle-codex`, used by `lifecycle-service`, that drives the Pi Host, maps its
   attempts to native turns, retains request bytes as native input through
   `lifecycle-store::retain_native_input_with_registry`, and produces the delivery
   causal record the task host reads. Research semantics stay in the task host. This
   is an architecture decision for the owners; this document does not make it.
3. **Not recommended:** having `work-engine-server`'s coding-job adapter or the Pi Host
   perform research admission or basis checks. That would put research semantics in
   the execution host and duplicate the task host (system design: one semantic owner),
   and LC-6 says "do not create a new runtime to bypass it", which requires explicit
   owner authority to override.

Why item 2 keeps the consumer seam and R1(c) does not: LC-4 and LC-5 place the
prepared basis, the adoption receipt and the preparation fence under
`retained-task-workflow` and the task host, and the M4 exit requires "one actually
authorized bounded execution with its basis and required verification evidence". With
item 2 the task host remains the consumer of all three and only the native
realization beneath `lifecycle-service` changes; R1(c) produces the evidence outside
them.

## 4. What carries over

| Prior work | Native Codex route | Pi route |
| --- | --- | --- |
| S1 (tool registry, dispatch citation) | Carries over once D3 selects the build | Does not carry over. Its claim shape (expected tool set, refusal on mismatch, retained snapshot) is reusable as a specification; the realization is new and simpler |
| 008 strict input capture | Carries over with D3 | Does not carry over; Pi needs its own retention (small). `AcquiredNativeInput`'s checking role would need a Pi analogue |
| Task-host join and preparation fence | Carries over (controlled today) | Fence semantics carry over; the join is new (section 3, item 2) |
| S2 drafts (`O/007-h2-artifacts/`) | Carry over | Mostly carry over: audit request, snapshot, rubric, LC-7 protocol, authority skeletons are host-neutral. Must change: the analysis tool names, the G1–G7 trace (Pi-specific gates replace G1, G2, G5 and the G3/G7 provider predicate), `advisory-input.md`'s delivery observation (008 → Pi retention), and the D5 section (does not apply) |
| Dev-vertical qualification | Not relevant | Ledger, failover, retention, crash/cancel and secret screening carry over; the profile bounds, tool set and live class admission do not |

## 5. Cost and slice sequence to one real H2 (estimates)

All rows are rough API-equivalent ranges including review, not authorized. Remaining
objective budget is about $64 before this comparison.

**Native Codex route, D1(b) narrowed claim, fresh start only (D7):**

| Slice | Estimate | Needs |
| --- | --- | --- |
| F: production startup composition | $15–25 | D3 |
| D: disable or approve seven utilities | $0–18 (about $0 if the utilities are approved as they are) | D5 |
| G4: production basis verifier, Preparer/Adopter installation and dispatch | $25–45 | D1, D3 |
| G5: production C1 consumption | $10–20 | D3, G4 |
| O1 installation and grants (exact manifests) | $3–6 | D8 |
| Three live runs plus verification | $6–20 (subscription-funded after 2026-11-21; Sol-class API-equivalent per arm perhaps $2–6) | D6, D8, D9 |
| **Total** | **about $60–135** | Earliest live date 2026-11-21 |

D1(a) adds A′ ($6–10) and A (unknown; 004 estimated 11–22 units after prerequisites),
plausibly another $40–90.

**Pi route, same narrowed claim class:**

| Slice | Estimate | Needs |
| --- | --- | --- |
| P0: read-only sizing of the task-host Pi realization, with C1 consumption sized separately from turn mapping, fence join and install predicate | $5–8 | none (route-independent information) |
| P-Q: re-review of review-3 remediation `7c103ad2` and owner acceptance of the hosted profile's qualification as the H2 base (LC-6 requires "an actual qualified build/provider/startup") | $5–15 | R7″ |
| P1: Pi analysis profile (tools, bounds, frame chunking, tools-digest check, request retention) and offline qualification | $20–35 | R1, D6 (model) |
| P2: task-host Pi realization (native turn mapping, native input retention, causal record, fence join, install predicate) | $50–120, **least certain** | R1, R2, architecture acceptance |
| G4 (shared) | $25–45 | D1 |
| Protection statement for Docker containment | $3–8 | D2′ |
| O1 installation and grants | $3–6 | D8 |
| Three live runs plus verification on API funds | $4–20 (depends on model; Luna-class runs cost cents, a Sol-class model several dollars per arm) | D6, D8, D9 |
| **Total** | **about $115–255** | No fork; funding independent of the Codex quota (not an earlier live date) |

Pi earliest live date: not estimated. The sequence is P0, then R7′, then P-Q and P1,
then P2, then G4, and only then a live arm. Nothing in these sources suggests that
it completes before 2026-11-21, so the advantage over Codex is funding-source
independence (R5, R6), not schedule.

**Pi without the task host (option R1(c) below):** P1 plus live runs plus a
hand-joined evidence package, about $35–65 (plus row P-Q below if the hosted path's
standing is to be accepted). It would not exercise production prepare/adopt or basis
verification through their owners, and the M4 exit requires the basis and
verification evidence through them, so it would not meet the M4 exit as defined; it
would be an LC-7 feasibility observation outside the lifecycle.

The Pi full route exceeds the remaining ~$64 at every point of its range. The Codex
full route's low end (about $60, if the seven utilities are approved as they are)
is close to it and its upper end exceeds it. The Pi route's extra cost is mostly P2,
which replaces D3, F, D and the protection witness.

## 6. Revised decision list (replaces D1–D9)

"RI" marks route-independent decisions the operator can answer now.

| ID | Decision | Options and consequence | RI | Replaces |
| --- | --- | --- | --- | --- |
| R1 | Execution route for the first real H2 | (a) native Codex host; (b) Pi hosted path under the task host (new adapter, section 3), which requires R7′; (c) Pi hosted path outside the task host, which redefines the M4 exit for this use and needs explicit operator authority; (d) report the unmet condition and close M4, the roadmap's prescribed outcome if the selected host cannot establish the required evidence | — | new |
| R2 | Claim scope | Full protection claim, or a named narrower claim for this one analytical audit; on Codex the narrowing concerns sockets/FD witness, on Pi it concerns whether Docker inspection is the protected-surface evidence. Under R2(full) on Codex the readiness half of D6 stays open: a credential-free, provider-disabled readiness fixture equivalent to the actual profile, or a real provider lease for readiness (Codex only). R2(narrow) retires it: S2 ties it to the full-claim route A (D1(a)) and its next slice lists D2 and D6 only for D1(a) | partly | D1; readiness half of D6 |
| R3 | Scope before the first H2 | Fresh start only, restart and rehydration refusals kept, rows stay open for whole-repair acceptance; or close them first | **RI** | D7 |
| R4 | Research authority | O1 channel rebinding (to the Pi root or a Claude root, or wait for Codex), research owner, decision owner, LC-7 estimand acceptance, verifier and receipt locations; per-task execution and inference grants with exact scope and budget (Codex: no grant exists, G6; Pi: mechanism per R8′); the owner who verifies and accepts the audit result. Each binds to S2 bytes | **RI** | D8 |
| R5 | Budget | Whether to extend the $100 objective (the Pi full route does not fit in ~$64 and the Codex full route fits only at its low end) and the live inference allowance | **RI** | D9 |
| R6 | Funding and model for live arms | Codex: ChatGPT lease after 2026-11-21 (G3 fixed). Pi: API-key route (funding independent of the quota, but no live arm exists before the Pi sequence completes), or subscription after 2026-11-21; name model and effort, same for both arms | — | D6 (funding and model half) |
| R7 (Codex only) | Select the modified native build | Approve `77fbfa4b…` or a successor and own the fork; require upstream; decline (then R1(a) is impossible) | — | D3 |
| R8 (Codex only) | Refusal of use vs prevention; the seven utilities | As D4 and D5 | — | D4, D5 |
| R9 (Codex only) | Capability-FD wording | As D2 | — | D2 |
| R7′ (Pi only) | Operator authority for a Pi realization under the task host | The decision that a Pi realization under `lifecycle-service` (section 3 item 2) is not the widening the M4 exit excludes (roadmap lines 228–230), or an amendment of the M4 exit for this use; symmetric with R1(c), which redefines the exit outright. Then architecture acceptance of section 3 item 2, after P0 sizing. If not granted and R7 is declined, R1(d) applies | — | new |
| R7″ (Pi only) | Accept the hosted Pi path's qualification standing as the H2 prerequisite | The qualification record is "not owner acceptance" and `7c103ad2` is not re-reviewed; LC-6 requires an actual qualified build/provider/startup. Accept after the re-review (row P-Q), or name what else qualifies the base. The analogue of R7 for the Pi Host | — | new |
| R8′ (Pi only) | Grant derivation | The task-host grant is the authority and the Pi lifecycle grant a derived fence, or an independent grant issuer first (K-c) | — | part of D8 |
| R8″ (Pi only) | Tool surface | Approve the analysis tool list (snapshot read, question open, result publish; no `workspace_exec`), and accept Host `tools`-digest prevention plus the pinned receiver and image as the equivalent of S1's claim | — | D4, D5 (Pi analogue) |
| R9′ (Pi only) | Framed IPC as the declared crossing; Docker inspection as the protected-set evidence | Owner wording, as D2 rephrased | — | D2 |

Answering R3, R4 and R5 now costs nothing in engineering and is needed on every route.
R1 is best answered after P0 if R7 is acceptable; if R7 is declined, R1(a) is
unavailable, and R1(b) needs R7′ and R7″, otherwise R1(d) applies.

## 7. Recommendation and main uncertainty

Recommendation, for the operator to accept or reject:

1. Answer R3, R4 and R5 now.
2. Ask the operator directly whether R7 (owning a native fork) is acceptable. If not,
   R1(a) is unavailable. The Pi route under the task host is then the only path to a
   real H2, and it requires R7′ (a Pi realization is not the widening the M4 exit
   excludes, or an amendment of that exit) and R7″. Otherwise R1(d), reporting the
   unmet condition and closing M4, is the roadmap's prescribed outcome.
3. If R7 is acceptable, run P0 ($5–8), a read-only sizing of the Pi realization under
   `lifecycle-service` with C1 consumption sized separately, before choosing. Default
   to the native Codex host. Cost alone never flips this: even at P2's low end the Pi
   route (about $115) costs more than Codex's low end (about $60). Only non-cost
   factors would flip it: no fork custody, Host-side request evidence, and funding
   independent of the Codex quota, weighed against that price difference and against
   R7′ and R7″, which Pi alone needs. The Codex route already has the task-host join,
   the preparation fence, C1 capture and S1 evidence; its remaining work (F, D, G4,
   G5) is smaller and better understood, and its live date is 2026-11-21 at the
   earliest.
4. Do not pursue R1(c) unless the operator wants a cheap feasibility observation and
   explicitly accepts that it does not meet the M4 exit.

Why not switch outright: the Pi route's strengths are in execution evidence and
containment, where Codex is weakest, but the Pi path has no connection to any
research-lifecycle owner, and LC-6 requires explicit owner authority to create a
runtime that bypasses the task-host consumer. The connection (P2) is the largest and least certain piece on either route.

Main uncertainty: the size of P2. `lifecycle-service` operations and actor are about
8 000 lines and `lifecycle-codex` about 8 000; how much of that a Pi realization must
mirror has not been measured. Secondary uncertainties: whether the Pi primary route
shares the exhausted Codex quota (inferred), whether the protection owner accepts
Docker inspection as protected-set evidence, and the per-arm inference cost, which
depends on a model not yet chosen.

## Evidence used

Plans and records: `planning/research-lifecycle-m4-s2-plan-20261117.md` §§1–3;
`planning/research-lifecycle-h2-execution-packet-draft-20261117.md`;
`O/007-h2-artifacts/gate-trace.md`, `snapshot-manifest.json`, `audit-request.json`,
`lc7-protocol.json`; `O/S2-ROOT-ACCEPTANCE.json`; `O/ROOT-ACCEPTANCE.json`;
`planning/dev-vertical-completion-20261115.md`;
`planning/dev-vertical-hosted-gate-contract-20261116.md`;
`planning/dev-vertical-phase4-hosted-transport-r2-20261116.md`;
`docs/dev-vertical-open-items.md`;
`records/dev-vertical-hosted-qualification-20261117/README.md`;
`planning/research-lifecycle-m0-contract-and-first-use-20261111.md` §§7–8;
`planning/research-lifecycle-m4-tool-registry-evidence-plan-20261116.md` §§1–2;
`records/research-lifecycle-m4cb-result-20261112.md`.

Source at `9b289171` (targeted reads): `work-engine-manual-host/src/pi_coding_hosted.rs`
(fetch validation, attempt dispatch, retention), `pi_coding_https.rs` (send),
`pi_coding_provider.rs` (`AttemptRecord`, context bound), `pi_coding.rs` (profile
limits, ready and tool checks, container args), `pi_docker.rs` (container flags,
stdout digest and snapshot), `receiver/pi-coding.mjs` (tool surface);
`pi-pressure-context-host/src/coding.rs` (signatures); `work-engine-server/src/job.rs`
(live class admission); `lifecycle-service/src/lib.rs`, `operations/mod.rs`
(Codex coupling); `lifecycle-codex/src/compiled_input.rs`;
`work-engine-task-host/src/research_delivery.rs`;
`retained-task-workflow/Cargo.toml`; `lifecycle-store/src/native_input.rs`; the
`Cargo.toml` dependency lists of the crates named in section 1.

Not done: no build, test, run, inference, credential use, install or commit. S2 loci
were not re-verified at `9b289171` except where cited above. Pricing for the live arms
is not read from a catalog.

## Review disposition

Review: `O/012-route-review/REVIEW.md` (accept with required changes). Repair
packet: `O/013-route-repair/PACKET.md`.

| Finding | Change |
| --- | --- |
| F1 | Cited the roadmap M4 exit (lines 228–230) in the short answer. R7′ is now the operator decision that a Pi realization is not the excluded widening (or an M4 exit amendment), symmetric with R1(c). The recommendation, R1, the section 6 closing text and section 7 step 2 now say: the only path to a real H2 requires R7′, otherwise R1(d) is the roadmap's prescribed outcome |
| F2 | R4 now includes per-task execution and inference grants (exact scope and budget; Codex G6, Pi via R8′) and the audit-result acceptance owner |
| F3 | The readiness half of D6 is kept in R2 as Codex-only under R2(full) and retired under R2(narrow), citing S2 (D1(a) route A; next slice lists D2 and D6 only for D1(a)). R6 keeps the funding and model half |
| F4 | Added R8″ (Pi only): approve the analysis tool list and accept Host `tools`-digest prevention plus receiver/image pin as the S1-equivalent claim; section 2(b) points to it |
| F5 | Added R7″ (Pi only) and cost row P-Q ($5–15, re-review of `7c103ad2`); Pi total now about $115–255. Line 61 corrected: the record is "not owner acceptance"; `owner_pending` is per-job output policy |
| F6 | Pi earliest-live date stated as not estimated, with the sequence; the advantage is restated as funding-source independence in the short answer, section 2(h), the section 5 total and R6 |
| F7 | "LC-6 forbids" replaced by "requires explicit owner authority" (section 3 item 3, section 7); added the LC-4/LC-5 and exit reasoning and one sentence on why item 2 keeps the consumer seam |
| F8 | Added a C1-consumption sizing note to section 2(a) and to row P0; removed the 4-request-limit advantage |
| F9 | Section 7 step 3 now states cost never flips the default and names the non-cost factors. Row D low end is $0–18; the Codex total becomes about $60–135, and the text on the ~$64 remaining budget (section 5, R5) is adjusted to match |
