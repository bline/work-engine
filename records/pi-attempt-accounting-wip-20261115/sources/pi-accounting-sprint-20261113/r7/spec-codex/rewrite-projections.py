import json,pathlib
r=pathlib.Path('/home/bline/cbm-work/p137-s1r7/s1');p=r/'spec/ck.json';s=json.loads(p.read_text())
for p0 in s['projections']:
 t=p0['type_id'];v=p0['variant'];p0['wire_tag']='eligible' if t=='Replacement' else 'decision' if t=='SettlementDecision' else 'state' if t in ['Valuation','Obligation','TemporalObligation','Field','FieldCount','GateVerification'] else 'variant';p0['wire_value']=(v=='eligible') if t=='Replacement' else v
s['finalization']=[]
for state in ['none','outstanding','closed']:
 for witness in ['eligible_usage','valid_keepall','other']:
  source='none' if state=='none' else 'reservation_bound' if state=='outstanding' or witness=='other' else 'settlement_amount' if witness=='eligible_usage' else 'reservation_confirmed'
  s['finalization'].append({'temporal':state,'witness':witness,'amount_source':source,'preserve':['cause'] if state=='none' else ['by'] if state=='closed' else []})
for n in s['invariants']:
 if n['id']=='F-PROJECTION':n['text']='Single-owned result-bearing finalization bindings select Obligation.amount_source and preserved fields. Projection records enforce legal serialized cross-field combinations. No other table selects this amount source.'
 if n['id']=='TEMPORAL-AND-FINALIZATION':
  # Replace only competing final amount-source sentences, preserving existing temporal semantics.
  n['text']=n['text'].replace('External projection preserves temporal state/cause/by; all variants carry amount_source. none:none; outstanding:reservation_bound; closed eligible Usage:settlement_amount, valid KeepAll:reservation_confirmed, other:reservation_bound.','Amount-source selection is exclusively the finalization bindings.')
p.write_text(json.dumps(s,indent=2,sort_keys=True)+'\n')
p=r/'tooling/spec-check/src/lib.rs';t=p.read_text().replace('    pub boundaries: Vec<Stage>,','    pub boundaries: Vec<Stage>,\n    pub finalization: Vec<ObligationBinding>,');t=t.replace('    let digest = sha256(', '    validate_wire_interface(s, interface)?;\n    let digest = sha256(',1);t=t.replace('        for p in &s.projections { body.push_str', '        body.push_str(&format!("## Finalization bindings\\n\\n```json\\n{}\\n```\\n\\n",serde_json::to_string_pretty(&s.finalization).unwrap()));\n        for p in &s.projections { body.push_str');p.write_text(t)
p=r/'tooling/spec-check/src/decisions.rs';t=p.read_text();t=t.replace('pub struct Projection {','pub struct ObligationBinding {pub temporal:String,pub witness:String,pub amount_source:String,pub preserve:Vec<String>}\n#[derive(Clone,Debug,Serialize,Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct Projection {').replace('pub external_state:String,','pub external_state:String,pub wire_tag:String,pub wire_value:Value,');pos=t.index(' let expected_boundaries=');t=t[:pos]+''' for temporal in ["none","outstanding","closed"] {for witness in ["eligible_usage","valid_keepall","other"] {
  let rows:Vec<_>=s.finalization.iter().filter(|b|b.temporal==temporal&&b.witness==witness).collect();if rows.len()!=1{return Err("finalization gap/overlap".into())}let b=rows[0];
  let source=if temporal=="none"{"none"}else if temporal=="outstanding"||witness=="other"{"reservation_bound"}else if witness=="eligible_usage"{"settlement_amount"}else{"reservation_confirmed"};
  let preserve:Vec<String>=if temporal=="none"{vec!["cause".into()]}else if temporal=="closed"{vec!["by".into()]}else{vec![]};
  if b.amount_source!=source||b.preserve!=preserve{return Err("illegal finalization result".into())}
 }}
 if s.finalization.len()!=9{return Err("unknown finalization tuple".into())}
'''+t[pos:];t+='''
pub(crate) fn validate_wire_interface(s:&Spec,i:&Value)->Result<()> {
 for (type_id,section,field) in [("Valuation","valuation","state"),("SettlementDecision","settlement","decision"),("Basis","settlement","basis")] {
  let actual:BTreeSet<_>=i[section][field].as_array().ok_or("missing external state mapping")?.iter().filter_map(Value::as_str).collect();
  let t=s.types.iter().find(|t|t.id==type_id).ok_or("missing mapped type")?;
  if actual!=t.variants.iter().map(|v|v.name.as_str()).collect(){return Err("unmapped external state".into())}
 }
 for (type_id,section) in [("RejectReason","settlement"),("UnpriceableReason","valuation")] {
  let actual:BTreeSet<String>=i[section]["reasons"].as_array().ok_or("missing external reasons")?.iter().filter_map(Value::as_str).map(String::from).collect();
  let t=s.types.iter().find(|t|t.id==type_id).ok_or("missing reason type")?;
  let expected:BTreeSet<String>=t.variants.iter().map(|v|if v.fields.contains_key("field"){format!("{}(field)",v.name)}else{v.name.clone()}).collect();
  if actual!=expected{return Err("unmapped external reason".into())}
 }
 for p in &s.projections {
  let tag=if p.type_id=="Replacement"{"eligible"}else if p.type_id=="SettlementDecision"{"decision"}else if ["Valuation","Obligation","TemporalObligation","Field","FieldCount","GateVerification"].contains(&p.type_id.as_str()){"state"}else{"variant"};
  let value=if p.type_id=="Replacement"{Value::Bool(p.variant=="eligible")}else{Value::String(p.variant.clone())};
  if p.wire_tag!=tag||p.wire_value!=value{return Err("conflicting external projection tag/value".into())}
 }
 Ok(())
}
''';p.write_text(t)
# JSON Schema mirrors serde fixed records, no extension/evaluation language.
p=r/'spec/ck.schema.json';d=json.loads(p.read_text());st={'type':'string'};arr={'type':'array','items':st};mp={'type':'object','additionalProperties':arr}
def obj(props,required=None):return {'type':'object','additionalProperties':False,'properties':props,'required':required or list(props)}
rev={'type':'integer','minimum':1}
d['$defs']['FinancialResult']=obj({'pool':st,'low':{'type':['string','null']},'high':{'type':['string','null']},'components':arr,'billing':st,'witnesses':arr})
d['$defs']['Branch']=obj({'id':st,'revision':rev,'guard':mp,'exclusion':{'type':['string','null']},'result':{'anyOf':[{'$ref':'#/$defs/FinancialResult'},{'type':'null'}]},'refs':arr})
d['$defs']['Decision']=obj({'id':st,'revision':rev,'stage':st,'dimensions':mp,'branches':{'type':'array','items':{'$ref':'#/$defs/Branch'}},'refs':arr})
d['$defs']['ProjectionField']=obj({'type':st,'source':st,'allowed':arr},['type','source'])
d['$defs']['Projection']=obj({'id':st,'revision':rev,'type_id':st,'variant':st,'external_state':st,'wire_tag':st,'wire_value':{'type':['string','boolean']},'fields':{'type':'object','additionalProperties':{'$ref':'#/$defs/ProjectionField'}},'refs':arr})
d['$defs']['ObligationBinding']=obj({'temporal':st,'witness':st,'amount_source':st,'preserve':arr})
for field,name in [('decisions','Decision'),('projections','Projection'),('boundaries','Stage'),('finalization','ObligationBinding')]:d['properties'][field]={'type':'array','items':{'$ref':'#/$defs/'+name}}
if 'finalization' not in d['required']:d['required'].append('finalization')
p.write_text(json.dumps(d,indent=2,sort_keys=True)+'\n')
