# Hosted profile: lifecycle gate failover contract

Prepared 2026-11-16 by the lifecycle gate owner (Pi session
`01a1e2a4-f353-743a-ba23-f42394958125`, owner by operator direction; board
seq 5772). Standing: the owner's implementation contract for piece (b) of the
[phase 4 design r2](dev-vertical-phase4-hosted-transport-r2-20261116.md). It
is not independent review or qualification.

## Operator decisions this contract implements

- The hosted profile is built on the lifecycle bridge composition (option A),
  not on the signed subscription profile. The lifecycle gate, not `PiOwner`,
  fences provider entry.
- The lifecycle grant must cover the permitted failover policy. A grant for
  one logical request must not silently become permission for unlimited
  provider dispatches or unauthorized payer changes. The grant binds the
  allowed routes and payers, the attempt limit and the deadline, and
  currentness is checked before each dispatch.

## Contract

1. **Policy.** `HostedProviderPolicy { routes: [(route, payer)],
   max_attempts_per_request }`. Routes and payers are non-empty, bounded and
   unique; the attempt limit is between 1 and 8. Its digest is SHA-256 over
   its canonical JSON.
2. **Grant binding.** In hosted mode the gate claims its turns (including
   the successor's rehydration turn) under grant ids that carry the policy
   digest: `grant:old:pi-hosted:<digest>` and
   `grant:new:pi-hosted:<digest>`. The trusted issuer must install exactly
   those grants. A different policy needs a different grant, and revoking or
   expiring the grant ends the policy. The lifecycle store's grant schema is
   unchanged.
3. **Deadline.** Every attempt's deadline is the minimum of the current
   grant's `expires_ms`, read inside the store transaction, and the
   configured `expires_ms`.
4. **Per-attempt entry.** `CodingEffectGate::provider_attempt(intent,
   dispatch)`. Each call:
   - re-runs the budget and expiry check;
   - requires hosted mode and the logical request most recently admitted
     by `coding_provider_request`, with its grant command already
     dispatched;
   - requires attempts in strict sequence, within the limit, on a bound
     `(route, payer)` pair;
   - keeps the admitted attempt (intent and policy digest) for the
     continuity report. It writes nothing into the turn's ingress, so the
     settled Pi event grammar is unchanged; the Host ledger durably records
     every attempt before its dispatch;
   - runs `dispatch(deadline)` inside `with_pi_coding_turn_send`, so the
     parent turn's grant currentness (not revoked, not expired, current
     revision, active ownership) holds while the dispatch starts. The
     attempt is consumed even if dispatch fails, and is never replayed.
5. **Scope.** The gate does not interpret outcomes. Whether a next attempt
   may follow (only after a definitive refusal) and every release are
   decided by the financial owner, the provider ledger in
   `pi_coding_provider.rs`.
6. **Existing behaviour.** The trait's default refuses hosted attempts.
   Configurations without a policy keep their grant ids and behaviour.

## Lifecycle kind

The hosted profile reuses the lifecycle store's `Live` profile and protocol
digests. Those digests describe the Pi event grammar, Host intent/result
protocol and checkpoint derivation, which hosted transport does not change.
Hosted turns are distinguished by their policy-bound grant ids. If an
acceptance owner requires a separately versioned lifecycle kind, it is an
additive `PiCodingKind` change.

## Amendment, 2026-11-16 (offline hosted Docker run)

The first offline Docker run of the hosted profile found two defects in the
original implementation (9d24449c). Recording `pi.provider_attempt` in the
turn ingress made every hosted turn's settlement fail closed, because the
settled event grammar has no such event. The successor's rehydration turn
also used the bare `grant:new`. Both are corrected as stated in items 2 and 4
above; a full hosted flow test now settles all four turns.

