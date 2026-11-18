# D3 design review: a real read-only review on the hosted Pi path

Reviewer: Fable 5.2, read-only, fresh context, for Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. Subject: `d3/DESIGN.md`.
Repository HEAD when read: `b3577d7a` (the design read `3c1fc206`). Source paths are under `rust/crates/` unless rooted.

**Verdict: `proceed_with_changes`.** The route is sound and its citations hold: more than 40 line citations checked against
source, one misplaced (body retention is README:86-89 and :128-129, not :74-76). Two P1 changes belong in the H0 and H1
packets before build spend, and OQ-1 needs rewording before it reaches the operator.

**Not established by this review** (`git cat-file`, `git diff` and the chatboard script were denied in this session): the
board state; a context reading (`usage_unavailable`); whether commit `96808de3…` exists (no loose object under
`.git/objects/96/`, packs not searched); drift of the cited crates between `22487ec6`, `3c1fc206` and `b3577d7a`;
gpt-6-luna's `full_reservation`; whether the docker inspect document is retained per run; the text of the hosted-review D6.

**The four checks.** (1) Route: truthful on the hosted side (no claimed crate changes; the binary checks its own digest,
`analysis-qualification.rs:78-87`), but not minimal (P2-5); the binding bypass drops three checks (P1-2); the result file can
be read as admitted (P2-2). (2) Realization: correct for the provider; for confinement the truthful claim is unstated, which
gives `Unestablished`, not `KnownIncompatible` (`realization.rs:83-97`). Option A is truthful only as route qualification
(P2-1). (3) Claims: claim 1 is checkable (P3-3); claim 3 is narrower than `mutation: none` and one check cannot fail (P2-3);
claim 2 can hold with invented evidence digests (P3-1). (4) Cost: fits $50 only without a second review round (P2-7).

## P1

**P1-1. The 700000 cap limits the request count, and H0(d) tests the wrong inequality.** Admission refuses when
`full_reservation × max_requests > budget_microusd` (`pi_coding_provider.rs:419-427`, called at `:492`). DESIGN:194 and :207
test only the per-request rule (`:635-645`), and DESIGN:72 presents 32 requests as available. Under 700000 a job gets
`floor(700000 / full_reservation)` requests. The coding qualification ran `max_requests: 4`
(`records/dev-vertical-hosted-qualification-20261117/qualify.py:98`; its budget value not read) and analysis runs used 8 with
5 000 000 (`analyze.py:23-25`). A reviewer that reads one file per turn then fails with "provider request outside its
admitted count" (`analysis.rs:215-222`) after spend. **Fix:** H0(d) computes N and stops if N < 4 (live-1 used 3, n=1); the
job sets `max_requests = N`; the template states the request and read limits and asks for all reads in one turn. Else OQ-3.

**P1-2. The binding and gate are bypassed, and their three checks are not re-created.**
- (a) *Refusal on known incompatibility.* The gate refuses before spawn (`work-engine-review-delivery/src/lib.rs:20-25`).
  The adapter must refuse to write a job when any blocker has class `KnownIncompatibility` (`prepare.rs:611-627`), with a
  test "frozen candidate + hosted claim → refused, no job". That record is what makes "the frozen world stays the reported
  negative" (DESIGN:27) an artifact.
- (b) *Realization claim.* The binding refuses a caller-supplied claim because the host derives it (`binding.rs:204-212`).
  DESIGN:32 passes a "hosted realization claim" without values. The profile grants two tools (`pi_coding_analysis.rs:9`), so
  a truthful claim names them, and mutation and capabilities are then `Unestablished` in every world
  (`realization.rs:134-138`, `:171-175`): two more blockers per condition than the R2-S run had. R2-S's no-tools claim would
  be false here. **Fix:** one constant (provider as the Host enforces it, `openai-codex`, `pi_coding_provider.rs:628`;
  confinement unstated; `ReadOnlyClaimed`; both tools; `fresh_initial`) that the checker compares with the retained bodies.
- (c) *Canonical recomputation.* "Freezing never establishes canonical derivation" (`lib.rs:17-18`). The checker must rebuild
  candidate → `prepare` → `freeze` from the retained requirements, request and transport and require the same `contract_sha256`.

## P2

- **P2-1. Option A's label does not carry over as written.** DERIVATION.md:21-26 justified dropping the constraints for a
  provider-disabled child, where the provider constraint bound nothing. On a live OpenAI route it removes a requirement the
  run violates. The delivered contract still carries "fresh guarded Claude reviewer … verified execution-evidence custody"
  (`requirements.json:27`, `:59`; `prepare.rs:431`) and run `production-qualification-20261107` (`prepare.rs:395`). A
  therefore qualifies the route and no condition of either world, the same standing as B, yet only B gets that label
  (DESIGN:221-223). Narrowing needs the condition owner's authority (evidence-and-claims §3.9, lines 25-28 and 49-57).
  **Fix:** reword OQ-1 (below); put both preparations, frozen (refused) and chosen world, in `check.json`.
- **P2-2. The result file can be read as an admitted review.** DESIGN:150 stores the answer bytes as `review-result.json`: a
  bare schema-valid object for the real frozen subject, possibly `acceptable_as_is`, and `exact_fields` leaves no room for a
  standing label (`implementation_review_v1.rs:36-47`). **Fix:** keep the answer only as a string inside an envelope, as
  `analysis-result.json` does (`analysis.rs:293-302`), with fixed fields: `standing: owner_pending`, `org_admission:
  none_requested`, `consumable: false`, world label, requested model, `proposition: known_unmet`, undischarged blocker count.
- **P2-3. Claim 3 and its evidence.** `git rev-parse <commit>^{tree}` (DESIGN:160) cannot fail, because a commit id fixes
  its tree; remove it. The subject is immutable, so "no route to mutate the subject" (DESIGN:172) is narrower than
  `mutation: none`, which covers any state the realization may write (`realization.rs:120-122`), and `/export` is a writable
  bind (`pi_coding.rs:607`). **Fix:** claim what is shown (two Host-served read-only tools, zero tool requests, containment
  as the Host verified it at run, `pi_coding.rs:637-724`, the export listing) and state that `mutation: none` is
  unestablished. Name the per-run artifact that carries the containment check; DESIGN:153-154 cites code.
- **P2-4. The binding size limit is the request context, not the snapshot.** H0(a) compares against 1 MiB and 256 files
  (DESIGN:71, :194). The Host refuses a request whose context exceeds 400 000 bytes (`pi_coding_analysis.rs:16`;
  `pi_coding_provider.rs:629-633`), and the snapshot holds each change twice (file and patch). In run set a3, 158 961
  snapshot bytes gave a 191 527-byte context and 45 812 gave 57 184 (README:98-99; ratios 1.20 and 1.25, n=2).
  **Fix:** H0(a) stops when snapshot bytes × 1.25 + prompt > 400 000.
- **P2-5. The job writer is not minimal.** It duplicates `analyze.py:41-112` and makes our crate a second producer of three
  dev-vertical schemas, including the runtime admission (`allow_provider`, budget, routes) on which Host authority rests
  (open-items:61-66); job preparation for this profile is an open dev-vertical item (open-items:36-38). `analyze.py`
  already takes a snapshot directory and task file (`:4`, `:116-117`); only `BUDGET` is fixed (`:25`). **Alternative:** the
  crate renders, materializes and checks, and the job comes from `analyze.py` with a one-line budget override asked for in
  DQ-3; a3, a4 and live-1 exercised that path. If the writer stays: list the self-authored runtime admission under
  Unclaimed, and take the expected binary digest from DQ-2, refusing on mismatch (hashing the file at the path is not a pin).
- **P2-6. OQ-2's fallback creates a third world.** `prepare` does not relate `scope.digest` to the subject
  (`prepare.rs:529-535`); the frozen subject's `patchIdentity` equals it (`requirements.json:17`;
  `work-engine-review-delivery/tests/common/mod.rs:77`). **Fix:** if H0(a) or (b) fails, stop; another commit
  (DESIGN:229-230) needs requirements with a new `scope.digest` from their owner.
- **P2-7. The reserve is overstated.** The table sums to 36.6 as stated. With D3 at 3.3 and this review (target 3.5, cap 5)
  it is 37.9–39.4, and root coordination for H0–H2r and a re-check after H2r are not priced. The reserve is at most
  10.6–12.1, not 13 (all `unverified` estimates; no node actuals read). **Fix:** fold H0 into the root or H1, apply P2-5,
  decide DQ-1 before H1, run L2 only if L1 passes, and replace the single $45 stop with "H3 does not start above $42",
  which keeps H3 + H4 (5.6) inside $50.

## P3

- **P3-1.** Parse checks only the form of evidence digests (`implementation_review_v1.rs:143-157`). Report grounded k of n in
  claim 2; mark ungrounded decisive evidence `evidence_ungrounded` for the operator.
- **P3-2.** "Same contract, new job id" (DESIGN:198) cannot hold: `attempt_id = job_id` is inside the envelope
  (`lib.rs:148-152`). Say "same preparation", assert it, and count every live job in the record (k of n).
- **P3-3.** Delivery check: body sha256 equals the attempt's `request_body_sha256`; exactly one user text item; its decoded
  text starts with the task bytes (`AnalysisConfig::prompt` is private, `analysis.rs:42`, so do not copy the suffix). Word
  claim 1 as "in the body the Host retained as dispatched"; receipt by the live provider is not independently observed.
- **P3-4.** Make `.review/patch.diff` the exact bytes hashed for `patchIdentity`; its manifest digest then equals the subject's
  `patchIdentity` and `scope.digest`, which the checker can assert.
- **P3-5.** The workspace lists members (`rust/Cargo.toml:3`, `:55`): the crate edits that file and the lockfile, absent from §2.
- **P3-6.** H0(d) needs the catalog under `~/.local/state/work-engine` (`analyze.py:18`, `:21`), which read-only packets
  exclude. Give it to the root or builder; the H1 packet must name those paths and Docker.
- **P3-7.** DESIGN:181 cites "D6" by label; the D6 in the org records I read is the S2 storage decision
  (`records/org-pi-slices-20261116/s2/design/PROPOSAL.md:291`). Quote the operator's text and date in the record.

## Operator decisions

**OQ-1 (reworded).**
> The frozen conditions require provider_family anthropic and execution_confinement guarded_sandbox. The hosted path runs
> gpt-6-luna (openai-codex) in a container that no owner maps to a confinement class. As owner of these conditions, choose.
> Every option is route qualification only: no condition is established and nothing is consumable.
> (A) Run the R2-S assumption world (frozen requirements without realizationConstraints). This is a new use: it was approved
> for a provider-disabled child. Its conditions still name a guarded Claude reviewer and the run
> production-qualification-20261107; the record marks both as not met, beside the frozen world's refusal.
> (B) Run the frozen candidate with its known-incompatibility blocker inside the delivered contract.
> (C) Defer to a Claude realization. It fixes the provider only; confinement stays unmapped.
> (D) Issue a D3 requirements revision: same obligation and scope, no realizationConstraints, a proposition and run id that
> describe this route. Unchanged compiler; one more fixture and derivation test.

Recommendation: D if the operator will sign the proposition text; otherwise A with P1-2, P2-1 and P2-2. Not B: it runs a
condition known to be unrealizable as if it were executable (§3.9).

**OQ-2 (add).** "The diff and changed files of that commit are sent to the provider. If the commit is absent or over the
bounds, D3 stops; another commit needs new requirements."

**OQ-3 (new).** "The 700000 µUSD cap admits N provider requests (H0 gives N). If N < 4, may review jobs use a higher proxy
budget on the analysis profile? live-1 ran with 5 000 000 and settled 11 029 µUSD over 3 requests (n=1)."
