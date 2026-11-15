from pathlib import Path
p=Path('/home/bline/cbm-work/p137-s1r7/s1/tooling/spec-check/src/lib.rs');s=p.read_text()
s=s.replace('pub type Result<T>', 'mod decisions;\npub use decisions::*;\npub type Result<T>')
s=s.replace('    pub owner_policies: Vec<Node>,','    pub owner_policies: Vec<Node>,\n    pub decisions: Vec<Decision>,\n    pub projections: Vec<Projection>,\n    pub boundaries: Vec<Stage>,')
s=s.replace('.chain(s.owner_policies.iter().map(|x| (&x.id, json(x))))','.chain(s.owner_policies.iter().map(|x| (&x.id, json(x))))\n        .chain(s.decisions.iter().map(|x| (&x.id,json(x))))\n        .chain(s.projections.iter().map(|x| (&x.id,json(x))))\n        .chain(s.boundaries.iter().map(|x| (&x.id,json(x))))')
s=s.replace('edges.insert(t.id.clone(), vec![]);','edges.insert(t.id.clone(), t.variants.iter().flat_map(|v| v.fields.values()).filter_map(|ty| named_type(ty, s)).collect());')
s=s.replace('    for st in &s.stages {\n        edges.insert', '    for d in &s.decisions { edges.insert(d.id.clone(),d.refs.clone()); }\n    for p in &s.projections { edges.insert(p.id.clone(),p.refs.clone()); }\n    for b in &s.boundaries { edges.insert(b.id.clone(),b.reads.clone()); }\n    for st in &s.stages {\n        edges.insert')
s=s.replace('pi-accounting-contract/1.5-s1','pi-accounting-contract/1.6-s1')
s=s.replace('.chain(s.owner_policies.iter().map(|x| x.id.as_str()));','.chain(s.owner_policies.iter().map(|x| x.id.as_str()))\n        .chain(s.decisions.iter().map(|x| x.id.as_str()))\n        .chain(s.projections.iter().map(|x| x.id.as_str()))\n        .chain(s.boundaries.iter().map(|x| x.id.as_str()));')
a=s.index('        (\n            "F-OBLIGATION",');b=s.index('        ("K-FINDINGS"',a);s=s[:a]+s[b:]
s=s.replace('    node_digests(s)?;','    validate_decisions(s)?;\n    node_digests(s)?;',1)
s=s.replace('    Ok(())\n}\nfn validate_value','    validate_cross_fields(type_id, variant, value)?;\n    Ok(())\n}\nfn validate_value')
s=s.replace('        body.push_str("## Serialization interface', '        for d in &s.decisions { body.push_str(&format!("## Result-bearing decision {}\\n\\n```json\\n{}\\n```\\n\\n",d.id,serde_json::to_string_pretty(d).unwrap())); }\n        for p in &s.projections { body.push_str(&format!("Projection: `{}`\\n\\n",serde_json::to_string(p).unwrap())); }\n        body.push_str("## Serialization interface')
# Replace first-enum constructor sampling test with all legal explicit projection cases, exhaustive nested enum cases tested separately.
a=s.index('    fn sample(s: &Spec, ty: &str)');b=s.index('\n}\n\n#[cfg(test)]\nmod generation_controls',a)
s=s[:a]+'''    #[test]
    fn all_declared_projection_cases_validate() {
        let s=fixture();
        validate_decisions(&s).unwrap();
        for p in &s.projections { assert!(!p.external_state.is_empty()); }
        let none=serde_json::json!({"amount_source":{"variant":"none","fields":{}},"cause":{"variant":"subscription","fields":{}}});
        validate_constructor(&s,"Obligation","none",&none).unwrap();
        for source in ["settlement_amount","reservation_confirmed","reservation_bound"] {
            let mut bad=none.clone();bad["amount_source"]["variant"]=source.into();
            assert!(validate_constructor(&s,"Obligation","none",&bad).is_err());
            let closed=serde_json::json!({"amount_source":{"variant":source,"fields":{}},"by":"settlement:1"});
            validate_constructor(&s,"Obligation","closed",&closed).unwrap();
        }
    }
'''+s[b:]
p.write_text(s)
