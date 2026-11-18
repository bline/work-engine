"""Derive requirements revision hosted-route-qualification-r2 from the frozen requirements.

Operator decision D, 2026-11-18: keep provider_family (anthropic), remove only
execution_confinement; new run identity; truthful propositions. Nothing else changes.

usage: python3 -I make_r2.py <frozen/requirements.json> <out requirements.json>
The output file must not exist.
"""
import json, sys

RUN_ID = 'hosted-route-qualification-r2-20261118'
PROP = ('Route qualification only (requirements revision hosted-route-qualification-r2): the initial RP3 '
        'implementation review answer for the pinned candidate was produced on the hosted Pi analysis profile '
        'work-engine.pi-analysis-hosted.v1 by the requested model claude-haiku-5-5 via Pi provider claude-sdk '
        '(Claude Code in the inference container, Anthropic Messages family, provider family anthropic), reading '
        'the subject only through the Host-served read-only snapshot tools snapshot_list and snapshot_read, which '
        'Claude Code presents as mcp__pi__snapshot_list and mcp__pi__snapshot_read. It is not the guarded Claude '
        'reviewer the frozen requirements name: execution confinement is not established. No qualification '
        'outcome establishes this condition for {use} or makes the result consumable.')
USES = {'cond-builder-projection': 'builder projection', 'cond-campaign-terminalization': 'campaign terminalization'}

src, out = sys.argv[1], sys.argv[2]
value = json.load(open(src))
value['problemScope']['runId'] = RUN_ID
for c in value['acceptanceConditions']:
    dims = [r['dimension'] for r in c['realizationConstraints']]
    assert dims == ['provider_family', 'execution_confinement'], dims
    c['realizationConstraints'] = [r for r in c['realizationConstraints'] if r['dimension'] != 'execution_confinement']
    c['proposition'] = PROP.format(use=USES[c['id']])
with open(out, 'x') as f:
    f.write(json.dumps(value, indent=2) + '\n')
