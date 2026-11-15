"""Exercise independently authored exact VC07 evidence bundles via the actual CLI."""
import json,pathlib,subprocess,sys
root=pathlib.Path(sys.argv[1]);binary=sys.argv[2];out=[]
for filename,wanted in [('VC07-unlinked.bundle.json',True),('VC07-conditional.bundle.json',True),('VC07-missing-completion.wrong.json',False),('VC07-relabel-dollar.wrong.json',False),('VC07-relabel-dollar-hidden.wrong.json',False),('VC07-unlinked-release.wrong.json',False)]:
 path=root/'oracle/r8-controls'/filename
 run=subprocess.run([binary,'evidence',str(root),str(path)],capture_output=True,text=True)
 out.append({'file':filename,'expected_valid':wanted,'actual_valid':run.returncode==0,'diagnostic':run.stderr.strip()})
print(json.dumps({'status':'PASS'if all(x['expected_valid']==x['actual_valid']for x in out)else'FAIL','controls':out,'limits':'supplied witness/reference coherence; raw applicability truth independently owned'}))
raise SystemExit(not all(x['expected_valid']==x['actual_valid']for x in out))
