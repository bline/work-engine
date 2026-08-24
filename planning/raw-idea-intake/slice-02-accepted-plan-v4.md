# Slice 2 accepted plan v4: Common-Root Formation Validation

- Run: `5a74bd5c-3ebc-4e6e-9d93-07dd96aede5a`
- Slice: `2`
- Proposed active attempt: `attempt-5`
- Plan version: `accepted-v4`
- Status: `accepted_v4`
- Acceptance: `procedural_auto_approval`
- Workflow route: `falsified-placement`
- Placement risk: `medium`
- Placement verdict: `confirmed`
- Repository mutations during reopened planning: this planning artifact only

## Binding and preserved authority

This artifact is a tightly bound revision of
`planning/raw-idea-intake/slice-02-accepted-plan-v3.md` at SHA-256
`2f084590cee7efcdfb020e8b5d4f16d6d58f29a36bb43d545cdc887971fbf510`.
That artifact is itself bound to
`planning/raw-idea-intake/slice-02-proposed-plan.md` at SHA-256
`4c0b2b231fca236147c9e11f7b2ead01a646bf55ac5ff42ddb1e26181fdae0d9`.
Together, the exact v2, v3, and this artifact define accepted plan v4. Only the
validator-root route revision below replaces v3 state. The accepted goal,
placement certificate, two baseline overlaps, eight fixture paths, invariants,
validation consequences, deferred scope, and stop conditions remain unchanged.

The original human approval remains bound through
`planning/raw-idea-intake/slice-02-plan-approval.md` at SHA-256
`cd84a346589de2573e19cb4a90b90540cc105a16510bf9472b4e150bdc89925b`.
The implementation authority remains
`proposals/idea-to-proposal-system/raw-idea-intake/implementation-authorization.md`
at SHA-256
`8ab9f94c2ccdad6c4aaaf43570e7d01d3f5edb56c32331daef8bf3aae96da7fa`.
The campaign remains `campaigns/raw-idea-intake.yaml` at SHA-256
`7983910fd3e0faffc5b37377a344fd6193ef42858d8f0b5cfe02b015b9bcb62c`.

The supervisor procedurally accepts this route correction because it changes no
product consequence, semantic ownership, authority, or path boundary. It only
aligns the validation invocation with proposal-packets' existing containment
contract and preserves the original human authorization provenance.

## Preserved exact goal

Exercise a fresh proposal former using only one owner-produced bounded intake
projection and its named evidence. Preserve truthful zero-, one-, and
multiple-proposal outcomes, validate every surviving proposal through the
existing packet contract, publish complete formed meaning at an exact immutable
Git checkpoint before review, and prove a fresh consumer can recover the
material proposal meaning without the raw idea collection or formation
conversation.

## Preserved confirmed placement certificate

- **Trigger:** An authorized fresh proposal former receives one validated
  bounded intake projection and only evidence named by that projection.
- **Producer:** `skills/proposal-former` challenges, declines, preserves, or
  splits surviving candidate meaning and authors the terminal semantic outcome.
- **State owner:** For surviving candidates, proposal manifests own stable
  identity and closed lifecycle metadata and their narratives own formed
  meaning. When no candidate survives, an attributed durable formation
  disposition distinguishes semantic completion from absence or failure without
  creating proposal identity or a second closed runtime owner.
- **Consumer:** Proposal-packets validates each surviving packet mechanically;
  a fresh evaluator/reviewer consumes the exact immutable checkpoint revision.
- **Lifecycle:** Validate and project one immutable intake subject; form zero,
  one, or multiple outcomes; validate every surviving packet; create and
  validate the exact checkpoint; revalidate archived packet bytes; only then
  begin semantic review.
- **Semantic consequence:** A fresh consumer can distinguish no proposal from
  one or several independently decidable formed candidates and recover their
  material meaning without rereading the raw collection, reconstructing the
  intake conversation, or treating intake judgment as proposal authority.
- **Downstream proof:** The Git-backed vertical consumes the actual owner-built
  projection, proves all three cardinalities, resolves every packet origin to
  the projection/source identity, validates packet repositories, and proves
  fresh-consumer usability from the exact archived checkpoint subject.
- **Insufficient substitute:** A static projection lookalike, raw-idea-only
  fixture, producer-only handoff, fake packet for zero, packet validation
  without formation-input binding, mutable working-tree review, or a new schema
  whose consumer and ownership necessity have not been demonstrated.

## Route revision from accepted v3

- **Failed premise:** Proposal-packets can resolve a shared parent projection
  when invoked with each outcome's nested `packets` directory as repository
  root.
- **Stale decisions:** The two per-outcome direct and archived command
  identities and the vertical's use of those nested roots.
- **Preserved evidence:** The exact owner-built shared projection and equality
  proof, cardinality fixtures, packet semantics, zero disposition, archive
  consequence, and absence of any new runtime, schema, owner, or path expansion
  remain applicable.
- **Replacement route:** Invoke proposal-packets once at the common
  `skills/proposal-former/tests/fixtures/intake-formation` root, recursively
  validating all three surviving packets while zero remains packetless. Assert
  the expected outcome-specific packet IDs and locations separately in the
  vertical.
- **Reason:** Validator containment is relative to the declared root, so the
  common semantic fixture root is the smallest root that truthfully contains
  both the shared input and all outputs.

## Preserved exact task-owned path set

Baseline overlaps:

- `skills/proposal-former/references/formation-contract.md`
- `skills/proposal-former/tests/test_proposal_former.py`

Fixture manifest:

- `skills/proposal-former/tests/fixtures/intake-formation/projection.json`
- `skills/proposal-former/tests/fixtures/intake-formation/zero/disposition.md`
- `skills/proposal-former/tests/fixtures/intake-formation/one/packets/cache-invalidation/packet.json`
- `skills/proposal-former/tests/fixtures/intake-formation/one/packets/cache-invalidation/proposal.md`
- `skills/proposal-former/tests/fixtures/intake-formation/multiple/packets/cache-invalidation/packet.json`
- `skills/proposal-former/tests/fixtures/intake-formation/multiple/packets/cache-invalidation/proposal.md`
- `skills/proposal-former/tests/fixtures/intake-formation/multiple/packets/cache-observability/packet.json`
- `skills/proposal-former/tests/fixtures/intake-formation/multiple/packets/cache-observability/proposal.md`

No other implementation path is authorized.

## Revised validation identities

Direct packet mechanics:

```bash
python3 skills/proposal-packets/scripts/proposal_packets.py validate skills/proposal-former/tests/fixtures/intake-formation
```

Archived packet mechanics after exact checkpoint creation:

```bash
python3 "$FORMATION_ARCHIVE_DIR/skills/proposal-packets/scripts/proposal_packets.py" validate "$FORMATION_ARCHIVE_DIR/skills/proposal-former/tests/fixtures/intake-formation"
```

The vertical validates the same common root and separately proves that the one
outcome contains exactly `work-engine.fixture.cache-invalidation`, the multiple
outcome contains exactly
`work-engine.fixture.cache-invalidation-semantics` and
`work-engine.fixture.cache-invalidation-observability`, and the zero outcome
contains no `packet.json`.

The implementation phase remains separate and must not resume until the
supervisor publishes the proposed `attempt-5` / `accepted-v4` active identity.
