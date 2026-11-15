from pathlib import Path
p=Path('subject/validator/r9_pilot.py');s=p.read_text();pos=s.index('def selected_binding')
s=s[:pos]+'''# Parse at the actual consumer boundary, before any consequence/consumed marker.
def strict_read(path):
 def members(pairs):
  d={}
  for k,v in pairs:
   if k in d:raise ValueError('duplicate member '+k)
   d[k]=v
  return d
 def invalid(v):raise ValueError('non JSON number '+v)
 return json.loads(path.read_text(),object_pairs_hook=members,parse_constant=invalid)

def finite_premises(F):
 try:
  exclusions=set(ADMITTED['premise_metadata_exclusions'])|{'expected'}
  for path,a in ADMITTED['admissions'].items():
   source=a.get('case_file',a.get('conditional_raw_file'))
   c=strict_read(root()/source)
   if not isinstance(c,dict) or sorted(c)!=a['premise_inventory']:raise ValueError(source+' top-level inventory')
   projected={k:([{n:v for n,v in e.items() if n!='expected'} for e in value] if k=='evaluations' else value) for k,value in c.items() if k not in exclusions}
   # Canonical JSON preserves bool/number, array ordering, absence/null and types.
   if json.dumps(projected,sort_keys=True,separators=(',',':'))!=json.dumps(a['premise_record'],sort_keys=True,separators=(',',':')):raise ValueError(source+' governing values/types/inventory')
   strict_read(root()/path)
   for payload in a['payloads']:strict_read(root()/payload)
  for path,want in ADMITTED['rate_premises'].items():
   if json.dumps(strict_read(root()/path),sort_keys=True)!=json.dumps(want,sort_keys=True):raise ValueError(path+' valuation rate')
  strict_read(root()/'oracle/r9-pilot/claims.json')
 except (OSError,ValueError,TypeError,KeyError,AttributeError) as e:
  F.add('finite admission','R10 finite premise outside independent admission: '+str(e));return False
 return True

'''+s[pos:];p.write_text(s)
p=Path('subject/validator/validate_corpus.py');s=p.read_text();s=s.replace("    for claim in r9_pilot.load_registry():\n        evaluated=[]", "    if not r9_pilot.finite_premises(F):\n        for f in F: print('FAIL',f)\n        return 1\n    for claim in r9_pilot.load_registry():\n        evaluated=[]");s=s.replace("    F = Failures(); rates = load_rates(os.path.join(subject, 'oracle/rates')); consumed=[]", "    F = Failures(); consumed=[]");s=s.replace("    for claim in r9_pilot.load_registry():\n        evaluated=[]", "    rates = load_rates(os.path.join(subject, 'oracle/rates'))\n    for claim in r9_pilot.load_registry():\n        evaluated=[]");p.write_text(s)
p=Path('subject/tooling/spec-check/tests/independent_raw_axes.rs');s=p.read_text();pos=s.index('fn r10_applicability')
s=s[:pos]+'''// Dedicated duplicate-rejecting parser: serde_json::Value alone overwrites members.
struct PremiseJson(Value);
impl<'de> serde::Deserialize<'de> for PremiseJson {
 fn deserialize<D:serde::Deserializer<'de>>(d:D)->Result<Self,D::Error>{
  struct Visitor;
  impl<'de> serde::de::Visitor<'de> for Visitor {
   type Value=PremiseJson;
   fn expecting(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{f.write_str("JSON with unique members")}
   fn visit_bool<E:serde::de::Error>(self,v:bool)->Result<PremiseJson,E>{Ok(PremiseJson(json!(v)))}
   fn visit_i64<E:serde::de::Error>(self,v:i64)->Result<PremiseJson,E>{Ok(PremiseJson(json!(v)))}
   fn visit_u64<E:serde::de::Error>(self,v:u64)->Result<PremiseJson,E>{Ok(PremiseJson(json!(v)))}
   fn visit_f64<E:serde::de::Error>(self,v:f64)->Result<PremiseJson,E>{serde_json::Number::from_f64(v).map(|n|PremiseJson(Value::Number(n))).ok_or_else(||E::custom("nonfinite"))}
   fn visit_str<E:serde::de::Error>(self,v:&str)->Result<PremiseJson,E>{Ok(PremiseJson(json!(v)))}
   fn visit_unit<E:serde::de::Error>(self)->Result<PremiseJson,E>{Ok(PremiseJson(Value::Null))}
   fn visit_seq<A:serde::de::SeqAccess<'de>>(self,mut a:A)->Result<PremiseJson,A::Error>{let mut v=Vec::new();while let Some(PremiseJson(x))=a.next_element()?{v.push(x);}Ok(PremiseJson(Value::Array(v)))}
   fn visit_map<A:serde::de::MapAccess<'de>>(self,mut a:A)->Result<PremiseJson,A::Error>{let mut m=serde_json::Map::new();while let Some((k,PremiseJson(v)))=a.next_entry::<String,PremiseJson>()?{if m.insert(k.clone(),v).is_some(){return Err(serde::de::Error::custom(format!("duplicate member {k}")));}}Ok(PremiseJson(Value::Object(m)))}
  }
  d.deserialize_any(Visitor)
 }
}
fn premise_read(p:impl AsRef<std::path::Path>)->Result<Value,String>{
 let bytes=fs::read(p.as_ref()).map_err(|e|format!("{}: {e}",p.as_ref().display()))?;
 serde_json::from_slice::<PremiseJson>(&bytes).map(|v|v.0).map_err(|e|e.to_string())
}
fn finite_premises(r:&std::path::Path,a:&Value)->Result<(),String>{
 let check=||->Result<(),String>{
  let exclusions=a["premise_metadata_exclusions"].as_array().ok_or("exclusions")?;
  for (path,entry) in a["admissions"].as_object().ok_or("admissions")? {
   let source=entry["case_file"].as_str().or_else(||entry["conditional_raw_file"].as_str()).ok_or("source")?;
   let c=premise_read(r.join(source))?;
   let obj=c.as_object().ok_or("record object")?;
   let inventory:Vec<_>=obj.keys().cloned().collect();
   if json!(inventory)!=entry["premise_inventory"] {return Err(format!("{source} top-level inventory"));}
   let mut projected=serde_json::Map::new();
   for (key,value) in obj {
    if key=="expected" || exclusions.iter().any(|x|x==key){continue;}
    let v=if key=="evaluations" {
     Value::Array(value.as_array().ok_or("evaluation array")?.iter().map(|e|{
      let mut ev=e.as_object().ok_or("evaluation object")?.clone();ev.remove("expected");Ok(Value::Object(ev))
     }).collect::<Result<Vec<_>,String>>()?)
    }else{value.clone()};projected.insert(key.clone(),v);
   }
   if Value::Object(projected)!=entry["premise_record"] {return Err(format!("{source} governing values/types/inventory"));}
   premise_read(r.join(path))?;
   for payload in entry["payloads"].as_object().ok_or("payloads")?.keys(){premise_read(r.join(payload))?;}
  }
  for (path,want) in a["rate_premises"].as_object().ok_or("rates")?{if premise_read(r.join(path))?!=*want{return Err(format!("{path} valuation rate"));}}
  premise_read(r.join("oracle/r9-pilot/claims.json"))?;
  Ok(())
 };
 check().map_err(|e|format!("R10 finite premise outside independent admission: {e}"))
}
'''+s[pos:]
s=s.replace('    let registry = read(r.join("oracle/r9-pilot/claims.json"));','    let admitted=r10_admitted();\n    finite_premises(r,&admitted)?;\n    let registry = read(r.join("oracle/r9-pilot/claims.json"));',1)
p.write_text(s)
