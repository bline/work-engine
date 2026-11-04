#!/usr/bin/python3
"""DRAFT live4 stopped-copy extractor. Preserve raw material before semantic checks."""
import hashlib, json, pathlib, shutil, sqlite3, sys

HERE = pathlib.Path(__file__).resolve().parent
TRIAL = HERE.parent
assert len(sys.argv) == 2
state = pathlib.Path(sys.argv[1]).resolve()
assert state in ((TRIAL / 'live4-state').resolve(), (TRIAL / 'rehearsal-state').resolve())
log = state / 'logs'
run = json.loads((log / 'result.json').read_text())
assert run['case'] == 's10-live4-selected-schema7-owner2-two-cycle-semantic'
assert run['status'] in ('passed', 'failed', 'uncertain_teardown')
sha = lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
frozen = state / 'stopped-copy-final'  # external scope controller, after cgroup quiescence
assert frozen.is_dir(), 'authoritative post-quiescence copy absent'
work = state / 'postrun-readback-v1'
assert not work.exists(), 'one-shot extraction; preserve previous readback'
work.mkdir(mode=0o700)
bindings = []
for name in ('tasks.sqlite', 'tasks.sqlite-wal', 'tasks.sqlite-shm',
             'lifecycle.sqlite', 'lifecycle.sqlite-wal', 'lifecycle.sqlite-shm'):
    original = frozen / name
    if not original.exists():
        continue
    before = sha(original)
    dest = work / name
    shutil.copy2(original, dest)
    assert before == sha(original) == sha(dest), name
    bindings.append({'frozen': str(original), 'readback': str(dest), 'sha256': before})
assert (work / 'tasks.sqlite').exists() and (work / 'lifecycle.sqlite').exists()
life = sqlite3.connect('file:' + str(work / 'lifecycle.sqlite') + '?mode=ro', uri=True)
task = sqlite3.connect('file:' + str(work / 'tasks.sqlite') + '?mode=ro', uri=True)
query = lambda sql, args=(): list(life.execute(sql, args))
turns = query('SELECT invocation_id,purpose,state,prompt,final_text,final_sha256 FROM native_turns ORDER BY entered_wall_ms')
material = [{'invocation_id': invocation, 'purpose': purpose, 'state': status,
             'prompt_sha256': hashlib.sha256(prompt).hexdigest(),
             'prompt_text': prompt.decode('utf-8', 'replace'),
             'final_sha256': final_sha,
             'final_text': final_text.decode('utf-8', 'replace') if final_text is not None else None}
            for invocation,purpose,status,prompt,final_text,final_sha in turns]
raw_path = log / 'model-material.json'
assert not raw_path.exists(), 'do not replace prior raw extraction'
raw_path.write_text(json.dumps(material, indent=2) + '\n')
raw_sha = sha(raw_path)
# Raw material is durable before parsing owner state or checking positive-path details.
owner_rows = task.execute('SELECT record_json FROM tasks WHERE task_id=?', ('task:s8:analysis',)).fetchall()
owner = json.loads(owner_rows[0][0]) if len(owner_rows) == 1 else None
usage = []; latest = {}
for session, sequence, raw in query("SELECT session_id,sequence,raw FROM native_ingress WHERE method='thread/tokenUsage/updated' ORDER BY observed_wall_ms,sequence"):
    wire = json.loads(raw.decode()); params = wire['params']
    item = {'session_id': session, 'sequence': sequence, 'thread_id': params['threadId'],
            'turn_id': params['turnId'], 'emitted_at_ms': wire.get('emittedAtMs'),
            'raw_token_usage': params['tokenUsage']}
    usage.append(item); latest[item['thread_id']] = item
read_rows = query('SELECT operation_attempt_id,call_id,state,result_sha256,result FROM native_operations ORDER BY rowid')
operations = [{'attempt_id': a, 'call_id': c, 'state': s, 'sha256': d,
               'result_bytes_hex': bytes(b).hex() if b is not None else None}
              for a,c,s,d,b in read_rows]
result = {
    'schema': 's10-live4-stopped-extraction-v1', 'run_result_sha256': sha(log / 'result.json'),
    'run_status': run['status'], 'mode': run['mode'], 'frozen_db_wal_bindings': bindings,
    'lifecycle_schema_version': query('PRAGMA user_version')[0][0],
    'task_schema_version': task.execute('PRAGMA user_version').fetchone()[0],
    'epochs': query('SELECT context_id,epoch,target_session_id,state FROM native_execution_epochs ORDER BY context_id,epoch'),
    'transitions': query('SELECT predecessor_context,successor_context,stage,semantic_required FROM transitions ORDER BY created_wall_ms'),
    'semantic_results': query('SELECT phase,verdict,eligibility,transition_id,verifier_invocation_id FROM semantic_results ORDER BY rowid'),
    'inputs': query('SELECT i.input_id,i.sequence,i.state,a.outcome,a.settlement_kind FROM inputs i LEFT JOIN effects e ON e.input_id=i.input_id LEFT JOIN attempts a ON a.effect_id=e.effect_id ORDER BY i.sequence'),
    'segments': query('SELECT queue_attempt_id,segment_no,state FROM native_delivery_segments ORDER BY queue_attempt_id,segment_no'),
    'native_turn_count': len(turns), 'native_operations': operations,
    'owner_record': owner, 'usage_events': usage, 'latest_cumulative_usage_by_thread': latest,
    'model_material_path': str(raw_path), 'model_material_sha256': raw_sha,
    'limits': ['Raw prompt/final texts are preserved before positive-path assertions; model understanding requires retained semantic review.',
               'Failure/refusal evidence remains eligible for assessment; no positive-path shape is fabricated.',
               'Usage is cumulative per thread; HTTP count and exact billing may be unavailable.']
}
assert all(sha(pathlib.Path(b['frozen'])) == b['sha256'] for b in bindings)
out = log / 'postrun-evidence.json'
assert not out.exists(), 'do not replace prior extraction'
out.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'status': 'extracted', 'mode': run['mode'], 'postrun_sha256': sha(out),
                  'model_material_sha256': raw_sha}, sort_keys=True))
