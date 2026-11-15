"""No domain startup: exact input, imports, ELF resolution and disposable sentinels."""
import os,sys,json,pathlib,hashlib,subprocess,socket,time,pty,termios,fcntl,signal,importlib.util
P=pathlib.Path
_deadline=float(os.environ['U2_ABSOLUTE_DEADLINE'])
def remaining(cap):
 r=min(cap,_deadline-time.monotonic());assert r>0,'absolute deadline exhausted';return r
sha=lambda p:hashlib.sha256(P(p).read_bytes()).hexdigest()
def qualify():
 c=json.loads(P(os.environ['U2_CLOSURE']).read_text())
 for e in c['entries']:
  p=P(e['destination']);assert p.is_file() and sha(p)==e['sha256'],str(p)
  assert p.stat().st_mode&0o777==e['mode'],str(p)
 assert os.getuid()==1000 and os.getgid()==1000
 status=P('/proc/self/status').read_text();assert 'CapEff:\t0000000000000000' in status and 'NoNewPrivs:\t1' in status
 assert [n for _,n in socket.if_nameindex()]==['lo']
 s=socket.socket();s.settimeout(remaining(1));error=s.connect_ex(('192.0.2.1',9));s.close();assert error!=0
 assert len(P('/proc/net/route').read_text().splitlines())==1
 # Fail closed if writable areas or empty HOME cannot be established.
 home=P(os.environ['HOME']);home.mkdir(mode=0o700,exist_ok=True);assert not list(home.iterdir())
 for d in [os.environ['TMPDIR'],os.environ['UI_TEST_EVIDENCE_DIR']]:
  p=P(d)/'sentinel-file';p.write_bytes(b'sentinel');assert p.read_bytes()==b'sentinel';p.unlink()
 # Import complete owner module without entering main/starting daemon/reading profiles.
 spec=importlib.util.spec_from_file_location('u2_events_preflight',os.environ['U2_EVENTS_OWNER']);mod=importlib.util.module_from_spec(spec);sys.modules[spec.name]=mod;spec.loader.exec_module(mod)
 # Every copied stdlib extension is resolved through the selected interpreter's loader.
 import encodings,sqlite3,ssl,ctypes,ctypes.util,select,datetime,threading,runpy,hashlib
 for k in ['U2_FIXTURE_BINARY','UI_TEST_HOST','U2_OBSERVE_BINARY']:
  r=subprocess.run(['/lib64/ld-linux-x86-64.so.2','--list',os.environ[k]],capture_output=True,timeout=remaining(5));assert r.returncode==0,r.stderr
 ui=next(m['target'] for m in c['mounts'] if m['target'].endswith('/work-engine-terminal-ui'))
 assert subprocess.run(['/lib64/ld-linux-x86-64.so.2','--list',ui],capture_output=True,timeout=remaining(5)).returncode==0
 # Exact helper route, normal termination and forced termination/reap.
 code='import signal,time,sys;signal.signal(signal.SIGTERM,lambda a,b:sys.exit(0));print("ready",flush=True);time.sleep(30)'
 child=subprocess.Popen(['/usr/bin/python3','-I','-c',code],stdout=subprocess.PIPE,text=True)
 assert child.stdout.readline().strip()=='ready'
 subprocess.run(['/bin/kill','-TERM',str(child.pid)],check=True,timeout=remaining(3));assert child.wait(timeout=remaining(3))==0
 child=subprocess.Popen(['/usr/bin/python3','-I','-c','import time;time.sleep(30)']);child.kill();assert child.wait(timeout=remaining(3))==-9
 master,slave=pty.openpty();before=termios.tcgetattr(slave);fcntl.fcntl(master,fcntl.F_SETFL,os.O_NONBLOCK)
 child=subprocess.Popen(['/usr/bin/python3','-I','-c','print("pty-sentinel",flush=True)'],stdin=slave,stdout=slave,stderr=slave)
 assert child.wait(timeout=remaining(3))==0;assert b'pty-sentinel' in os.read(master,4096);assert termios.tcgetattr(slave)==before;os.close(master);os.close(slave)
 # Socket/file operations used by the bounded route, without domain protocols.
 unix=P(os.environ['TMPDIR'])/'sentinel.sock';s=socket.socket(socket.AF_UNIX);s.bind(str(unix));s.close();unix.unlink()
 s=socket.socket();s.bind(('127.0.0.1',0));s.close()
 return {'status':'qualified','uid':os.getuid(),'input_files':len(c['entries']),'imports':sorted(n for n in sys.modules if n and not n.startswith('_')),'network_errno':error,'pty':'passed','term_and_kill_reap':'passed','domain_started':False}
if __name__=='__main__':print(json.dumps(qualify()))
