"""Compare Python Draft2020-12 and the actual Rust CLI on exact wire/schema states."""
import copy,json,pathlib,subprocess,sys,tempfile
import jsonschema
root=pathlib.Path(sys.argv[1]);binary=pathlib.Path(sys.argv[2]);checks=[]
canonical=json.loads((root/'spec/ck.json').read_text())
wire={'exposure':{'pool':'pi_api','low':{'nano':'15','basis':'usage_valuation'},'high':{'kind':'at_least','nano':'500','basis':'proxy_reservation'}},'valuation':{'state':'valued','complete':False,'low':'15','high':'15','identities':['openai/model'],'derivations':[]},'obligation':{'state':'none','amount_source':'none','cause':'orphan'}}
expected={'kind':'accounting_expected_assertion','id':'independent','version':'1.7-s1','raw_subject_ref':'raw:sha256','policy_refs':['PLAN43d0e7c8'],'authored_by':'independent','exact_subject_refs':[],'assertions':[]}
def add(command,schema,v,label,expected_valid):checks.append((command,schema,v,label,expected_valid))
add('canonical','ck.schema.json',canonical,'canonical correct boundaries',True)
for target,val in [('boundaries','C'),('stages','C-primary')]:
 bad=copy.deepcopy(canonical);bad[target][0]['id']=val;add('canonical','ck.schema.json',bad,'interchanged '+target,False)
add('wire','wire.schema.json',wire,'actual decimal/flat wire',True)
for path,val in [(('exposure','high','nano'),15),(('exposure','high','kind'),'variant'),(('exposure','pool'),'PiApi'),(('valuation','low'),15),(('obligation','amount_source'),'settlement_amount')]:
 bad=copy.deepcopy(wire);at=bad
 for k in path[:-1]:at=at[k]
 at[path[-1]]=val;add('wire','wire.schema.json',bad,'bad '+'.'.join(path),False)
for reason,wanted in [('TierUnestablished(Missing)',True),('TierUnestablished(Unrecognised)',True),('TierUnestablished',False)]:
 bad=copy.deepcopy(wire);bad['valuation']={'state':'unpriceable','supported_low':None,'priced_high':'15','reasons':[reason]};add('wire','wire.schema.json',bad,'qualified reason '+reason,wanted)
add('evidence','accounting.schema.json',expected,'separate expected kind',True)
for key,val in [('kind','accounting_input_proof'),('version','1.6-s1'),('eligible',True)]:
 bad=copy.deepcopy(expected);bad[key]=val;add('evidence','accounting.schema.json',bad,'interchanged/malformed evidence '+key,False)
with tempfile.TemporaryDirectory(prefix='schema-agreement-') as tmp:
 for command,schema_name,value,label,wanted in checks:
  schema=json.loads((root/'spec'/schema_name).read_text());jsonschema.Draft202012Validator.check_schema(schema)
  python_valid=jsonschema.Draft202012Validator(schema).is_valid(value)
  path=pathlib.Path(tmp)/'subject.json';path.write_text(json.dumps(value))
  result=subprocess.run([str(binary),command,str(root),str(path)],capture_output=True,text=True)
  rust_valid=result.returncode==0
  if python_valid!=wanted or rust_valid!=wanted:raise AssertionError((label,wanted,python_valid,rust_valid,result.stderr))
 print(json.dumps({'status':'PASS','agreement_cases':len(checks),'canonical_stage_boundary':'distinct','scope':'standard schema, structural constructors and actual wire; no raw financial conformance'}))
