#!/usr/bin/env python3
"""Actual S5 simulated native lifecycle + passive terminal UI, isolated by bwrap.

Derived from S5's retained generation_switch_proof.py route. SQL reads here
are test-driver assertions only; the UI receives only public socket snapshots.
No live provider, credentials, UI command transport, or production activation.
"""
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import sqlite3
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path('/mnt/run')
SERVICE = Path('/mnt/lifecycle-service')
VERIFIER = Path('/mnt/controlled_peer')
NATIVE = '/usr/lib/node_modules/@openai/codex/node_modules/@openai/codex-linux-x64/vendor/x86_64-unknown-linux-musl/bin/codex'
FACT = b'ORION-7'
CONTINUATION = json.dumps({
    'protocol_version': 1, 'predecessor_safe': True, 'source_frozen': True,
    'checkpoint_ready': True, 'switch_authorized': True,
    'successor_context': 'context:gen2',
}, sort_keys=True, separators=(',', ':'))
requests = []
errors = []

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_POST(self):
        raw = self.rfile.read(int(self.headers['content-length']))
        body = json.loads(raw)
        index = len(requests) + 1
        requests.append({'auth': bool(self.headers.get('authorization')), 'body': body})
        if index == 1:
            item = {'type':'function_call','id':'gen1-tool','call_id':'gen1-call',
                    'namespace':'workspace','name':'snapshot_read',
                    'arguments':json.dumps({'snapshot_id':'snapshot:gen1','member_id':'m1','offset':0,'length':7})}
        else:
            if index == 2 and 'ORION-7' not in json.dumps(body):
                errors.append('predecessor did not receive exact child result')
            user_texts = [part.get('text') for message in body.get('input',[])
                          if message.get('type')=='message' and message.get('role')=='user'
                          for part in message.get('content',[])]
            if index == 3 and CONTINUATION not in user_texts:
                errors.append('successor rehydration request lacked predecessor continuation')
            if index == 4 and 'Produce successor domain response.' not in user_texts:
                errors.append('successor domain prompt absent')
            if index == 4:
                if query('SELECT context_id,owner_kind FROM subjects') != [('context:gen2','domain')] \
                   or query('SELECT stage FROM transitions') != [('reconciled',)]:
                    errors.append('successor domain request preceded committed switch')
            text = {2:CONTINUATION,3:'REHYDRATED',4:'SUCCESSOR-DOMAIN'}.get(index,'UNEXPECTED')
            if index > 4: errors.append('unexpected extra inference request')
            item = {'type':'message','id':f'message-{index}','role':'assistant','status':'completed',
                    'content':[{'type':'output_text','text':text,'annotations':[]}]}
        response = {'id':f'response-{index}','object':'response','created_at':1,'status':'completed',
                    'output':[item],'usage':{'input_tokens':3,'output_tokens':2,'total_tokens':5}}
        events = [{'type':'response.created','response':{'id':f'response-{index}','status':'in_progress','output':[]}},
                  {'type':'response.output_item.done','output_index':0,'item':item},
                  {'type':'response.completed','response':response}]
        wire = ''.join('data: '+json.dumps(event,separators=(',',':'))+'\n\n' for event in events).encode()
        self.send_response(200)
        self.send_header('Content-Type','text/event-stream')
        self.send_header('Content-Length',str(len(wire)))
        self.end_headers()
        self.wfile.write(wire)

def query(sql, args=()):
    db = ROOT / 'store/lifecycle.sqlite'
    con = sqlite3.connect(f'file:{db}?mode=ro', uri=True)
    try: return con.execute(sql,args).fetchall()
    finally: con.close()

def await_query(sql, predicate, timeout=20):
    deadline = time.monotonic()+timeout
    while time.monotonic()<deadline:
        try:
            rows = query(sql)
            if predicate(rows): return rows
        except sqlite3.Error: pass
        time.sleep(.05)
    raise RuntimeError(f'timed out waiting for {sql}')

def command_digest(payload):
    envelope={'codec':'jcs-rfc8785-v1','domain':'work-engine.lifecycle','kind':'command',
              'payload':payload,'schema':2}
    return hashlib.sha256(json.dumps(envelope,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()

def send_replace(revision, command_id="command:replace", context="context:gen1"):
    payload={'protocol_version':2,'principal_ref':'principal:s5','command_id':command_id,
             'subject_id':'subject:s5','context_generation':context,'build_id':'build:s5',
             'proof_run_id':'proof:s5','grant_ref':'grant:replace','expected_revision':str(revision),
             'kind':'request_replacement','payload':{'reason':'native successor checkpoint'}}
    payload['request_digest']=command_digest(payload)
    wire=json.dumps({'op':'command','command':payload},separators=(',',':')).encode()+b'\n'
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as client:
        client.settimeout(20)
        client.connect(str(ROOT/'socket/service.sock'))
        client.sendall(wire)
        result=b''
        while not result.endswith(b'\n'):
            chunk=client.recv(4096)
            if not chunk: break
            result+=chunk
    return json.loads(result)

def config(generation, server_port, now):
    context=f'context:gen{generation}'
    grant=lambda name,ctx,scope:{'grant_ref':name,'context_generation':ctx,'scope':scope,
                                 'expires_wall_ms':now+120000,'revision':1}
    grants=[grant('grant:native1','context:gen1','native_turn'),
            grant('grant:snapshot1','context:gen1','snapshot_read'),
            grant('grant:replace','context:gen1','request_replacement'),
            grant('grant:native2','context:gen2','native_turn'),
            grant('grant:snapshot2','context:gen2','snapshot_read'),
            grant('grant:enqueue2','context:gen2','enqueue_input')]
    return {'profile':'native_simulated','store_root':str(ROOT/'store'),'socket_path':str(ROOT/'socket/service.sock'),
            'peer_socket_path':str(ROOT/'unused.sock'),'verifier_executable':str(VERIFIER),
            'verifier_ledger_path':str(ROOT/'verifier-ledger.jsonl'),'proof_barrier_dir':None,
            'trusted_issuer':'s5-trusted','principal':{'uid':os.getuid(),'principal_ref':'principal:s5','producer_ref':'producer:s5'},
            'subject_id':'subject:s5','context_generation':context,'build_id':'build:s5','proof_run_id':'proof:s5',
            'grants':grants,'max_executions':1,'max_wait_ms':1000,
            'native':{'executable':NATIVE,'root':str(ROOT/f'native{generation}'),
                'simulator_base_url':f'http://127.0.0.1:{server_port}/v1',
                'session_id':f'session:gen{generation}','invocation_id':f'invocation:gen{generation}',
                'attempt_id':f'attempt:gen{generation}',
                'prompt':('Read snapshot:gen1 member m1 bytes 0 through 7 and report the fact.' if generation==1
                          else 'Produce successor domain response.'),
                'turn_grant_ref':f'grant:native{generation}','turn_grant_revision':1,
                'snapshot_grant_ref':f'grant:snapshot{generation}','snapshot_grant_revision':1,
                'same_native_session_required':False,
                'snapshot':{'id':f'snapshot:gen{generation}','source_revision':f'synthetic:gen{generation}',
                    'members':[{'id':'m1','sha256':hashlib.sha256(FACT).hexdigest(),'bytes':list(FACT)}]}}}


# PTY output is forced through a full resize redraw for stable text assertions;
# raw ANSI frames and the independently read wire snapshots are retained.
import argparse
import errno
import fcntl
import pty
import re
import select
import shutil
import struct
import sys
import tempfile
import termios


class Terminal:
    def __init__(self, socket_path):
        self.master, self.slave = pty.openpty()
        self.original = termios.tcgetattr(self.slave)
        self.size(180, 65)
        self.process = subprocess.Popen(
            ['/mnt/ui', '--connect', str(socket_path), '--subject', 'subject:s5'],
            stdin=self.slave, stdout=self.slave, stderr=self.slave,
            env={'HOME': str(ROOT), 'PATH': '/usr/bin:/bin', 'TERM': 'xterm-256color'})
        self.read(.9)

    def size(self, width, height):
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))

    def read(self, seconds):
        result = bytearray()
        until = time.monotonic() + seconds
        while time.monotonic() < until:
            if select.select([self.master], [], [], .02)[0]:
                try:
                    part = os.read(self.master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not part:
                    break
                result.extend(part)
        return bytes(result)

    def command(self, name):
        os.write(self.master, b'\x10' + name.encode())
        self.read(.15)
        os.write(self.master, b'\r')
        self.read(.8)

    def capture(self, name, expected):
        self.command('connection refresh')
        self.size(100, 30)
        os.kill(self.process.pid, signal.SIGWINCH)
        self.read(.15)
        self.size(180, 65)
        os.kill(self.process.pid, signal.SIGWINCH)
        frame = self.read(.3)
        (ROOT / (name + '.ansi')).write_bytes(frame)
        plain = re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', frame).decode(errors='replace')
        (ROOT / (name + '.txt')).write_text(plain)
        for item in expected:
            assert item in plain, f'{name}: missing {item!r}; see retained frame'
        assert self.process.poll() is None
        return plain

    def close(self):
        try:
            if self.process.poll() is None:
                os.write(self.master, b'\x03')
                self.read(.2)
                self.process.wait(timeout=3)
            assert self.process.returncode == 0
            assert termios.tcgetattr(self.slave) == self.original, 'terminal restoration'
        finally:
            if self.process.poll() is None:
                self.process.kill()
                self.process.wait(timeout=3)
            os.close(self.master)
            os.close(self.slave)


def snapshot(name):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(3)
        client.connect(str(ROOT/'socket/service.sock'))
        client.sendall(b'{"op":"snapshot","subject_id":"subject:s5"}\n')
        data = bytearray()
        while not data.endswith(b'\n'):
            chunk = client.recv(65536)
            assert chunk, 'truncated snapshot'
            data.extend(chunk)
            assert len(data) <= 131072
    result = json.loads(data)
    assert result['protocol_version'] == 1 and result['subject_id'] == 'subject:s5'
    (ROOT/(name+'.json')).write_text(json.dumps(result, indent=2)+'\n')
    return result


def inner(failure):
    global ROOT, requests, errors
    ROOT = Path('/mnt/failure' if failure else '/mnt/success')
    ROOT.mkdir(mode=0o700)
    for name in ('store', 'socket', 'native1', 'native2'):
        (ROOT/name).mkdir(mode=0o700)
    # The failed cut withholds the verifier's exact retained source; native
    # completion alone must never become a completed transition in the UI.
    ledger = '' if failure else json.dumps({'source_id':'native:invocation:gen1',
                                            'final_text':CONTINUATION})+'\n'
    (ROOT/'verifier-ledger.jsonl').write_text(ledger)
    requests, errors = [], []
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    environment = {'HOME':str(ROOT), 'PATH':'/usr/bin:/bin', 'TMPDIR':str(ROOT), 'CARGO_TARGET_DIR':str(ROOT/'unused-target'), 'RUST_LOG':'error'}
    processes = []
    terminal = None
    exit_codes = []

    def start(generation):
        path = ROOT/f'config{generation}.json'
        path.write_text(json.dumps(config(generation, server.server_port, int(time.time()*1000))))
        process = subprocess.Popen([str(SERVICE),str(path)], env=environment, cwd=ROOT,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        processes.append(process)
        return process

    def stop(process):
        process.send_signal(signal.SIGTERM)
        process.wait(timeout=12)
        assert process.returncode == 0, process.stderr.read().decode(errors='replace')
        exit_codes.append(process.returncode)

    try:
        first = start(1)
        await_query("SELECT state FROM native_turns WHERE invocation_id='invocation:gen1'",
                    lambda rows: rows == [('completed_settled',)])
        operations = query('SELECT state FROM native_operations')
        assert operations == [('completed_settled',)], operations
        initial = snapshot('predecessor')
        assert initial['context_generation'] == 'context:gen1'
        terminal = Terminal(ROOT/'socket/service.sock')
        terminal.capture('predecessor', ['context:gen1', 'coherent read', 'current snapshot', 'Lifecycle observation', 'read-only', 'usage unavailable'])
        reply = send_replace(0)
        assert reply.get('accepted') is True, reply
        if failure:
            # Failed verifier completion is durable. It does not qualify the
            # native source, so no checkpoint/switch can be presented as success.
            await_query("SELECT exit_success FROM native_verifier_results", lambda rows: rows == [(0,)])
            blocked = snapshot('verification-failed')
            assert blocked['current_admission'] == {'kind':'fenced','reason':'durable_recovery_required'}, blocked
            assert blocked['transition']['value']['stage'] == 'recovery_required', blocked
            assert blocked['transition']['value']['completed'] is False, blocked
            assert blocked['context_generation'] == 'context:gen1', blocked
            terminal.capture('verification-failed', ['fenced / durable_recovery_required', 'RecoveryRequired', 'completed false', 'context:gen1'])
            assert len(requests) == 2, requests
        else:
            await_query('SELECT stage FROM transitions', lambda rows: rows == [('switching',)])
            switching = snapshot('switching')
            assert switching['transition']['value']['stage'] == 'switching'
            terminal.capture('switching', ['blocked by current transition', 'Switching', 'completed false'])
            stop(first)
            terminal.capture('outage', ['last known only', 'context:gen1'])
            second = start(2)
            await_query("SELECT state FROM native_turns WHERE invocation_id='invocation:gen2'",
                        lambda rows: rows == [('completed_settled',)])
            completed = snapshot('successor')
            assert completed['context_generation'] == 'context:gen2'
            assert completed['current_admission']['kind'] == 'domain'
            assert completed['transition']['value']['stage'] == 'reconciled'
            assert completed['transition']['value']['completed'] is True
            assert initial['cursor']['store_id'] == completed['cursor']['store_id']
            assert int(completed['cursor']['commit_sequence']) > int(initial['cursor']['commit_sequence'])
            terminal.command('connection reconnect')
            terminal.capture('successor', ['context:gen2', 'Reconciled', 'completed true', 'domain / context', 'coherent read'])
            stale = send_replace(0, 'command:stale', 'context:gen1')
            assert stale.get('accepted') is not True, stale
            (ROOT/'stale-command.json').write_text(json.dumps(stale, indent=2)+'\n')
            stable = snapshot('after-stale-command')
            assert {k:v for k,v in stable.items() if k != 'cursor'} == {k:v for k,v in completed.items() if k != 'cursor'}, (stable, completed)
            assert int(stable['cursor']['commit_sequence']) >= int(completed['cursor']['commit_sequence'])
            assert len(requests) == 4 and not errors, (requests, errors)
            sessions = query('SELECT context_id,thread_id FROM native_sessions ORDER BY created_wall_ms')
            assert len(sessions) == 2 and sessions[0][1] != sessions[1][1], sessions
            turns = query('SELECT purpose,state FROM native_turns ORDER BY entered_wall_ms,rowid')
            assert turns == [('domain_work','completed_settled'), ('successor_rehydrate','completed_settled'), ('domain_work','completed_settled')], turns
            stop(second)
        assert all(not request['auth'] for request in requests), 'credential header present'
        if first.poll() is None:
            stop(first)
        terminal.close()
        terminal = None
        result = {'scenario': 'missing-verifier-source' if failure else 'generation-handoff',
                  'requests': len(requests), 'authenticated_requests': 0, 'errors': errors,
                  'native_operations': operations,
                  'native_turns':query('SELECT purpose,state FROM native_turns ORDER BY entered_wall_ms,rowid'),
                  'transitions':query('SELECT stage FROM transitions'),
                  'native_verification':query('SELECT disposition,exit_success FROM native_verifier_results'),
                  'sessions':query('SELECT context_id,thread_id FROM native_sessions ORDER BY created_wall_ms'),
                  'service_exits':exit_codes,
                  'limits':['UI shows current snapshot, not event history',
                            'native child operations are test-driver SQL evidence, absent from public snapshot',
                            'no transcript or qualified usage percentage',
                            'failed verifier cause absent from snapshot; fenced admission and incomplete RecoveryRequired visible']}
        (ROOT/'receipt.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result), flush=True)
    finally:
        if terminal is not None:
            try:
                terminal.close()
            except Exception as error:
                # close() kills/reaps the UI in its finally clause. Continue
                # through service cleanup even if terminal restoration failed.
                print(f'UI cleanup: {error}', file=sys.stderr)
        for process in processes:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=12)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
            (ROOT/f'service-{process.pid}.txt').write_text(process.stderr.read().decode(errors='replace'))
            process.stdout.close()
            process.stderr.close()
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('ui', nargs='?')
    parser.add_argument('service', nargs='?')
    parser.add_argument('verifier', nargs='?')
    parser.add_argument('--evidence', type=Path)
    parser.add_argument('--inside', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.inside:
        # Outer invocation creates a network namespace and hides host HOME.
        try:
            connection = socket.create_connection(('1.1.1.1',443), .2)
        except OSError as error:
            assert error.errno == 101, error
        else:
            connection.close()
            raise AssertionError('network isolation absent')
        inner(False)
        inner(True)
        return
    assert args.ui and args.service and args.verifier, 'three explicit binaries required'
    native = Path(NATIVE)
    assert hashlib.sha256(native.read_bytes()).hexdigest() == 'f34a4d2301892ae96c90097786bfe5dc269f187b6f69faf42a7b357b8c081e35', 'S5 installed native pin drift'
    if args.evidence:
        args.evidence.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix='ui-s5-') as directory:
        run = Path(directory)
        os.chmod(run, 0o700)
        bindings = [(str(Path(source).resolve()), '/mnt/'+name)
                    for source, name in [(args.ui,'ui'), (args.service,'lifecycle-service'),
                                         (args.verifier,'controlled_peer')]]
        shutil.copy2(__file__, run/'smoke.py')
        identities = {Path(target).name: hashlib.sha256(Path(source).read_bytes()).hexdigest()
                      for source, target in bindings}
        identities['smoke.py'] = hashlib.sha256((run/'smoke.py').read_bytes()).hexdigest()
        identities['native'] = hashlib.sha256(native.read_bytes()).hexdigest()
        (run/'binaries.json').write_text(json.dumps(identities,indent=2)+'\n')
        try:
            completed = subprocess.run(['bwrap','--unshare-net','--unshare-pid','--die-with-parent','--ro-bind','/','/',
                '--dev-bind','/dev','/dev','--proc','/proc','--bind',str(run),'/mnt',
                '--tmpfs','/home',
                *[argument for source,target in bindings for argument in ('--ro-bind',source,target)],
                'python3','/mnt/smoke.py','--inside'],
                env={'PATH':'/usr/bin:/bin','HOME':'/mnt','TMPDIR':'/mnt'},
                capture_output=True, text=True, timeout=100)
            print(completed.stdout, end='')
            assert completed.returncode == 0, completed.stderr
        finally:
            if args.evidence:
                for path in run.rglob('*'):
                    if path.is_file() and len(path.relative_to(run).parts) <= 2 and (path.suffix in ('.ansi','.txt') or path.name in (
                        'receipt.json','binaries.json','predecessor.json','switching.json',
                        'successor.json','after-stale-command.json','stale-command.json','verification-failed.json')):
                        target = args.evidence/path.relative_to(run)
                        target.parent.mkdir(parents=True,exist_ok=True)
                        shutil.copy2(path,target)
    print('PASS: isolated populated S5 service + passive UI; disposable state removed')


if __name__ == '__main__':
    main()
