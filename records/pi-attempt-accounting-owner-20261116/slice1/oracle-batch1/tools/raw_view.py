# Throwaway navigation helper: emits the permitted raw view of a case (no expected/revisions/rationale/historical).
import json,sys
EXCLUDE={'expected','revisions','revisions_r3','revisions_r4','revisions_r5','revisions_r6','revisions_r7','revisions_r8','rationale','historical','notes','findings','ambiguity','reachability','controls','invariants','group','class','provenance'}
def strip(o):
    return o
p=sys.argv[1]
d=json.load(open(p))
out={}
for k,v in d.items():
    if k in EXCLUDE: continue
    if k=='evaluations':
        out[k]=[{kk:vv for kk,vv in e.items() if kk!='expected'} for e in v]
    else: out[k]=v
print(json.dumps(out,indent=1))
print('TOP_KEYS',list(d.keys()),file=sys.stderr)
