import json,pathlib
p=pathlib.Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/realization/s1-r9-pilot/spec')
s=json.loads((p/'accounting.schema.json').read_text());d=s['$defs'];st={'type':'string','minLength':1};arr={'type':'array','items':st,'uniqueItems':True}
def obj(props):return {'type':'object','additionalProperties':False,'properties':props,'required':list(props)}
d['PrimaryEvidenceClass']=obj({'origin_refs':dict(arr,minItems=1),'representative_origin_ref':st})
d['PrimaryMembership']=obj({'obligation_id':st,'reservation_classes':{'type':'array','items':{'$ref':'#/$defs/PrimaryEvidenceClass'},'uniqueItems':True},'settlement_classes':{'type':'array','items':{'$ref':'#/$defs/PrimaryEvidenceClass'},'uniqueItems':True},'usage_origin_refs':arr})
d['AccountingResult']['properties']['primary_memberships']={'type':'array','items':{'$ref':'#/$defs/PrimaryMembership'},'uniqueItems':True}
d['AccountingResult']['allOf']=[{'if':{'properties':{'primary_validities':{'minItems':1}}},'then':{'required':['primary_memberships']}}]
for k in ['InputLink','DefiniteLink']:
 d[k]['properties']['link_id']=st;d[k]['required'].append('link_id')
d['UnbudgetedCompletion']['properties']['usage_origin_refs']=dict(arr,minItems=1);d['UnbudgetedCompletion']['required'].append('usage_origin_refs')
pair=obj({'from':st,'to':st});d['Grant']['properties']['admitted_payload']={'oneOf':[pair,obj({'pairs':{'type':'array','minItems':1,'uniqueItems':True,'items':pair}})]}
d['Grant']['allOf']=[{'if':{'properties':{'guarantee_kind':{'const':'exact_row_coreference'}}},'then':{'required':['admitted_payload']}}]
(p/'accounting.schema.json').write_text(json.dumps(s,indent=2,sort_keys=True)+'\n')
