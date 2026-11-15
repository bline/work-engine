import os,sys,json,pathlib,subprocess,time
c=json.loads(pathlib.Path(os.environ['U2_CLOSURE']).read_text());r=c['realization'];t=r['timing'];ep=r['entrypoints']
sys.path.insert(0,str(pathlib.Path(ep['preflight']).parent))
from preflight import qualify
out=pathlib.Path(os.environ['U2_OUTER_OUTPUT']);deadline=float(os.environ['U2_ABSOLUTE_DEADLINE'])
def remaining(cap):
    v=min(cap,deadline-time.monotonic());assert v>0,'absolute deadline exhausted';return v
try:
    q=qualify();(out/'pre-effect.json').write_text(json.dumps(q,indent=2))
except BaseException as e:
    (out/'pre-effect-refusal.json').write_text(repr(e));raise
mode=sys.argv[1];assert mode in set(r['modes'].values())
if mode=='observer-sentinel':
    x=subprocess.run([ep['observer']],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=remaining(t['S']));assert x.returncode==64;raise SystemExit(0)
if mode=='success':raise SystemExit(0)
if mode=='failure':raise SystemExit(37)
if mode in ['timeout','attachment']:
    time.sleep(remaining(t['W_watchdog']));raise SystemExit(0)
assert mode=='fixture'
try:x=subprocess.run([ep['observer'],'--released-startup-observation'],timeout=remaining(t['I']));code=x.returncode
except subprocess.TimeoutExpired:code=124
remaining_processes=[]
for p in pathlib.Path('/proc').iterdir():
    if p.name.isdigit() and int(p.name)!=os.getpid():
        try:remaining_processes.append({'pid':int(p.name),'cmdline':(p/'cmdline').read_bytes().replace(b'\0',b' ').decode(errors='replace')})
        except (FileNotFoundError,PermissionError,ProcessLookupError):pass
(out/'before-boundary-exit.json').write_text(json.dumps({'driver_code':code,'remaining':remaining_processes,'consequence':'Remaining descendants forcibly ended at container teardown; no natural closure claim'},indent=2))
raise SystemExit(code)
