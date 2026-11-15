from pathlib import Path
import json, subprocess, signal, sys
r=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/control')
a=json.loads((r/'listener4-argv.json').read_text())
a[a.index('--since')+1]='5451'
i=a.index('--budget-cursor'); del a[i:i+2]
a[a.index('--timeout-seconds')+1]='4200'
(r/'listener7-argv.json').write_text(json.dumps(a,indent=2)+'\n')
p=subprocess.Popen(a,cwd='/home/bline/code/work-engine',stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,bufsize=1)
(r/'listener7-process.json').write_text(json.dumps({'pid':p.pid,'owner':'01a1d6e2-32ba-7701-9115-740dcb41af11','collector':'root exec foreground; natural decision boundary collection','binding':'fresh baseline after ownnew_context epoch10','argv':a},indent=2)+'\n')
try:
 with (r/'listener7.jsonl').open('w') as log:
  for line in p.stdout:
   log.write(line);log.flush();print(line,end='',flush=True)
 code=p.wait()
except KeyboardInterrupt:
 p.send_signal(signal.SIGINT);code=p.wait()
(r/'listener7-terminal.json').write_text(json.dumps({'pid':p.pid,'exit_code':code})+'\n')
sys.exit(code if code>=0 else 128-code)
