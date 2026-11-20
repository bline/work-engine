#!/usr/bin/env python3
"""Slice 4 smoke: `work-engine-pi-host prepare` from a composition profile over
the real claude-sdk runtime and the offline relay observer, then `serve` on the
prepared serve config, driven through two relay turns by serve.py's controller.
Usage: prepare-smoke.py <out-dir> <host-bin> <observer-bin> <serve.py>"""
import importlib.util, json, os, subprocess, sys, time
from pathlib import Path

out, host, observer, serve_py = Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4]
spec = importlib.util.spec_from_file_location('serve', serve_py); sv = importlib.util.module_from_spec(spec); spec.loader.exec_module(sv)
out.mkdir(parents=True)
runtime = Path('/home/bline/.local/state/work-engine/dev-vertical-h2-route-20261117/runtime-claude-sdk')
obs = subprocess.Popen([observer], env={**os.environ, 'HOSTED_OBSERVER_SCENARIO': 'relay'}, stdout=subprocess.PIPE, stderr=open(out / 'observer.stderr', 'w'), text=True)
ready = json.loads(obs.stdout.readline())
profile = {'schema': 'work-engine.pi-host-composition.v1',
           'runtime': {'root': str(runtime / 'root'), 'manifest': {'path': str(runtime / 'runtime-manifest.json'), 'sha256': sv.fsha(runtime / 'runtime-manifest.json')}},
           'docker': '/usr/bin/docker', 'provider': 'claude-sdk', 'model': 'claude-haiku-5-5',
           'hosted': {'routes': [{'id': 'primary', 'payer': 'synthetic-subscription', 'endpoint': f"loopback:{ready['primary']}"},
                                 {'id': 'secondary', 'payer': 'synthetic-api', 'endpoint': f"loopback:{ready['secondary']}"}], 'max_attempts_per_request': 2},
           'max_requests': 8, 'budget_microusd': 8 * 1320001, 'max_context_bytes': 400000, 'max_tool_calls': 16,
           'turn_deadline_secs': 300, 'job_secs': 900}
(out / 'profile.json').write_text(json.dumps(profile))
d = subprocess.run([host, 'profile', 'describe', '--profile', str(out / 'profile.json')], capture_output=True, text=True)
print('describe', d.returncode, d.stdout.strip()[:400], d.stderr.strip()[:300])
p = subprocess.run([host, 'prepare', '--profile', str(out / 'profile.json'), '--root', str(out / 'attempt-1'), '--snapshot-id', 'snapshot:serve-qual'], capture_output=True, text=True)
print('prepare', p.returncode, p.stderr.strip()[:300])
record = json.loads(p.stdout)
print('record', {k: record[k] for k in ('job_id', 'job_sha256', 'deadline_unix_seconds', 'pi_host_json_sha256', 'composition_sha256')})
env = {k: v for k, v in os.environ.items() if k != 'WORK_ENGINE_HOSTED_CREDENTIALS'}
dynamic = json.loads(subprocess.run([host, 'dynamic-tools'], capture_output=True, text=True, check=True).stdout)
proc = subprocess.Popen([host, 'serve', '--config', str(out / 'attempt-1' / 'serve-config.json')], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(out / 'host.stderr', 'w'), env=env)
c = sv.Controller(proc, open(out / 'protocol.jsonl', 'w'))
init = c.reply_to(c.request('pi-host/initialize', {}), 30)
assert init['result']['job_sha256'] == record['job_sha256'], init
ts = c.reply_to(c.request('thread/start', {'model': 'claude-haiku-5-5', 'dynamicTools': dynamic}), 300)
thread = ts['result']['thread']['id']
statuses, eligible = [], []
for prompt in sv.PROMPTS:
    turn, completed = sv.run_turn(c, thread, prompt, [])
    statuses.append(completed['status'])
    cap = c.reply_to(c.request('pi-host/capture', {'threadId': thread, 'turnId': turn}), 30)['result']
    eligible.append(json.loads(Path(cap['path']).read_text())['eligibility'])
proc.stdin.close(); proc.wait(timeout=180); obs.kill()
state = json.loads((out / 'attempt-1' / 'pi-host-state' / 'pi-state.json').read_text())
names = subprocess.run(['docker', 'ps', '-a', '--format', '{{.Names}}'], capture_output=True, text=True).stdout.split()
print('turns', statuses, 'eligibility', eligible, 'exit', proc.returncode, 'failures', state['failures'],
      'container_left', state['container_name'] in names, 'record', json.loads((out / 'attempt-1' / 'pi-host-state' / 'pi-host-serve-record.json').read_text())['status'])
