# Skill-update delivery: organization for review

Status: proposed; implementation has not started. This is the second external manual-orchestration pilot.

```text
Human — initial organization and implementation decision
└─ Root coordinator — Sol 6.1 / xhigh
   ├─ Delivery supervisor — Sol 6.1 / xhigh, retain discovery session
   │  ├─ Publisher implementer — Terra / medium, retain discovery session
   │  │  └─ skill_release.py and test_skill_release.py
   │  └─ Refresh-guidance implementer — Terra / medium, retain discovery session
   │     └─ coordination-channels/SKILL.md
   └─ Reviewer — Astra / high, future fresh top-level standard-tier session
```

The observed discovery sessions use the default tier; those settings are explicitly proposed for implementation. Assignment records, rather than this logical tree, own actual session identity, runtime ancestry, configuration and collection. Both Terra discovery sessions are collected and preserved. A future continuation needs its own scope/context/configuration check.

The proposed routing choice is one announcement on exact topic `all` when installed skills are affected. An announceable new release with unchanged installed content stays on `skill-release` and demands no reload. Existing opt-out, same-current suppression, one publication attempt and unconfirmed-publication behavior remain intact.

This reaches ordinarily configured `all` listeners. Dedicated `skill-release`-only listeners would need to include `all` for changed-skill notices. Broadcasting changed notices also makes them eligible for collection by sessions whose own loaded skills are unaffected; those sessions assess relevance. The alternative keeps `skill-release` and deliberately updates subscriber selections, preserving dedicated consumers but requiring a bootstrap/update owner for already-running sessions. No complete listener inventory has been established.

The consumer-guidance owner adds a compact projection in the existing communication skill: assess relevance before dependent work, re-read affected installed guidance when allowed, preserve useful task context, and forward relevant changes to affected children. Removed guidance becomes an owned missing-guidance dependency. Publication, selector match, observed receipt and actual refresh remain separate claims; runtime and experiment exposure authority is preserved.

Proposed validation joins the real publisher and existing selector in a temporary board/release fixture, followed by a named retained-consumer rehearsal before its next dependent fixture action. No new consumer agent is required: the retained refresh owner performs the bounded rehearsal. This would establish that fixture and receiver only. No implementation, functional tests, rehearsal, reviewer launch, live release, commit or deployment has occurred during formation.

Discovery established two distinct assignable outcomes and a real exact-topic boundary. Implementation continuity, revision handling and accepted cost remain untested. Small organization revisions require coordinator approval; large ones require human approval, classified by consequences.

The exact candidate is [organization-r1.json](organization-r1.json); its digest and pending human decision are recorded separately in [decision-r1.json](decision-r1.json). Detailed discovery and proposed packets are linked from the JSON. Concurrent manual-orchestration automatic-continuation source is excluded.
