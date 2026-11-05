# Synthetic task and answer owner boundary v1

This source candidate provides callable Rust projection, signed-answer consumption
and exact historical reads over a real registered task/proposal store and the
canonical board. It does not construct any production/live owner or enable an
admission, capacity claim, outbox, native operation or action-bearing approval.
Independent acceptance and deployment qualification remain separate.

`owner_boundary::SyntheticOwner::open_synthetic` takes backend-owned configuration
without Deserialize: database, repository, executable/dependency pins, actor,
owner, session, RP/origin, credential, expected key fingerprint, synthetic epoch
and optional existing scoped-control configuration/profile pins. Consume and
project input cannot change these. The owner implementation and its durable
adapter and registry contract must match the exact source compiled into Rust.
The fixed Python surface accepts snapshot or conditional-reply JSON only; it
never executes caller code or writes board payloads from Rust. Pin checks are
fixture integrity checks, not deployment isolation or a hermetic build claim.

The policy is explicitly SYNTHETIC, hash pinned and loaded at construction. Its
owner, RP, origin, credential/key fingerprint and expiration must match. Missing,
changed, wrong-key or expired construction policy fails. Loaded policy bytes
remain immutable for that boundary instance. File changes/removal afterwards do
not implement mutable key revocation. No concurrent live policy replacement,
protected passkey identity, real human presence, authentication service or live
admission is qualified. The synthetic epoch advances through monotonic elapsed
time, including SQLite and subprocess waits; a newly constructed fixture chooses
its epoch again and is not a persistent production clock.

## Public callables and compatibility

- `project(task_id)` reads the actual task/proposal heads, generations and exact
  registered request, and matches it to a complete canonical board revision. It
  reports current, held, replaced, revoked, expired or unavailable standing plus
  exact references, request bytes/digest, owner-source and policy identities.
- `consume(Consume { selection, signed })` holds the existing SQLite IMMEDIATE
  transaction through registry validation, verification and conditional board
  CAS. All five signed strings are preserved exactly. The board publication is
  the only durable consumption point. No SQLite decision journal or receipt is
  added. The transaction may roll back after board success without undoing it.
- `read_exact` reconciles a stable answer UUID against immutable row bytes in a
  complete board revision, exact registered request/task/proposal bindings,
  generations, raw answer and all assertion strings. It re-verifies cryptography
  at signed creation time for history, separately evaluating current standing.
  Absence is unknown, never proof of no publication. Changed ID reuse conflicts.
- `register_*_fixture` are explicitly synthetic backend producer helpers using
  the existing registry owner's CAS and sticky withdrawal rules. They confer no
  signing or execution authority. Real authenticated coordinator registration
  remains with `service::coordinator`.

Selections are observations, not authority tokens. Stale board revisions fail
without automatic rebasing. Exact historical retries return the same row-derived
decision identity after restart, expiry, supersession or revocation and never
assert a renewed current grant. Admission is always null. Historical reads are
not independent attestation that all current eligibility checks held when an
arbitrary ordinary board writer published a signed row.

The request codec is the established owner-answer/v1 codec: compact sorted JSON,
UTF-8, ensure_ascii=False over the **full canonical row**, including timestamp,
sequence and envelope fields. The board owner produces those bytes; Rust does
not substitute pretty delivery slices or new JCS. The original registered bytes
must equal that representation. The accepted 30-vector golden file remains
unchanged. `proof::verify_answer` handles all decisions; the existing execution
verifier `proof::verify` still accepts approval with empty comment only.

Approve/answered and reject/declined close a request. Defer/deferred and
ask/partial stay open. Outcomes, exceptions and observed results are different
concepts. Discussion, silence, projection views, agent JSON, storage credentials,
role and `data.verified` are never substitutes for the signed proof.

## Currentness and controls

Registry heads/generations and task deadline are checked under IMMEDIATE. Board
closure/supersession and opaque revision are validated by the board owner before
CAS. Proof, task, policy and applicable control expiry constrain a monotonic
publication deadline; acquisition waits cannot reset that deadline. Source
subprocesses have fixed arguments, bounded runtime/output and collected exits.
Explicit conditional rejection is conflict/held; uncertain process/ACK failure
after publication entry is unknown. Never replace an answer ID or automatically
retry an unknown publication to manufacture a fresh effect.

For scoped tasks, backend configuration binds the existing signed control owner
and profile. Its head verification and `synchronize` run under the held registry
transaction; a second canonical revision check and final CAS fence board changes.
Persisted sticky revocation is checked too. The fixture's default disposable
profile has no scoped control; other profiles without a binding are held. This
adds no control state machine and does not change current admission behavior.

Time checks occur immediately before invoking the durable owner's CAS. They do
not establish an atomic wall-clock predicate inside Git itself or a hard real-time
scheduling guarantee. Source/file pins likewise do not establish protection
against a hostile replacement during execution. These are synthetic qualification
limits for independent review, not live deployment promises.

## Executable evidence

Run offline with an isolated target:

```
CARGO_TARGET_DIR=/tmp/private-owner-target cargo test --offline -p session-task-admission
cargo clippy --offline -p session-task-admission --all-targets -- -D warnings
python3 -m unittest discover -s skills/durable-state/tests -p test_codex_chatboard_contracts.py
```

`tests/owner_boundary.rs` creates private Git and SQLite stores, produces actual
canonical request bytes and cryptographic assertions, and exercises all four
decisions, exact retries, competing closes, wrong key/tamper, withdrawal,
supersession, actual SQLite lock waits/expiry and existing signed scoped-control
revocation. It executes a child consumer that loses its answer ACK through a
process kill, then reopens and reconciles without a replacement ID/effect.
The board tests crash a process immediately before and after actual durable
publication, and exercise real CAS conflicts and exact concurrent candidates.
The consumer crash test is before consumption or after consume returns but before
client ACK; it does not claim an injected Rust crash at every instruction inside
publication. Together these tests cover actual store/process behavior at their
stated boundaries, not an exhaustive distributed crash model.

`examples/owner_boundary_snapshot.py` is a fixed read-only synthetic source
adapter for the existing scoped-control fixture. It invokes the canonical board
owner's explicit snapshot surface and is not an installed/live publisher.
With `OWNER_BOUNDARY_FIXTURE_DIR` set, the joined test writes four labelled
synthetic projection/input/decision examples suitable for offline AH adapter work.
They are observations of disposable stores, never live authoritative snapshots.
