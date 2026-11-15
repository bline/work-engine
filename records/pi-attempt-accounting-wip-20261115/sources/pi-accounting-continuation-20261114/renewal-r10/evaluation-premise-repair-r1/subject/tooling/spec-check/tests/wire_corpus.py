"""Actual corpus constructor-shape qualification only; money is never recomputed."""
import json,pathlib,subprocess,sys,tempfile
import jsonschema
root=pathlib.Path(sys.argv[1]);binary=sys.argv[2]
schema=json.loads((root/'spec/wire.schema.json').read_text());validator=jsonschema.Draft202012Validator(schema)
values=[];failures=[];cases=0
for f in sorted((root/'corpus/cases').glob('*.json')):
 case=json.loads(f.read_text());cases+=1
 for idx,attempt in enumerate(case['expected']['attempts']):
  value={k:attempt[k]for k in ['exposure','valuation','obligation','accounting_result_ref']if k in attempt};values.append(value)
  errors=list(validator.iter_errors(value))
  if errors:failures.append({'case':f.name,'attempt':idx,'errors':[e.message for e in errors]})
with tempfile.TemporaryDirectory(prefix='wire-corpus-') as tmp:
 path=pathlib.Path(tmp)/'fragments.json';path.write_text(json.dumps(values));result=subprocess.run([binary,'wire',str(root),str(path)],capture_output=True,text=True)
 if result.returncode:failures.append({'rust_exit':result.returncode,'stderr':result.stderr})
print(json.dumps({'status':'FAIL'if failures else'PASS','corpus_cases':cases,'actual_attempt_fragments':len(values),'failures':failures,'qualification':'actual constructor shapes only; independent raw financial assertions and accounting refs remain separate'}))
raise SystemExit(bool(failures))
