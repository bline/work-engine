# Rust review-episode R0 characterization and target contract

Status: R0 characterization, 2026-10-07. This document is subordinate to the
[reviewed implementation plan](rust-review-episode-implementation-plan.md)
(`SHA-256 dd17594b61ca7cc70f853b6c92024ffe1472006e4c8ac2530b63df513297515d`).
It freezes executable observations and selects the proposed migration contract.
It does not authorize R1–R5, change production semantics, accept a claims
contract on another owner's behalf, or qualify a cutover.

## Evidence and fixture boundaries

The four [R0 fixtures](../app-server/tests/fixtures/review-episode-rust) are
static JSON oracles. The [characterization test](../app-server/tests/services/review-episode/rust-contract-characterization.test.mjs)
executes the current JavaScript episode, implementation-review, claims,
native-review closure and campaign services against disposable SQLite files.
The fixtures were generated from the current JavaScript codec/service once;
the test compares live output against frozen bytes and revisions. No external
provider, live database, migration, Rust executable, live campaign operation or
real external effect is used; reviewer behavior is synthetic.

| Fixture | Frozen evidence |
| --- | --- |
| `codec-v1.json` | Exact canonical bytes and SHA-256 for nested objects/arrays, escapes, UTF-16 key order, BMP/supplementary and lone-surrogate strings, safe-integer bounds, missing versus null. |
| `states-v1-v2.json` | Six-part identity key, v1 begin and v2 blocked/succeeded complete canonical states, revision hashes and history order. The revision is the digest of semantic state without its `revision` field; stored JSON contains the field. |
| `commands.json` | Begin, result, uncertainty and replacement revisions/history, plus command sequence names for replay, conflict and CAS tests. |
| `semantic-deltas.json` | Old behavior and selected future dispositions kept in distinct fields. Future entries are design decisions, not a claim that the current service passes them. |

The codec identifier for this domain is `review-episode-js-json-v1` pending S0
namespace/interface agreement. It recursively sorts object keys by JavaScript
UTF-16 code units, uses JavaScript `JSON.stringify` scalar/string rules and
does not append a newline. In particular U+10000 sorts before U+E000. The
claim-evidence codec has different bytes and must not replace this one.
Compatibility representation for R1 is a JSON string/key value carrying exact
UTF-16 code units (including unpaired surrogates), rather than a Rust `String`
that silently changes them. A parser/serializer may use a raw escaped token
internally if it round-trips to the same code units and canonical bytes. Until
that path exists and is tested, an imported row containing an unsupported
unpaired surrogate is refused/quarantined with its source bytes preserved;
replacement-character normalization and revision regeneration are forbidden.
Non-JSON/undefined values are outside validated episode state, so these fixtures
do not extend the codec to arbitrary JavaScript objects.

## Selected authority and admission contract

The existing native App Server host remains the trusted composer. It derives
the selection and authority from the campaign, pins the reviewer runtime
session, runs the implementation-review v1 validator and asks the claims owner
for required production-path evidence. A Rust request is admitted only through
that host-bound path. The Rust application revalidates the deterministic
result/subject/finding contract and current episode CAS; its typed values do
not establish caller trust. A public CLI callable with an `authorized` boolean,
an arbitrary claim status, or an attacker-chosen database root has no admission
authority. The actual launcher/file access boundary must exclude the review
grant from writable episode files and privileged invocation before R3
integration can qualify; R5 remains the operational cutover gate.

An admitted command binds the exact episode identity, selection revision,
authority source/manifest digest, writer actor/provider/generation and runtime
session, expected state revision, selected subject digest, transition ID and
content digest. A result command also binds its result digest. Required
production-path evidence is checked against claim-owner records, including
claim ID/revision and subject (candidate plus review episode), covered state,
consumer and consumption boundary, establishment ID/status, observation ID
and digest, the observation's attempt/result digest and immutable reference
digests. The host cannot satisfy these checks by forwarding a caller-supplied
reference or status. A trusted claims-owned correction handoff may describe
not-yet-committed successors, but those refs are not represented to a
consumer as durable established evidence until the claims correction is read
back. Missing owner verification fails the dependent operation closed.

Required claims in the existing v2 native selection apply to the *admitted
native review result* and its exact candidate/episode subject at the builder
and campaign terminal boundaries. The observation also binds the result digest.
They do not grant reviewer independence, result acceptance, or campaign
acceptance. An established admission for result A cannot establish result B,
even when a retained reviewer uses the same episode or candidate: a different
result digest needs a new claims-owned establishment and a new exact admission
before dependent projection or terminal consumption. If the candidate changes,
the new claim subject is also required. Previous admissions remain immutable
history. This is the R0 selected target for correction/remediation. Claims and
campaign owners must accept and integrate its affected routes in RC before
they qualify; the evidence scope itself is no longer an open R0 decision.

The compatibility adapter remains synchronous for `begin`, `resumeInitial`,
`transition`, `validateResult`, `recover`, `read` and `history`. It preserves
result-contract errors separately from store/transport errors. On no reply or
broken transport after possible commit, it reads exact durable transition and
history before retry; it does not enter the reviewer provider or fall back to
the JavaScript writer. It must frame/version/size bound requests and responses,
correlate operation IDs and fail explicitly on oversized history. The host's
responsiveness budget and launcher access boundary remain R3 qualification
gates, not claimed measurements here.

## Semantic deltas and owning gates

| Case | Current observed behavior | Selected target and required correction gate |
| --- | --- | --- |
| Matching replay | `transition` and `begin` return the latest state before checking expected revision or current writer. A superseded writer can observe that state via a matching command replay; a new write remains fenced. | R1 command semantics reject superseded-writer replay at the command interface. An independently authorized reader may still read history. R3 host authenticates/authorizes before exposing any replay state. This is a deliberate compatibility delta; RC must resolve any legacy recovery caller that relies on the old shortcut, while retaining idempotent replay for a current writer. |
| Evidence succession and questions | `record_result` keeps an acceptable result with unresolved questions pending, but `succeed_evidence` changes phase to `reported` and pending action to `return_review_result_to_builder` when its two successors are established. The question array remains. | Preserve questions as unresolved work: with an acceptable verdict and nonempty questions, succession remains in `remediation` with `await_remediation` (or a separately accepted equivalent nonterminal phase/action). R1 may encode the target; RC/native consumers must accept its downstream meaning before affected operations qualify. Historical hashes are not rewritten. |
| Later result evidence | A v2 `record_result` without `evidenceAdmissions` carries prior admissions into a changed subject/result. Correction/remediation paths currently omit `admitProductionPath`. | Keep old admissions for lineage but mark them inapplicable to the new result at dependent consumption. R3/RC must obtain claims-owned admissions for the exact new result and required subject or leave its builder/campaign consumption blocked. Claim, observation and consumption refs remain exact; no status-only shortcut. |
| Correction join | Claims `correct` calls episode succession before claim-store correction commit; campaign publication follows. Separate SQLite transactions can leave three different durable positions. | RC supplies bounded operation-ID reconciliation at the episode/claims/campaign join. A consumer verifies the exact three-way lineage before enabling a required claim. Incomplete or conflicting joins remain unresolved, with no inferred acceptance, rollback of committed review truth, provider replay or generic distributed transaction framework. |

Two controlled local tests characterize the separate transaction cuts. The
first invokes the real claims correction owner and injects an exception after
its episode callback commits but before the claims transaction. The episode
has four admissions and reports, while claims has only two predecessor
establishments and no succession. Repeating that exact claims operation reuses
the episode transition and commits the two successor establishments and
succession. This case deliberately does not construct a campaign succession.

The second invokes the actual `campaign.succeedReviewSelection` composition
with real episode, claims and campaign SQLite stores, an accepted v2 selection
and a controlled synthetic reviewer result. The campaign store write is
interrupted after the closure has committed episode succession and the claims
owner has committed its correction. The readback binds the claims succession
to the predecessor campaign revision and selection, predecessor and successor
episode revisions, successor selection and two establishment records. The
campaign still holds its original selection, evidence-unestablished episode
binding and revision, with no recorded selection succession. The synthetic
reviewer was entered once. These tests establish the two existing windows and
the need for reconciliation; they do not demonstrate a Rust process crash,
successful campaign retry, future correction implementation, or cutover.

The bounded reconciliation rule is keyed by operation ID, episode identity and
predecessor/successor revisions, both claim IDs and revisions, establishment
IDs, observation digest, selection revisions and campaign predecessor revision.
At restart, read all three owners and compare those exact links. If only the
episode successor exists, verify its immutable result and claims-owned
prepared correction inputs before resuming the claim commit; keep dependent
consumption closed. If episode plus claims correction exist but campaign does
not, publish campaign succession only after its expected revision and selected
claims still match. A conflicting campaign CAS, changed selection, missing
observation or mismatched claim/result leaves the join unresolved for the
owning operator route. A repeated operation returns the existing exact
successor; a conflicting use of the ID fails. Consumers must not rely on
episode phase alone during this interval. RC must make this rule executable
and qualify each interruption/restart cut before R4 or full R5 cutover.

## R0 exit and later prerequisites

R0 freezes the JavaScript oracle and the target decisions above, without
claiming a Rust implementation. R1 still requires S0 acceptance of value
interfaces and codec namespace/placement. R2 needs the exact v1/v2 codec,
SQLite import integrity and process/CAS tests. R3 needs trusted host admission,
claim-owner verification, synchronous bridge behavior and measured latency.
RC owns affected replay/question/claim-scope/join corrections with their
episode, claims and campaign owners; R4 cannot qualify affected routes until
those gates are met. R5 additionally needs selected roots, operational
authority, one-writer fencing and readback/rollback evidence. The separate
Python/Git-backed independent-review-state owner and its MCP consumer are
outside this migration and remain live.
