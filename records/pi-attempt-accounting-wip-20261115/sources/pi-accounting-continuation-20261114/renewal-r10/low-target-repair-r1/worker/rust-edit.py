from pathlib import Path
p=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/renewal-r10/low-target-repair-r1/subject/tooling/spec-check/tests/independent_raw_axes.rs');s=p.read_text()
s=s.replace('|| !accepts_usage_axis(&row, &c["inputs"]["sessions"][0]["launch"], &rates, a)', '''|| price_range(&row, &c["inputs"]["sessions"][0]["launch"], &rates).map(|(_,hi,_)|hi)!=Some(15_000_000)
                    || a["valuation"]!=json!({"state":"unpriceable","reasons":["IdentityWithoutRate"],"supported_low":null,"priced_high":"15000000"})
                    || a["exposure"]["low"]!=json!({"nano":"0","basis":"none"})
                    || a["exposure"]["pool"]!="pi_api"
                    || a["exposure"]["high"]["basis"]!="proxy_reservation"''')
s=s.replace('return Err("model7 actual malformed identity/conservative hold".into());','return Err("model7 actual malformed identity/conservative hold; R10 model7 unsupported minimum/priced high/conservative hold".into());')
needle='            println!("R10 evaluated predicate={} subject={} bundle={} outcome=pass",claim["relation"],selected["case_file"],selected["bundle_file"]);'
s=s.replace(needle,'''            if id=="RESPONSE-MODEL7-ACTUAL" || id=="EQUAL-AMOUNT-DISTINCT-RESERVATIONS" {
                let bundle=read(r.join(b["bundle_file"].as_str().ok_or("selected bundle")?));
                r10_selected_low_consequence(&c,b,&bundle)?;
            }
'''+needle)
s+='''

// Independently authored finite facts from raw/pinned policy; see DERIVATION.md.
// This is not a general financial or attribution reducer.
fn r10_selected_low_consequence(c:&Value,b:&Value,bundle:&Value)->Result<(),String>{
 let cid=c["case_id"].as_str().ok_or("selected case id")?;
 let eq=b["control_file"]=="cases/equal-reservations.json";
 let extra=cid=="A04-undated-settlement-between-dated-neighbours";
 let key="res:S1/turn-1:rid=1";let key2="res:S1/turn-1:rid=2";
 let usage=if eq {"S1/turn-1:3"} else {"S1/turn-1:2"};
 let docs=bundle["documents"].as_array().ok_or("selected documents")?;
 if docs.len()!=2 || docs[0]["kind"]!="accounting_input_proof" || docs[1]["kind"]!="accounting_result" {return Err("R10 selected result/input inventory".into());}
 let inp=&docs[0];let result=&docs[1];let path=b["bundle_file"].as_str().ok_or("selected bundle file")?;
 if inp["id"]!=format!("input:{cid}") || result["id"]!=format!("result:{cid}") || result["input_proof_ref"]!=inp["id"] || inp["lineage_id"]!=cid || result["lineage_id"]!=cid || inp.get("independence_frame").is_some(){return Err("R10 selected result/input join".into());}
 let attempts=c["expected"]["attempts"].as_array().ok_or("selected attempts")?;
 let keys=if extra {vec![key,key2]} else {vec![key]};
 if attempts.len()!=keys.len() || attempts.iter().zip(&keys).any(|(a,k)|a["key"]!=*k){return Err("R10 selected attempt inventory".into());}
 let origins=if eq || extra {vec!["S1/turn-1:1","S1/turn-1:2","S1/turn-1:3"]} else {vec!["S1/turn-1:1","S1/turn-1:2"]};
 let mut units=vec![json!({"unit_id":key,"state":"active","origin_refs":if eq {origins.clone()} else {origins[..2].to_vec()},"exposure_ref":format!("case:{cid}#attempt:{key}")})];
 if extra {units.push(json!({"unit_id":key2,"state":"active","origin_refs":["S1/turn-1:3"],"exposure_ref":format!("case:{cid}#attempt:{key2}")}));}
 let owners:Vec<_>=result["origins"].as_array().ok_or("selected origins")?.iter().map(|o|json!([o["origin_ref"],o["active_high_unit_id"]])).collect();
 let expected_owners:Vec<_>=origins.iter().map(|o|json!([o,if extra && *o=="S1/turn-1:3" {key2} else {key}])).collect();
 if result["units"]!=json!(units) || owners!=expected_owners {return Err("R10 selected unit/origin join".into());}
 for (i,a) in attempts.iter().enumerate(){if a["accounting_result_ref"]!=format!("{path}#result:{cid}") || a["evidence"]!=units[i]["origin_refs"]{return Err("R10 selected result/unit reference".into());}}
 if !eq {
  if attempts[0]["obligation"]!=json!({"state":"closed","amount_source":"reservation_bound","by":usage}) || attempts[0]["settlement"]["decision"]!="valid" {return Err("R10 selected model7 lawful temporal closure".into());}
  if extra && (attempts[1]["obligation"]!=json!({"state":"outstanding","amount_source":"reservation_bound"}) || attempts[1]["exposure"]!=json!({"pool":"pi_api","low":{"basis":"none","nano":"0"},"high":{"kind":"at_least","nano":"300000000","basis":"proxy_reservation"}})){return Err("R10 selected separate reservation300M".into());}
  let mut ms=vec![];let mut vs=vec![];
  let mut roles=vec![(key,vec!["S1/turn-1:1"],vec![usage])];if extra {roles.push((key2,vec!["S1/turn-1:3"],vec![]));}
  for (k,rs,ss) in roles {
   let classes=|os:&Vec<&str>|os.iter().map(|o|json!({"origin_refs":[o],"representative_origin_ref":o})).collect::<Vec<_>>();
   ms.push(json!({"obligation_id":format!("validity:{k}"),"reservation_classes":classes(&rs),"settlement_classes":classes(&ss),"usage_origin_refs":ss}));
   vs.push(json!({"obligation_id":format!("validity:{k}"),"distinct_reservation_refs":rs,"distinct_settlement_refs":ss,"validity_result":if ss.is_empty(){"absent"}else{"valid"},"applicable_check_refs":[format!("case:{cid}#attempt:{k}/settlement")]}));
  }
  if result["primary_memberships"]!=json!(ms) || result["primary_validities"]!=json!(vs) || result["coverage_decisions"]!=json!([]){return Err("R10 selected model7 membership/validity join".into());}
 }
 if result["valuation_records"]!=json!([{"valuation_id":format!("valuation:{key}"),"usage_origin_refs":[usage],"complete":eq,"wire_ref":format!("case:{cid}#attempt:{key}/valuation")}]){return Err("R10 selected valuation/wire join".into());}
 let evs=c["evaluations"].as_array().ok_or("selected evaluations")?;
 if evs.len()!=1 || evs[0]["name"]!="default" {return Err("R10 selected evaluation inventory".into());}
 let ev=&evs[0];let obs=ev["expected"]["objectives"].as_array().ok_or("selected expected objectives")?;
 if ev["objectives"].as_array().map(Vec::len)!=Some(1) || ev["objectives"][0]["objective"]!="o" || obs.len()!=1 || obs[0]["objective"]!="o" {return Err("R10 selected objective inventory".into());}
 let ats:Vec<_>=keys.iter().map(|k|json!({"attempt":k,"objective":"o","ownership":"owned","placement":"inside"})).collect();
 if ev["expected"]["attribution"]!=json!(ats){return Err("R10 selected raw ownership/placement".into());}
 let mut lows=vec![];let mut assertions=vec![];
 for label in ["owned_closed","owned_outstanding"]{
  let positive=eq && label=="owned_outstanding";
  let amount=if positive {"15000000"}else{"0"};
  let has_usage=if eq {label=="owned_outstanding"} else {label=="owned_closed"};
  let inventory=if has_usage {vec![usage]} else {vec![]};
  let scope=format!("{cid}/default/o/{label}");
  lows.push(json!({"scope_ref":scope,"selected_low":amount,"single_floor":amount,"independent_floor":"0","basis":if positive{"usage_valuation"}else{"none"},"credit_mode":if positive{"single_ambiguous"}else{"none"},"credits":[],"seed_origin_refs":inventory,"alternative_origin_refs":inventory}));
  let high=if (eq && label=="owned_outstanding") || (!eq && label=="owned_closed") {json!({"kind":"at_least","nano":"500000000"})} else if extra && label=="owned_outstanding" {json!({"kind":"at_least","nano":"300000000"})} else {json!({"kind":"exact","nano":"0"})};
  if obs[0][label]["supported_low_evidence_ref"]!=format!("{path}#{scope}"){return Err("R10 selected low reference join".into());}
  if obs[0][label]["low"]!=amount || obs[0][label]["high"]!=high {return Err("R10 selected independent low/result/rollup".into());}
  let owned=if has_usage {vec![key]}else if extra && label=="owned_outstanding"{vec![key2]}else{vec![]};
  assertions.push(json!({"evaluation":"default","objective":"o","label":label,"expected_low":amount,"seed_origin_refs":inventory,"alternative_origin_refs":inventory,"owned_unit_refs":owned}));
 }
 if b["low_assertions"]!=json!(assertions){return Err("R10 selected independent assertion inventory".into());}
 // Full-list comparison preserves uniqueness and total inventory before maps exist.
 if result["low_rollups"]!=json!(lows){return Err("R10 selected complete low witness inventory/fields".into());}
 Ok(())
}
'''
p.write_text(s)
