//! Structural specification checks and deterministic views only. No raw-attempt evaluator.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};
mod accounting;
mod applicability;
pub use applicability::*;
mod corpus;
pub use corpus::*;
mod decisions;
pub use accounting::*;
pub use decisions::*;
pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub schema_version: String,
    pub contract_version: String,
    pub types: Vec<TypeDef>,
    pub stages: Vec<Stage>,
    pub rules: Vec<Rule>,
    pub formulas: Vec<Formula>,
    pub invariants: Vec<Node>,
    pub views: Vec<View>,
    pub owner_policies: Vec<Node>,
    pub decisions: Vec<Decision>,
    pub projections: Vec<Projection>,
    pub boundaries: Vec<Stage>,
    pub finalization: Vec<ObligationBinding>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeDef {
    pub id: String,
    pub revision: u32,
    pub variants: Vec<Variant>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub name: String,
    pub fields: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub id: String,
    pub revision: u32,
    pub reads: Vec<String>,
    pub outputs: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub revision: u32,
    pub stage: String,
    pub partition: String,
    pub dimensions: BTreeMap<String, Vec<String>>,
    pub guard: BTreeMap<String, Vec<String>>,
    pub text: String,
    pub reads: Vec<String>,
    pub outputs: Vec<Constructor>,
    pub formulas: Vec<String>,
    pub refs: Vec<String>,
    pub depends: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constructor {
    pub type_id: String,
    pub variant: String,
    pub fields: Vec<String>,
    #[serde(default)]
    pub bindings: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Formula {
    pub id: String,
    pub revision: u32,
    pub text: String,
    pub inputs: Vec<String>,
    pub refs: Vec<String>,
    pub depends: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub revision: u32,
    pub text: String,
    pub refs: Vec<String>,
    pub depends: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub path: String,
    pub nodes: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub schema: String,
    pub consumers: Vec<Consumer>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub id: String,
    pub owner: String,
    pub dependencies: Vec<String>,
    pub acknowledgement: Option<Acknowledgement>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acknowledgement {
    pub digest: String,
    pub disposition: String,
    pub authority: String,
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn read_spec(root: &Path) -> Result<Spec> {
    let schema = fs::read(root.join("spec/ck.schema.json")).map_err(|e| e.to_string())?;
    if schema != include_bytes!("../../../spec/ck.schema.json") {
        return Err("compiled fixed schema differs from runtime schema".into());
    }
    let b = fs::read(root.join("spec/ck.json")).map_err(|e| e.to_string())?;
    let value: Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
    validate_schema(
        &serde_json::from_slice(&schema).map_err(|e| e.to_string())?,
        &value,
    )?;
    serde_json::from_value(value).map_err(|e| format!("fixed schema: {e}"))
}
fn json<T: Serialize>(v: &T) -> Value {
    serde_json::to_value(v).expect("serializable declarations")
}
fn body_nodes(s: &Spec) -> BTreeMap<String, Value> {
    let mut n = BTreeMap::new();
    for (id, v) in s
        .types
        .iter()
        .map(|x| (&x.id, json(x)))
        .chain(s.stages.iter().map(|x| (&x.id, json(x))))
        .chain(s.rules.iter().map(|x| (&x.id, json(x))))
        .chain(s.formulas.iter().map(|x| (&x.id, json(x))))
        .chain(s.invariants.iter().map(|x| (&x.id, json(x))))
        .chain(s.owner_policies.iter().map(|x| (&x.id, json(x))))
        .chain(s.decisions.iter().map(|x| (&x.id, json(x))))
        .chain(s.projections.iter().map(|x| (&x.id, json(x))))
        .chain(s.boundaries.iter().map(|x| (&x.id, json(x))))
    {
        n.insert(id.clone(), v);
    }
    if let Some(body) = n.get_mut("F-PROJECTION") {
        body.as_object_mut()
            .expect("node object")
            .insert("finalization".into(), json(&s.finalization));
    }
    n
}
fn node_edges(s: &Spec) -> BTreeMap<String, Vec<String>> {
    let mut edges = BTreeMap::new();
    for t in &s.types {
        edges.insert(
            t.id.clone(),
            t.variants
                .iter()
                .flat_map(|v| v.fields.values())
                .filter_map(|ty| named_type(ty, s))
                .collect(),
        );
    }
    for d in &s.decisions {
        edges.insert(
            d.id.clone(),
            d.refs
                .iter()
                .chain(d.branches.iter().flat_map(|b| b.refs.iter()))
                .cloned()
                .collect(),
        );
    }
    for p in &s.projections {
        edges.insert(p.id.clone(), p.refs.clone());
    }
    for b in &s.boundaries {
        edges.insert(b.id.clone(), b.reads.clone());
    }
    for st in &s.stages {
        let mut reads = st.reads.clone();
        if st.id == "C" {
            reads.extend(["C-validity".into(), "C-auxiliary".into()]);
        }
        if st.id == "S" {
            reads.push("S-consume".into());
        }
        if st.id == "V" {
            reads.extend([
                "USAGE-PRESENCE".into(),
                "VALUATION".into(),
                "RAW-FIELD-PREDICATES".into(),
                "FINANCIAL-CANDIDATES".into(),
            ]);
        }
        if st.id == "F" {
            reads.push("WHOLE-COVERAGE".into());
        }
        if st.id == "X" {
            reads.extend(["ATOMIZATION".into(), "SUPPORTED-LOW".into()]);
        }
        edges.insert(st.id.clone(), reads);
    }
    for n in s.invariants.iter().chain(&s.owner_policies) {
        edges.insert(
            n.id.clone(),
            n.refs.iter().chain(&n.depends).cloned().collect(),
        );
    }
    for f in &s.formulas {
        edges.insert(
            f.id.clone(),
            f.refs
                .iter()
                .chain(&f.depends)
                .cloned()
                .chain(
                    f.inputs
                        .iter()
                        .filter_map(|v| v.split_once('.').map(|(stage, _)| stage.to_string())),
                )
                .collect(),
        );
    }
    for r in &s.rules {
        edges.insert(
            r.id.clone(),
            r.refs
                .iter()
                .chain(&r.depends)
                .chain(&r.formulas)
                .chain(&r.reads)
                .chain(std::iter::once(&r.stage))
                .chain(r.outputs.iter().map(|o| &o.type_id))
                .cloned()
                .collect(),
        );
    }
    edges
}
fn digest_node(
    id: &str,
    bodies: &BTreeMap<String, Value>,
    edges: &BTreeMap<String, Vec<String>>,
    stack: &mut BTreeSet<String>,
    cache: &mut BTreeMap<String, String>,
) -> Result<String> {
    if let Some(d) = cache.get(id) {
        return Ok(d.clone());
    }
    if !stack.insert(id.into()) {
        return Err(format!("dependency cycle at {id}"));
    }
    let body = bodies
        .get(id)
        .ok_or_else(|| format!("unresolved node {id}"))?;
    let mut deps = BTreeMap::new();
    for d in edges.get(id).into_iter().flatten() {
        deps.insert(d.clone(), digest_node(d, bodies, edges, stack, cache)?);
    }
    let bytes = serde_json::to_vec(&(body, deps)).map_err(|e| e.to_string())?;
    let digest = sha256(&bytes);
    cache.insert(id.into(), digest.clone());
    stack.remove(id);
    Ok(digest)
}
pub fn node_digests(s: &Spec) -> Result<BTreeMap<String, String>> {
    let bodies = body_nodes(s);
    let edges = node_edges(s);
    let mut cache = BTreeMap::new();
    for id in bodies.keys() {
        digest_node(id, &bodies, &edges, &mut BTreeSet::new(), &mut cache)?;
    }
    Ok(cache)
}
fn unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for v in values {
        if v.is_empty() || !seen.insert(v) {
            return Err(format!("duplicate/empty {label}: {v}"));
        }
    }
    Ok(())
}
fn valid_type(t: &str, defs: &BTreeSet<&str>) -> bool {
    if let Some(inner) = t
        .strip_prefix("List<")
        .or_else(|| t.strip_prefix("Option<"))
    {
        return inner.strip_suffix('>').is_some_and(|v| valid_type(v, defs));
    }
    matches!(t, "String" | "Nano" | "Bool" | "Json" | "U64") || defs.contains(t)
}
fn product(dims: &BTreeMap<String, Vec<String>>) -> Result<Vec<BTreeMap<String, String>>> {
    let mut points = vec![BTreeMap::new()];
    for (k, vs) in dims {
        if vs.is_empty() {
            return Err(format!("empty dimension {k}"));
        }
        unique(vs.iter().map(String::as_str), "dimension value")?;
        if points.len().saturating_mul(vs.len()) > 100_000 {
            return Err("guard domain exceeds bounded structural limit".into());
        }
        points = points
            .into_iter()
            .flat_map(|point| {
                vs.iter().map(move |v| {
                    let mut p = point.clone();
                    p.insert(k.clone(), v.clone());
                    p
                })
            })
            .collect();
    }
    Ok(points)
}
pub fn validate(s: &Spec) -> Result<()> {
    if s.schema_version != "pi-accounting-spec/1"
        || s.contract_version != "pi-accounting-contract/1.7-s1"
    {
        return Err("unsupported schema/contract version".into());
    }
    validate_schema(
        &serde_json::from_slice(include_bytes!("../../../spec/ck.schema.json"))
            .map_err(|e| e.to_string())?,
        &json(s),
    )?;
    for (id, bytes) in [
        (
            "SCHEMA-ACCOUNTING",
            include_bytes!("../../../spec/accounting.schema.json").as_slice(),
        ),
        (
            "SCHEMA-WIRE",
            include_bytes!("../../../spec/wire.schema.json").as_slice(),
        ),
    ] {
        let owner = s
            .invariants
            .iter()
            .find(|n| n.id == id)
            .ok_or("missing schema declaration owner")?;
        let declared: Value = serde_json::from_str(&owner.text).map_err(|e| e.to_string())?;
        let actual: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if declared != actual {
            return Err(format!("stale schema declaration projection {id}"));
        }
    }
    let ids = s
        .types
        .iter()
        .map(|x| x.id.as_str())
        .chain(s.stages.iter().map(|x| x.id.as_str()))
        .chain(s.rules.iter().map(|x| x.id.as_str()))
        .chain(s.formulas.iter().map(|x| x.id.as_str()))
        .chain(s.invariants.iter().map(|x| x.id.as_str()))
        .chain(s.owner_policies.iter().map(|x| x.id.as_str()))
        .chain(s.decisions.iter().map(|x| x.id.as_str()))
        .chain(s.projections.iter().map(|x| x.id.as_str()))
        .chain(s.boundaries.iter().map(|x| x.id.as_str()));
    unique(ids, "node ID")?;
    let bodies = body_nodes(s);
    for (id, v) in &bodies {
        if v.get("revision").and_then(Value::as_u64) == Some(0) {
            return Err(format!("zero revision {id}"));
        }
    }
    let defs = s.types.iter().map(|t| t.id.as_str()).collect();
    for t in &s.types {
        if t.variants.is_empty() {
            return Err(format!("no variants {}", t.id));
        }
        unique(t.variants.iter().map(|v| v.name.as_str()), "variant")?;
        for v in &t.variants {
            for ty in v.fields.values() {
                if !valid_type(ty, &defs) {
                    return Err(format!("unknown field type {ty}"));
                }
            }
        }
    }
    // The domain is deliberately fixed, not an extensible graph language.
    let fixed = [
        ("E", vec![]),
        ("C", vec!["E"]),
        ("S", vec!["C"]),
        ("V", vec!["C"]),
        ("F", vec!["C", "S", "V"]),
        ("X", vec!["C", "S", "V", "F"]),
        ("K", vec!["C", "S", "V", "F", "X"]),
    ];
    if s.stages.len() != fixed.len() {
        return Err("fixed stage set changed".into());
    }
    for (id, allowed) in fixed {
        let st = s
            .stages
            .iter()
            .find(|x| x.id == id)
            .ok_or_else(|| format!("missing stage {id}"))?;
        if st.reads.iter().map(String::as_str).collect::<BTreeSet<_>>()
            != allowed.into_iter().collect()
        {
            return Err(format!("illegal declared read-set {id}"));
        }
    }
    let expected_partitions: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = [
        (
            "C-KEY",
            [(
                "key_fact",
                vec![
                    "usable_reservation",
                    "unusable_reservation",
                    "settlement_no_reservation",
                    "unbudgeted",
                    "subscription",
                    "auxiliary",
                    "ambiguous",
                ],
            )]
            .into(),
        ),
        (
            "S-DECISION",
            [
                ("reservations", vec!["zero", "unique", "conflicting"]),
                ("settlements", vec!["zero", "one", "many"]),
                ("checks", vec!["pass", "fail"]),
            ]
            .into(),
        ),
        (
            "S-TIME",
            [(
                "temporal_fact",
                vec!["no_reservation", "valid_exact_join", "other_reserved"],
            )]
            .into(),
        ),
        (
            "V-PRESENCE",
            [(
                "usage_fact",
                vec![
                    "absent_or_null",
                    "all_billing_absent_null",
                    "auxiliary_only",
                    "schema_conflict",
                    "nonobject",
                    "unknown_schema",
                    "malformed_billing",
                    "missing_required",
                    "reported",
                ],
            )]
            .into(),
        ),
        (
            "F-REPLACEMENT",
            [(
                "replacement_fact",
                vec![
                    "no_reservation",
                    "not_valid_usage",
                    "usage_unpriceable",
                    "usage_incomplete",
                    "usage_bound",
                ],
            )]
            .into(),
        ),
        ("K-FINDINGS", [("predicate", vec!["true", "false"])].into()),
    ]
    .into();
    let fs = s
        .formulas
        .iter()
        .map(|x| x.id.as_str())
        .collect::<BTreeSet<_>>();
    for f in &s.formulas {
        if f.text.trim().is_empty() || f.inputs.is_empty() {
            return Err(format!("empty formula {}", f.id));
        }
        unique(f.inputs.iter().map(String::as_str), "formula input")?;
        for input in &f.inputs {
            if let Some((stage, field)) = input.split_once('.') {
                if stage == "K" {
                    return Err("classification feeding money".into());
                }
                if !s
                    .stages
                    .iter()
                    .any(|x| x.id == stage && x.outputs.iter().any(|o| o == field))
                {
                    return Err(format!("unknown formula symbol {input}"));
                }
            } else if ![
                "input",
                "cached",
                "write",
                "output",
                "r_in",
                "r_cached",
                "r_write",
                "r_out",
                "tier",
                "cache_read",
                "w5m",
                "w1h",
                "cache_write",
                "r_read",
                "r_5m",
                "r_1h",
                "producer_price",
                "charged",
                "components",
                "B",
                "owned_high",
            ]
            .contains(&input.as_str())
            {
                return Err(format!("unknown primitive formula symbol {input}"));
            }
        }
    }
    let mut partitions: BTreeMap<&str, Vec<&Rule>> = BTreeMap::new();
    for r in &s.rules {
        let expected = expected_partitions
            .get(r.partition.as_str())
            .ok_or("unknown finite partition")?;
        let dimensions = r
            .dimensions
            .iter()
            .map(|(k, v)| (k.as_str(), v.iter().map(String::as_str).collect::<Vec<_>>()))
            .collect::<BTreeMap<_, _>>();
        if &dimensions != expected {
            return Err(format!("changed finite partition domain {}", r.partition));
        }
        let stage = s
            .stages
            .iter()
            .find(|x| x.id == r.stage)
            .ok_or_else(|| format!("unknown rule stage {}", r.stage))?;
        if r.reads.iter().any(|x| !stage.reads.contains(x)) {
            return Err(format!("illegal rule read-set {}", r.id));
        }
        if r.stage != "K"
            && (r.reads.iter().any(|x| x == "K") || r.formulas.iter().any(|x| x.starts_with("K.")))
        {
            return Err("classification feeding money".into());
        }
        for f in &r.formulas {
            if !fs.contains(f.as_str()) {
                return Err(format!("missing formula {f}"));
            }
            let formula = s.formulas.iter().find(|x| &x.id == f).unwrap();
            for input in &formula.inputs {
                if let Some((source, _)) = input.split_once('.') {
                    if !stage.reads.iter().any(|x| x == source) {
                        return Err(format!("formula reads undeclared stage {}: {input}", r.id));
                    }
                }
            }
        }
        if r.partition == "F-REPLACEMENT" {
            validate_replacement_binding(r)?;
        }
        for o in &r.outputs {
            if r.stage == "S" && o.type_id == "SettlementDecision" {
                return Err("S cannot construct C-validity decision".into());
            }
            if r.partition == "S-DECISION"
                && (o.type_id != "SettlementForward"
                    || o.bindings.get("validity_ref").and_then(Value::as_str)
                        != Some("C-validity.validity_ref"))
            {
                return Err("S must forward immutable validity_ref".into());
            }
            let t = s
                .types
                .iter()
                .find(|x| x.id == o.type_id)
                .ok_or_else(|| format!("unknown constructor {}", o.type_id))?;
            let v = t
                .variants
                .iter()
                .find(|v| v.name == o.variant)
                .ok_or_else(|| format!("unknown variant {}", o.variant))?;
            unique(o.fields.iter().map(String::as_str), "constructor field")?;
            if o.fields.iter().collect::<BTreeSet<_>>() != v.fields.keys().collect() {
                return Err(format!("constructor field mismatch {}.{}", t.id, v.name));
            }
        }
        if r.guard.keys().collect::<BTreeSet<_>>() != r.dimensions.keys().collect() {
            return Err(format!("incomplete guard {}", r.id));
        }
        for (d, vs) in &r.guard {
            if vs.is_empty() || vs.iter().any(|x| !r.dimensions[d].contains(x)) {
                return Err(format!("illegal guard selector {}", r.id));
            }
            unique(vs.iter().map(String::as_str), "guard value")?;
        }
        partitions.entry(&r.partition).or_default().push(r);
    }
    if partitions.keys().copied().collect::<BTreeSet<_>>()
        != expected_partitions.keys().copied().collect()
    {
        return Err("missing finite partition".into());
    }
    for (id, rs) in partitions {
        let first = rs[0];
        for r in &rs {
            if r.stage != first.stage || r.dimensions != first.dimensions {
                return Err(format!("partition declaration mismatch {id}"));
            }
        }
        for point in product(&first.dimensions)? {
            let count = rs
                .iter()
                .filter(|r| point.iter().all(|(k, v)| r.guard[k].contains(v)))
                .count();
            if count != 1 {
                return Err(format!("guard gap/overlap {id} {point:?}: {count}"));
            }
        }
    }
    for r in s.rules.iter().filter(|r| r.partition == "S-DECISION") {
        if r.outputs.len() != 1 {
            return Err("S requires one forwarding output".into());
        }
    }
    validate_decisions(s)?;
    node_digests(s)?;
    unique(s.views.iter().map(|v| v.path.as_str()), "view path")?;
    let allowed = [
        "contract/CK-GENERATED.md",
        "contract/TRANSITIONS.md",
        "corpus/SCHEMA.md",
    ];
    for v in &s.views {
        if !allowed.contains(&v.path.as_str())
            || v.nodes
                != match v.path.as_str() {
                    "contract/CK-GENERATED.md" => "contract",
                    "contract/TRANSITIONS.md" => "transitions",
                    "corpus/SCHEMA.md" => "schema",
                    _ => "invalid",
                }
        {
            return Err(format!("unapproved view destination {}", v.path));
        }
        if Path::new(&v.path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("unsafe view path".into());
        }
    }
    Ok(())
}
pub fn validate_constructor(s: &Spec, type_id: &str, variant: &str, value: &Value) -> Result<()> {
    let t = s
        .types
        .iter()
        .find(|t| t.id == type_id)
        .ok_or_else(|| format!("unknown type {type_id}"))?;
    let v = t
        .variants
        .iter()
        .find(|v| v.name == variant)
        .ok_or_else(|| format!("unknown variant {variant}"))?;
    let obj = value
        .as_object()
        .ok_or("constructor value must be object")?;
    if obj.keys().collect::<BTreeSet<_>>() != v.fields.keys().collect() {
        return Err(format!("constructor fields {type_id}.{variant}"));
    }
    for (name, ty) in &v.fields {
        validate_value(s, ty, &obj[name])?;
    }
    validate_cross_fields(type_id, variant, value)?;
    Ok(())
}
fn validate_value(s: &Spec, ty: &str, v: &Value) -> Result<()> {
    let good = if let Some(t) = ty.strip_prefix("List<").and_then(|x| x.strip_suffix('>')) {
        let a = v.as_array().ok_or("expected list")?;
        for x in a {
            validate_value(s, t, x)?;
        }
        true
    } else if let Some(t) = ty.strip_prefix("Option<").and_then(|x| x.strip_suffix('>')) {
        if !v.is_null() {
            validate_value(s, t, v)?;
        }
        true
    } else {
        match ty {
            "String" => v.is_string(),
            "Bool" => v.is_boolean(),
            "Json" => true,
            "U64" => v.as_u64().is_some(),
            "Nano" => v.as_str().is_some_and(|x| {
                !x.is_empty()
                    && (x == "0" || !x.starts_with('0'))
                    && x.bytes().all(|b| b.is_ascii_digit())
                    && x.parse::<i128>().is_ok()
            }),
            named => {
                let o = v.as_object().ok_or("expected tagged type")?;
                if o.len() != 2 {
                    return Err("tagged value needs variant and fields".into());
                }
                let variant = o
                    .get("variant")
                    .and_then(Value::as_str)
                    .ok_or("missing variant")?;
                validate_constructor(s, named, variant, o.get("fields").ok_or("missing fields")?)?;
                true
            }
        }
    };
    if good {
        Ok(())
    } else {
        Err(format!("value does not conform to {ty}"))
    }
}
pub fn generated(s: &Spec, interface: &Value) -> Result<BTreeMap<String, Vec<u8>>> {
    validate(s)?;
    let owned = s
        .invariants
        .iter()
        .find(|x| x.id == "SERIALIZATION")
        .ok_or("missing serialization owner")?;
    let canonical: Value =
        serde_json::from_str(&owned.text).map_err(|e| format!("serialization owner: {e}"))?;
    if &canonical != interface {
        return Err("stale serialization interface projection".into());
    }
    validate_wire_interface(s, interface)?;
    let digest = sha256(&serde_json::to_vec(&(s, interface)).map_err(|e| e.to_string())?);
    let mut rendered = BTreeMap::new();
    for view in &s.views {
        let mut body = format!(
            "# Generated C–K normative view\n\nSource: spec/ck.json; contract {}; input SHA256 {}.\n\nThis rendering checks declared consistency only. It is not a reducer, financial proof, live trust qualification, freeze or S2a entry.\n\n## Stage interfaces\n\n| Stage | Reads | Outputs |\n|---|---|---|\n",
            s.contract_version, digest
        );
        for st in &s.stages {
            body.push_str(&format!(
                "| {} | {} | {} |\n",
                st.id,
                st.reads.join(", "),
                st.outputs.join(", ")
            ));
        }
        body.push_str("\n## Fixed tagged types\n\nTag/field encoding below describes typed specification constructors. The corpus projection is declared in the serialization interface.\n\n");
        for t in s.types.iter().filter(|_| view.nodes != "transitions") {
            body.push_str(&format!(
                "### {} r{}\n\n```json\n{}\n```\n\n",
                t.id,
                t.revision,
                serde_json::to_string_pretty(t).unwrap()
            ));
        }
        for f in s.formulas.iter().filter(|_| view.nodes != "schema") {
            body.push_str(&format!(
                "### {} r{}\n\n{}\n\nInputs: {}.\n\n",
                f.id,
                f.revision,
                f.text,
                f.inputs.join(", ")
            ));
        }
        for n in s.invariants.iter().filter(|n| {
            view.nodes == "contract"
                || view.nodes == "schema"
                    && ["CORPUS-ENVELOPE", "ENCODING", "S2A-SCOPE"].contains(&n.id.as_str())
        }) {
            body.push_str(&format!("### {} r{}\n\n{}\n\n", n.id, n.revision, n.text));
        }
        body.push_str("## Finite declared rule partitions\n\nGuards partition symbolic facts supplied by the declared stage interface. Coverage does not prove these predicates faithfully classify all raw evidence.\n\n");
        for r in s.rules.iter().filter(|_| view.nodes != "schema") {
            body.push_str(&format!(
                "### {} r{} ({})\n\n{}\n\nGuard: `{}`. Constructor: `{}`. Formulas: {}.\n\n",
                r.id,
                r.revision,
                r.stage,
                r.text,
                serde_json::to_string(&r.guard).unwrap(),
                serde_json::to_string(&r.outputs).unwrap(),
                r.formulas.join(", ")
            ));
        }
        for d in &s.decisions {
            body.push_str(&format!(
                "## Result-bearing decision {}\n\n```json\n{}\n```\n\n",
                d.id,
                serde_json::to_string_pretty(d).unwrap()
            ));
        }
        body.push_str(&format!(
            "## Finalization bindings\n\n```json\n{}\n```\n\n",
            serde_json::to_string_pretty(&s.finalization).unwrap()
        ));
        for p in &s.projections {
            body.push_str(&format!(
                "Projection: `{}`\n\n",
                serde_json::to_string(p).unwrap()
            ));
        }
        body.push_str("## Serialization interface\n\n```json\n");
        body.push_str(&serde_json::to_string_pretty(interface).map_err(|e| e.to_string())?);
        body.push_str("\n```\n\n");
        for n in s.owner_policies.iter().filter(|_| view.nodes == "contract") {
            body.push_str(&format!(
                "## {} owner-policy reference\n\n{}\n",
                n.id, n.text
            ));
        }
        rendered.insert(view.path.clone(), body.into_bytes());
    }
    for (id, path) in [
        ("SCHEMA-ACCOUNTING", "spec/accounting.schema.json"),
        ("SCHEMA-WIRE", "spec/wire.schema.json"),
    ] {
        let owner = s
            .invariants
            .iter()
            .find(|n| n.id == id)
            .ok_or("missing schema owner")?;
        let schema: Value = serde_json::from_str(&owner.text).map_err(|e| e.to_string())?;
        let mut bytes = serde_json::to_vec_pretty(&schema).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        rendered.insert(path.into(), bytes);
    }
    Ok(rendered)
}
pub fn compare_generated(root: &Path, s: &Spec, interface: &Value) -> Result<()> {
    for (p, bytes) in generated(s, interface)? {
        let actual = fs::read(root.join(&p)).map_err(|e| format!("view {p}: {e}"))?;
        if actual != bytes {
            return Err(format!("stale generated bytes: {p}"));
        }
    }
    Ok(())
}
pub fn consumer_digests(s: &Spec, deps: &Dependencies) -> Result<BTreeMap<String, String>> {
    if deps.schema != "pi-accounting-dependencies/1" {
        return Err("dependency schema".into());
    }
    let mut bodies = body_nodes(s);
    let mut edges = node_edges(s);
    for c in &deps.consumers {
        if bodies.contains_key(&c.id) {
            return Err(format!("duplicate dependency consumer {}", c.id));
        }
        bodies.insert(c.id.clone(), json(&(c.id.clone(), c.owner.clone())));
        edges.insert(c.id.clone(), c.dependencies.clone());
    }
    let mut cache = BTreeMap::new();
    for c in &deps.consumers {
        digest_node(&c.id, &bodies, &edges, &mut BTreeSet::new(), &mut cache)?;
    }
    Ok(cache)
}
pub fn check_acknowledgements(s: &Spec, deps: &Dependencies) -> Result<Vec<String>> {
    let digests = consumer_digests(s, deps)?;
    let mut pending = vec![];
    for c in &deps.consumers {
        match &c.acknowledgement {
            Some(a) => {
                if a.digest != digests[&c.id] {
                    return Err(format!("stale acknowledgement {}", c.id));
                }
                if a.disposition.trim().is_empty() || a.authority.trim().is_empty() {
                    return Err(format!("missing substantive disposition {}", c.id));
                }
            }
            None => pending.push(c.id.clone()),
        }
    }
    Ok(pending)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Spec {
        serde_json::from_str(include_str!("../../../spec/ck.json")).unwrap()
    }
    #[test]
    fn fixed_schema_and_partitions() {
        validate(&fixture()).unwrap();
        let mut raw = json(&fixture());
        raw["unknown"] = Value::Bool(true);
        assert!(serde_json::from_value::<Spec>(raw).is_err());
    }
    #[test]
    fn guard_gap_and_overlap() {
        let mut s = fixture();
        s.rules.remove(0);
        assert!(validate(&s).unwrap_err().contains("gap/overlap"));
        let mut s = fixture();
        let mut r = s.rules[0].clone();
        r.id = "duplicate-guard".into();
        s.rules.push(r);
        assert!(validate(&s).unwrap_err().contains("gap/overlap"));
    }
    #[test]
    fn missing_constructor_field() {
        let mut s = fixture();
        let r = s
            .rules
            .iter_mut()
            .find(|r| r.outputs.iter().any(|o| !o.fields.is_empty()))
            .unwrap();
        r.outputs[0].fields.clear();
        assert!(
            validate(&s)
                .unwrap_err()
                .contains("constructor field mismatch")
        );
    }
    #[test]
    fn illegal_s_reads_v_and_classification_money() {
        let mut s = fixture();
        s.stages
            .iter_mut()
            .find(|x| x.id == "S")
            .unwrap()
            .reads
            .push("V".into());
        assert!(validate(&s).is_err());
        let mut s = fixture();
        s.formulas[0].inputs.push("K.blocking".into());
        assert!(
            validate(&s)
                .unwrap_err()
                .contains("classification feeding money")
        );
    }
    #[test]
    fn stale_formula_and_idempotence() {
        let s = fixture();
        let interface = serde_json::from_str(
            &s.invariants
                .iter()
                .find(|x| x.id == "SERIALIZATION")
                .unwrap()
                .text,
        )
        .unwrap();
        let first = generated(&s, &interface).unwrap();
        assert_eq!(first, generated(&s, &interface).unwrap());
        let mut changed = s.clone();
        changed
            .formulas
            .iter_mut()
            .find(|x| x.id == "ERROR-EXPOSURE")
            .unwrap()
            .text = "stale low=0; high=AtLeast(R)".into();
        assert_ne!(first, generated(&changed, &interface).unwrap());
    }
    #[test]
    fn stale_dependency_ack_without_revision_bump() {
        let s = fixture();
        let mut deps = Dependencies {
            schema: "pi-accounting-dependencies/1".into(),
            consumers: vec![
                Consumer {
                    id: "oracle".into(),
                    owner: "independent".into(),
                    dependencies: vec!["ERROR-EXPOSURE".into()],
                    acknowledgement: None,
                },
                Consumer {
                    id: "review".into(),
                    owner: "reviewer".into(),
                    dependencies: vec!["oracle".into()],
                    acknowledgement: None,
                },
            ],
        };
        let ds = consumer_digests(&s, &deps).unwrap();
        deps.consumers[1].acknowledgement = Some(Acknowledgement {
            digest: ds["review"].clone(),
            authority: "test".into(),
            disposition: "reviewed assertion under test".into(),
        });
        check_acknowledgements(&s, &deps).unwrap();
        let mut changed = s;
        changed
            .formulas
            .iter_mut()
            .find(|x| x.id == "ERROR-EXPOSURE")
            .unwrap()
            .text
            .push_str(" changed");
        assert!(
            check_acknowledgements(&changed, &deps)
                .unwrap_err()
                .contains("stale acknowledgement")
        );
    }
    #[test]
    fn all_declared_projection_cases_validate() {
        let s = fixture();
        validate_decisions(&s).unwrap();
        for p in &s.projections {
            assert!(!p.external_state.is_empty());
        }
        let none = serde_json::json!({"amount_source":{"variant":"none","fields":{}},"cause":{"variant":"subscription","fields":{}}});
        validate_constructor(&s, "Obligation", "none", &none).unwrap();
        for source in [
            "settlement_amount",
            "reservation_confirmed",
            "reservation_bound",
        ] {
            let mut bad = none.clone();
            bad["amount_source"]["variant"] = source.into();
            assert!(validate_constructor(&s, "Obligation", "none", &bad).is_err());
            let closed = serde_json::json!({"amount_source":{"variant":source,"fields":{}},"by":"settlement:1"});
            validate_constructor(&s, "Obligation", "closed", &closed).unwrap();
        }
    }
}

#[cfg(test)]
mod generation_controls {
    use super::*;
    fn fixture() -> Spec {
        serde_json::from_str(include_str!("../../../spec/ck.json")).unwrap()
    }
    fn interface(s: &Spec) -> Value {
        serde_json::from_str(
            &s.invariants
                .iter()
                .find(|n| n.id == "SERIALIZATION")
                .unwrap()
                .text,
        )
        .unwrap()
    }
    #[test]
    fn hand_edited_generated_error_formula_is_detected_without_rewriting() {
        let s = fixture();
        let i = interface(&s);
        let dir = std::env::temp_dir().join(format!("s1-stale-formula-{}", std::process::id()));
        for (path, bytes) in generated(&s, &i).unwrap() {
            let target = dir.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, bytes).unwrap();
        }
        compare_generated(&dir, &s, &i).unwrap();
        let target = dir.join("contract/CK-GENERATED.md");
        let clean = fs::read_to_string(&target).unwrap();
        let bad = clean.replace(
            "AtLeast(max(R, valued.high or priced_high or 0))",
            "AtLeast(R)",
        );
        assert_ne!(bad, clean);
        fs::write(&target, &bad).unwrap();
        assert!(
            compare_generated(&dir, &s, &i)
                .unwrap_err()
                .contains("stale generated bytes")
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), bad);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn each_canonical_node_revision_invalidates_generated_identity() {
        let s = fixture();
        let i = interface(&s);
        let original = generated(&s, &i).unwrap();
        let mut count = 0;
        for field in [
            "types",
            "stages",
            "rules",
            "formulas",
            "invariants",
            "owner_policies",
            "decisions",
            "projections",
            "boundaries",
        ] {
            let raw = json(&s);
            for index in 0..raw[field].as_array().unwrap().len() {
                let mut changed = raw.clone();
                changed[field][index]["revision"] =
                    Value::from(raw[field][index]["revision"].as_u64().unwrap() + 1);
                let changed: Spec = serde_json::from_value(changed).unwrap();
                assert!(
                    original != generated(&changed, &i).unwrap(),
                    "revision identity did not change: {field}/{index}"
                );
                count += 1;
            }
        }
        assert!(count > 100);
    }
    #[test]
    fn unknown_symbol_reference_and_stale_interface_fail() {
        let mut s = fixture();
        s.formulas[0].inputs.push("unknown_symbol".into());
        assert!(
            validate(&s)
                .unwrap_err()
                .contains("unknown primitive formula symbol")
        );
        let s = fixture();
        assert!(
            generated(&s, &serde_json::json!({"forged":true}))
                .unwrap_err()
                .contains("stale serialization interface")
        );
    }
}

/// Registered reverse closure only; omitted conceptual dependencies remain a review concern.
pub fn affected_consumers(
    s: &Spec,
    deps: &Dependencies,
    changed: &[String],
) -> Result<Vec<String>> {
    let bodies = body_nodes(s);
    let mut edges = node_edges(s);
    for c in &deps.consumers {
        edges.insert(c.id.clone(), c.dependencies.clone());
    }
    for id in changed {
        if !bodies.contains_key(id) && !edges.contains_key(id) {
            return Err(format!("unknown changed node {id}"));
        }
    }
    let mut affected = changed.iter().cloned().collect::<BTreeSet<_>>();
    loop {
        let previous = affected.len();
        for (id, inputs) in &edges {
            if inputs.iter().any(|x| affected.contains(x)) {
                affected.insert(id.clone());
            }
        }
        if previous == affected.len() {
            break;
        }
    }
    Ok(deps
        .consumers
        .iter()
        .filter(|c| affected.contains(&c.id))
        .map(|c| c.id.clone())
        .collect())
}
