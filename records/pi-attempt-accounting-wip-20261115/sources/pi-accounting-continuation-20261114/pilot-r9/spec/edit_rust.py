from pathlib import Path
p=Path('/home/bline/cbm-work/pi-accounting-continuation-20261114/realization/s1-r9-pilot/tooling/spec-check/src/accounting.rs');s=p.read_text()
a=s.index('    for l in rows(input, "exact_links")? {');b=s.index('    if let Some(f)',a)
s=s[:a]+'''    let links = keyed(input, "exact_links", "link_id")?;
    let grants = keyed(input, "grants", "grant_id")?;
    for l in links.values() {
        let source = name(l, "from_origin_ref")?;
        let target = name(l, "to_origin_ref")?;
        if source == target { return Err("self-link cannot establish co-reference".into()); }
        let r = BTreeSet::from([source.into(), target.into()]);
        subset(&r, &os, "link origin")?;
        let id = name(l, "grant_ref")?;
        grant(input, id, "exact_row_coreference", &r)?;
        let payload = &grants[id]["admitted_payload"];
        let pairs: Vec<&Value> = if payload.get("from").is_some() {
            vec![payload]
        } else {
            rows(payload, "pairs")?.iter().collect()
        };
        if !pairs.iter().any(|p| p["from"] == source && p["to"] == target) {
            return Err("input link pair not admitted by exact grant payload".into());
        }
    }
'''+s[b:]
a=s.index('    for v in validities.values() {')
s=s[:a]+'''    let memberships = if result.get("primary_memberships").is_some() {
        keyed(result, "primary_memberships", "obligation_id")?
    } else { BTreeMap::new() };
    if memberships.keys().collect::<Vec<_>>() != validities.keys().collect::<Vec<_>>() {
        return Err("primary validity/membership inventory mismatch".into());
    }
    let mut primary_owner = BTreeMap::new();
    let mut primary_sets = BTreeMap::new();
    let mut usage_sets = BTreeMap::new();
    for (obligation, m) in &memberships {
        let mut all = BTreeSet::new();
        for (classes, distinct) in [("reservation_classes", "distinct_reservation_refs"),
                                     ("settlement_classes", "distinct_settlement_refs")] {
            let mut representatives = BTreeSet::new();
            for class in rows(m, classes)? {
                let members = refs(class, "origin_refs")?;
                subset(&members, &os, "primary membership origin")?;
                let ordered: Vec<_> = rows(class, "origin_refs")?.iter().map(|v| v.as_str().unwrap()).collect();
                if ordered != members.iter().map(String::as_str).collect::<Vec<_>>()
                    || members.first().map(String::as_str) != Some(name(class, "representative_origin_ref")?) {
                    return Err("primary class is not canonical or representative is not minimum member".into());
                }
                if members.len() > 1 {
                    let first = inv[members.first().unwrap()];
                    let contracts = keyed(input, "initial_contracts", "contract_id")?;
                    let contract = contracts[name(first, "initial_contract_ref")?];
                    if contract["duplicate_rule"] != "scope_exact_line" || contract.get("scope_id").is_none()
                        || !members.iter().all(|o| {
                            let c = contracts.get(inv[o]["initial_contract_ref"].as_str().unwrap());
                            inv[o]["raw_bytes_sha256"] == first["raw_bytes_sha256"]
                                && c.is_some_and(|c| c["duplicate_rule"] == "scope_exact_line" && c["scope_id"] == contract["scope_id"])
                        })
                        || !rows(input, "grants")?.iter().any(|g| {
                            g["guarantee_kind"] == "exact_copy_equivalence"
                                && refs(g, "covered_origin_refs").is_ok_and(|r| members.is_subset(&r))
                        }) {
                        return Err("primary class lacks admitted exact-copy equivalence".into());
                    }
                }
                representatives.insert(name(class, "representative_origin_ref")?.to_owned());
                for o in members {
                    if primary_owner.insert(o.clone(), obligation.clone()).is_some() {
                        return Err("primary origin belongs to multiple classes/roles/obligations".into());
                    }
                    all.insert(o);
                }
            }
            if representatives != refs(validities[obligation], distinct)? {
                return Err("primary validity representatives differ from membership".into());
            }
        }
        let usage = refs(m, "usage_origin_refs")?;
        subset(&usage, &os, "primary usage membership")?;
        primary_sets.insert(obligation.clone(), all);
        usage_sets.insert(obligation.clone(), usage);
    }
'''+s[a:]
a=s.index('    for l in rows(result, "definite_links")? {');b=s.index('    let _ = keyed(result, "coverage_decisions"',a)
s=s[:a]+'''    let links = keyed(result, "definite_links", "link_id")?;
    let input_links = keyed(input, "exact_links", "link_id")?;
    for l in links.values() {
        let admitted = input_links.get(name(l, "admitted_input_link_ref")?)
            .ok_or("result link not admitted by exact input link identity")?;
        if l["from_origin_ref"] != admitted["from_origin_ref"]
            || l["to_origin_ref"] != admitted["to_origin_ref"] {
            return Err("result link endpoints differ from exact admitted link".into());
        }
    }
'''+s[b:]
s=s.replace('        let mut origins = refs(c, "monetary_origin_refs")?;\n        origins.extend(refs(c, "anchor_origin_refs")?);','''        let monetary = refs(c, "monetary_origin_refs")?;
        let anchors = refs(c, "anchor_origin_refs")?;
        if !monetary.is_disjoint(&anchors) { return Err("monetary and anchor coverage overlap".into()); }
        let mut origins = monetary.clone();
        origins.extend(anchors.clone());''')
s=s.replace('            if c["route"] == "budgeted" {','''            let expected_usage;
            let mut request_origins;
            if c["route"] == "budgeted" {''',1)
s=s.replace('                    return Err("coverage obligation/validity reference mismatch".into());\n                }','''                    return Err("coverage obligation/validity reference mismatch".into());
                }
                let obligation = name(c, "obligation_id")?;
                subset(&primary_sets[obligation], &monetary, "coverage missing selected primary inventory")?;
                if monetary.iter().any(|o| primary_owner.get(o).is_some_and(|id| id != obligation)) {
                    return Err("coverage includes another obligation primary origin".into());
                }
                expected_usage = usage_sets[obligation].clone();
                subset(&expected_usage, &origins, "coverage missing selected usage inventory")?;
                request_origins = primary_sets[obligation].clone();
                request_origins.extend(expected_usage.clone());''',1)
s=s.replace('                for primary in validities.values() {\n                    let mut dollars = refs(primary, "distinct_reservation_refs")?;\n                    dollars.extend(refs(primary, "distinct_settlement_refs")?);','''                expected_usage = refs(completion, "usage_origin_refs")?;
                if !expected_usage.contains(usage) { return Err("completion usage inventory omits its usage origin".into()); }
                subset(&expected_usage, &origins, "completion usage outside coverage")?;
                request_origins = expected_usage.clone();
                for dollars in primary_sets.values() {''')
a=s.index('            subset(\n                &refs(valuation, "usage_origin_refs")?');b=s.index('            let resolved =',a)
s=s[:a]+'''            if refs(valuation, "usage_origin_refs")? != expected_usage {
                return Err("coverage valuation usage differs from selected membership inventory".into());
            }
'''+s[b:]
a=s.index('            for link_ref in refs(c, "definite_link_refs")?');b=s.index('\n        }\n    }\n    for l in rows(result, "low_rollups")?',a)
s=s[:a]+'''            let cited = refs(c, "definite_link_refs")?;
            for link_ref in &cited {
                if !links.contains_key(link_ref) {
                    return Err("dangling coverage definite-link identity".into());
                }
            }
            for a in anchors {
                if !cited.iter().any(|id| {
                    let l = links[id];
                    let source = l["from_origin_ref"].as_str().unwrap();
                    let target = l["to_origin_ref"].as_str().unwrap();
                    (source == a && target != a && request_origins.contains(target) && origins.contains(target))
                        || (target == a && source != a && request_origins.contains(source) && origins.contains(source))
                }) {
                    return Err("released anchor lacks cited exact link to selected covered request".into());
                }
            }'''+s[b:]
p.write_text(s)
