# Work Engine Control Plane and Client Protocol

## Status

Exploratory control-plane direction with one active-construction foundation: `role-scheduler`.

## Idea

Provide a bounded coordination layer for stable logical role identity, activation/delivery, scheduling, runtime routing, acknowledgements, subscriptions/reconciliation, and human-facing client interaction.

The control plane coordinates semantic owners; it does not absorb their truth.

## Current evidence

The `role-scheduler` prototype already provides:

- durable SQLite schedules;
- logical-role addressing;
- revisioning;
- scheduled → presented delivery;
- acknowledgement/cancellation;
- daemon lifecycle;
- blocking wait and streaming subscribe;
- restart persistence.

It does not yet establish generalized activation leases, authority validation, robust claim recovery, or host-mediated wake-up.

## Control-plane ownership

Candidate responsibilities:

- stable logical routing identity;
- activation/lease coordination;
- scheduled-obligation delivery;
- runtime-binding lookup;
- delivery/claim state;
- subscription and reconciliation cursors;
- runtime/delivery health;
- bounded projection routing;
- human/client control packets.

### Future resource-claim coordination

Safe parallel mutation may eventually require atomic resource claims or leases
over repository regions, logical artifacts, review-sensitive scopes, or other
domain-defined resources. This remains an exploratory control-plane consumer,
not part of the current scheduler or agent-state proposal.

The domain owner defines what a resource and conflict mean. A control-plane
realization may own claim identity, fencing, expiry, renewal, release, and crash
reconciliation without becoming the semantic owner of the protected resource.
Review-specific applicability claims remain with review-scope coordination.

## Domain boundary

The control plane does not own:

- proposal meaning;
- workflow semantic state;
- review findings;
- Git/checkpoint truth;
- model judgment;
- human authority.

It carries references and authorized intent.

## Client protocol

A client should be able to:

- reconcile an authoritative projection at a known revision;
- discover available controls;
- advertise environment capabilities;
- submit identity/revision-bound intent;
- receive approval/input requests;
- observe pending obligations and runtime state.

A control packet must bind action, subject, revision, authority context, and expected consequence. UI buttons do not create authority.

## Relationship to scheduler

Scheduling is one control-plane service. Time becoming due never grants permission or activates a role by itself.

## Relationship to runtime adapters

The control plane coordinates authorized activation and delivery; runtime adapters realize provider-specific execution.

## Relationship to Studio

Studio is a client/control surface over this protocol, not the control plane's semantic owner.

## Compact statement

> The control plane owns coordination and delivery semantics, not domain truth or model judgment.
