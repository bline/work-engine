#!/usr/bin/env python3
"""Project retained proof JSON into UI-only historical archive v1. No engine calls."""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]
UI = Path(__file__).resolve().parents[1]
BASE = ROOT / 'experiments/context-lifecycle-rust'
LIVE = BASE / 'evidence/live-text-vertical-2026-10-07'
REJECT = BASE / 'evidence/live-admission-rejection-2026-10-07/diagnosis.json'
RECEIPT_SHA = 'feb5b5317dced64d452c75fb30393e843442ab3b50c6c3e3f65ef418e077f673'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def require(ok, reason):
    if not ok:
        raise ValueError(reason)

def project():
    require(digest(LIVE/'receipt.json') == RECEIPT_SHA, 'receipt differs from parent milestone 1127')
    receipt = json.loads((LIVE/'receipt.json').read_text())
    for item in receipt['evidence']:
        if item['path'] in ('result.json', 'durable-summary.json', 'preflight.json'):
            require(digest(LIVE/item['path']) == item['sha256'], 'source digest mismatch: '+item['path'])
    result = json.loads((LIVE/'result.json').read_text())
    summary = json.loads((LIVE/'durable-summary.json').read_text())
    rejection = json.loads(REJECT.read_text())
    require(receipt['result'] == 'passed', 'not a passing proof')
    require(receipt['live']['entered_native_turns'] == 3 and receipt['live']['remaining_authorized_turns'] == 0, 'unexpected turn accounting')
    require(result['first']['thread_id'] != result['second']['thread_id'] == result['readiness']['thread_id'], 'thread binding mismatch')
    require(result['readiness']['package_revision'] == result['package']['revision'], 'package revision mismatch')
    require(receipt['live']['coordinator_revision'] == summary['coordinator'][0]['revision'], 'revision mismatch')
    sources = {}
    for name, path in [('receipt',LIVE/'receipt.json'),('result',LIVE/'result.json'),('summary',LIVE/'durable-summary.json'),('rejection',REJECT)]:
        sources[name] = {'path': str(path.relative_to(ROOT)), 'sha256': digest(path)}
    def row(kind, context, text, details, refs):
        return dict(kind=kind, context=context, text=text, details=details, refs=refs)
    records = []
    def turn(key, context):
        item = result[key]
        effect = next(e for e in summary['effects'] if e['key'] == item['effect'])
        delivery = next(d for d in summary['deliveries'] if d['key'] == item['delivery'])
        native = next(n for n in summary['native_records'] if n['key'] == item['effect'])
        require(effect['state'] == delivery['outcome'] == 'Success' and effect['domain'] == 'CodexTextTurnCompletion', 'missing accepted domain outcome')
        require(native['output'] == item['output'] and native['thread_id'] == item['thread_id'] and native['turn_id'] == item['turn_id'] and native['text_violation'] is None, 'native/receipt identity or output mismatch')
        require(hashlib.sha256(json.dumps(item['output'], ensure_ascii=False, separators=(',', ':')).encode()).hexdigest() == item['output_digest'], 'output digest mismatch')
        details = f"Native observation: {native['state']}\nCoordinator outcome: {effect['state']}\nEvidence domain: {effect['domain']}\nContext: {item['context']} / epoch {item['epoch']}\nThread: {item['thread_id']}\nTurn: {item['turn_id']}\nEffect: {item['effect']}\nDelivery: {item['delivery']}\nRequest digest: {item['request_digest']}\nOutput digest: {item['output_digest']}"
        records.append(row('agent',context,item['output'],'',[f'result#/{key}/output']))
        records.append(row('accepted',context,{'first':'Input 1 accepted','readiness':'Exact package readiness accepted','second':'Input 2 accepted'}[key],details,[f'result#/{key}','summary#/effects','summary#/deliveries','summary#/native_records']))
    turn('first','A')
    records.append(row('system','A → B','Continuation package retained',f"Package revision: {result['package']['revision']}\nDigest: {result['readiness']['package_digest']}\nA captured package alone is not successor activation.",['result#/package','result#/readiness/package_digest']))
    turn('readiness','B')
    second = next(q for q in result['package']['queue'] if q['id'] == 'vertical-input-2')
    prompt = bytes(second['payload']).decode()
    fact = result['first']['output'].split(':',1)[1]
    require(fact not in prompt and result['second']['output'] == 'SECOND:'+fact, 'carried fact result mismatch')
    records.append(row('input','B',prompt,'Actual queued second input; the carried value is absent.',['result#/package/queue']))
    turn('second','B')
    records.append(row('system','B','Historical proof closed · no inference authority remains',f"Coordinator PID {receipt['live']['coordinator_pid']} survived the handoff. Final revision {receipt['live']['coordinator_revision']}.\nAll 3 authorized native turns consumed. HTTP inference request count unavailable.\nCleanup is separate from settlement; native child exit status was not directly observed.",['receipt#/live','receipt#/cleanup']))
    ds = rejection['durable_state']
    require(all(ds[k] == 0 for k in ['effects','deliveries','adapter_records','adapter_commands','adapter_bindings']), 'rejected setup has entry evidence')
    rejected = [row('rejected','setup','Admission rejected before entry · 0 provider turns',rejection['cause']+'\n'+rejection['scope_of_zero_entry_conclusion']+'\nSeparate attempt; not a failed turn in the successful run.',['rejection#/durable_state','receipt#/preceding_rejected_setup'])]
    v = receipt['validation']
    checks = [row('system','validation',f"{v['default_tests_passed']} default tests · {v['principal_native_loopback_probes_passed']} controlled probes passed",f"Provider-disabled tests and simulated-inference native probes, separate from the live run.\nFormatting: {v['formatting']}\nClippy: {v['clippy_all_targets_deny_warnings']}\nReview: {v['review']}",['receipt#/validation'])]
    return dict(format='work-engine-ui-history', version=1, recorded_at=receipt['recorded_at_utc'], scope='Bounded text-only proof; no production readiness or activation.', sources=sources, limitations=receipt['limitations'], live=records, rejected=rejected, validation=checks)

if __name__ == '__main__':
    target = UI/'replays/live-proof-2026-10-07.v1.json'
    data = json.dumps(project(),ensure_ascii=False,indent=2)+'\n'
    if sys.argv[1:] == ['--check']:
        require(target.read_text() == data, 'archive is not the exact projection of retained evidence')
        print('PASS: archive matches source hashes, receipt identities, outcomes, outputs and distinct attempt accounting')
    elif not sys.argv[1:]:
        target.write_text(data)
        print(target)
    else:
        raise SystemExit('usage: export_live_replay.py [--check]')
