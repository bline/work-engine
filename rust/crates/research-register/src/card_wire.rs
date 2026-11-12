//! Deterministic current-use wire format. This codec does not establish authority.
use crate::{
    Json, parse,
    use_bundle::{BlobPin, revision},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub const SCHEMA: &str = "research_a4_current_v1";
pub const CODEC: &str = "research_a4_json_v1";
macro_rules! wire { ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    pub struct $name { $(pub $field: $ty),* }
}; }
wire!(CurrentCardRequest {
    decision_id: String,
    decision_revision: String,
    request_sha256: String
});
wire!(Envelope { schema: String, codec: String, outcome: String, request: Option<CurrentCardRequest>, card: Option<Card>, detail: Option<String> });
wire!(Card { source: Source, bundle: BlobPin, admission: Admission, producer_sha256: String, records: Vec<RecordCopy>, assessments: Vec<String>, qualifications: Vec<String>, applicability: Vec<String>, anchor: Anchor, delivered: Vec<Delivered>, not_delivered: NotDelivered, cross_statement: String, decision_finding_statement: String, historical_utf8: Option<String>, claims: Claims, coverage: Coverage });
wire!(Source { commit: String, register_path: String, register_tree: String, inputs: Vec<BlobPin> });
wire!(Admission { profile_sha256: String, receipt_pins: Vec<BlobPin> });
wire!(RecordCopy {
    revision: String,
    kind: String,
    utf8: String
});
wire!(Object {
    oid: String,
    kind: String,
    sha256: String,
    size: u64
});
wire!(Anchor { report_sha256: String, row_utf8: String, objects: Vec<Object> });
wire!(Delivered { finding: String, study: BlobPin, finding_utf8: String, legacy_applicability: Vec<String>, membership: Vec<Membership>, events: Vec<Event>, judgments: Vec<String> });
wire!(Membership {
    kind: String,
    via: String
});
wire!(Event {
    id: String,
    utf8: String
});
wire!(NotDelivered { pending: Vec<Disclosure>, declined: Vec<Disclosure>, other_revision: Vec<Disclosure>, cross_decision: Vec<Disclosure> });
wire!(Disclosure { id: String, r#type: String, decision_id: String, decision_revision: String, finding: String, relation: String, authorization_status: String, owner_state: OwnerState, notice: Option<String> });
wire!(OwnerState {
    owner: String,
    status: String
});
wire!(Claims { scope: String, row_count: u64, status: String, materialized_rows: Vec<String>, exact_copies: Option<Vec<String>> });
wire!(Coverage { selection: String, truncated: bool, read_scope_sha256: String, disclosure_scope_sha256: String, limitations: Vec<String> });
pub const OUTCOMES: &[&str] = &[
    "card",
    "not_registered",
    "revision_unknown",
    "historical_unsupported",
    "invalid_register",
    "invalid_bundle",
    "unreadable",
    "authority_unavailable",
    "authority_mismatch",
    "anchor_gap",
    "scope_unavailable",
    "coverage_unavailable",
    "resource_exceeded",
    "controls_unavailable",
    "unsupported_version",
    "bad_request",
    "cancelled",
    "delivery_unknown",
];
impl Envelope {
    pub fn refusal(request: Option<CurrentCardRequest>, code: &str) -> Self {
        assert!(OUTCOMES.contains(&code) && code != "card");
        Self {
            schema: SCHEMA.into(),
            codec: CODEC.into(),
            outcome: code.into(),
            request,
            card: None,
            detail: Some(code.into()),
        }
    }
    pub fn success(request: CurrentCardRequest, card: Card) -> Self {
        Self {
            schema: SCHEMA.into(),
            codec: CODEC.into(),
            outcome: "card".into(),
            request: Some(request),
            card: Some(card),
            detail: None,
        }
    }
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let mut copy = self.clone();
        if let Some(c) = &mut copy.card {
            sort_pins(&mut c.source.inputs);
            sort_pins(&mut c.admission.receipt_pins);
            c.records.sort_by(|a, b| a.revision.cmp(&b.revision));
            c.assessments.sort();
            c.qualifications.sort();
            c.applicability.sort();
            c.anchor
                .objects
                .sort_by(|a, b| (&a.kind, &a.oid).cmp(&(&b.kind, &b.oid)));
            c.delivered.sort_by(|a, b| a.finding.cmp(&b.finding));
            for e in &mut c.delivered {
                e.legacy_applicability.sort_by_cached_key(|row| {
                    crate::parse::document("legacy applicability", row.as_bytes(), &mut vec![])
                        .and_then(|j| {
                            j.get("applicability_id")
                                .and_then(crate::Json::as_str)
                                .map(str::to_owned)
                        })
                        .unwrap_or_else(|| row.clone())
                });
                e.membership
                    .sort_by(|a, b| (&a.kind, &a.via).cmp(&(&b.kind, &b.via)));
                e.events.sort_by(|a, b| a.id.cmp(&b.id));
            }
            for group in [
                &mut c.not_delivered.pending,
                &mut c.not_delivered.declined,
                &mut c.not_delivered.other_revision,
                &mut c.not_delivered.cross_decision,
            ] {
                group.sort_by(|a, b| a.id.cmp(&b.id));
            }
            c.coverage.limitations.sort();
        }
        let mut out =
            canonical_value(&serde_json::to_value(copy).map_err(|e| e.to_string())?).into_bytes();
        out.push(b'\n');
        Ok(out)
    }
    fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA || self.codec != CODEC {
            return Err("unsupported_version".into());
        }
        if !OUTCOMES.contains(&self.outcome.as_str())
            || (self.outcome == "card") != self.card.is_some()
            || (self.outcome == "card" && (self.detail.is_some() || self.request.is_none()))
            || (self.outcome != "card" && self.detail.as_deref() != Some(&self.outcome))
        {
            return Err("bad_request".into());
        }
        if let Some(q) = &self.request {
            hex(&q.request_sha256, 64)?;
        }
        if let Some(c) = &self.card {
            hex(&c.source.commit, 40)?;
            hex(&c.source.register_tree, 40)?;
            for p in c
                .source
                .inputs
                .iter()
                .chain(&c.admission.receipt_pins)
                .chain(std::iter::once(&c.bundle))
                .chain(c.delivered.iter().map(|e| &e.study))
            {
                pin(p)?;
            }
            for h in [
                &c.producer_sha256,
                &c.admission.profile_sha256,
                &c.anchor.report_sha256,
                &c.coverage.read_scope_sha256,
                &c.coverage.disclosure_scope_sha256,
            ]
            .into_iter()
            .chain(c.assessments.iter())
            .chain(c.qualifications.iter())
            .chain(c.applicability.iter())
            {
                hex(h, 64)?;
            }
            for r in &c.records {
                hex(&r.revision, 64)?;
                if ![
                    "subject",
                    "evidence",
                    "assessment",
                    "qualification",
                    "request",
                    "applicability",
                ]
                .contains(&r.kind.as_str())
                    || revision(r.utf8.as_bytes()) != r.revision
                {
                    return Err("bad_request".into());
                }
            }
            for o in &c.anchor.objects {
                hex(&o.oid, 40)?;
                hex(&o.sha256, 64)?;
                if !["commit", "tree", "blob", "tag"].contains(&o.kind.as_str()) {
                    return Err("bad_request".into());
                }
            }
            if c.coverage.selection != "current_links_and_corrections"
                || c.coverage.truncated
                || c.claims.scope != "delivered_findings"
                || c.claims.row_count != c.claims.materialized_rows.len() as u64
                || (c.claims.row_count == 0
                    && (c.claims.status != "empty" || c.claims.exact_copies != Some(vec![])))
                || (c.claims.row_count > 0
                    && (c.claims.status != "unavailable" || c.claims.exact_copies.is_some()))
            {
                return Err("bad_request".into());
            }
            for e in &c.delivered {
                for m in &e.membership {
                    if ![
                        "decision_finding",
                        "recorded_authorized_applicability",
                        "admitted_companion",
                        "correction",
                    ]
                    .contains(&m.kind.as_str())
                    {
                        return Err("bad_request".into());
                    }
                }
            }
            for d in c
                .not_delivered
                .pending
                .iter()
                .chain(&c.not_delivered.declined)
                .chain(&c.not_delivered.other_revision)
                .chain(&c.not_delivered.cross_decision)
            {
                if d.r#type != "applicability"
                    || !["pending", "authorized", "declined"]
                        .contains(&d.authorization_status.as_str())
                {
                    return Err("bad_request".into());
                }
            }
        }
        Ok(())
    }
}
pub fn hex(s: &str, n: usize) -> Result<(), String> {
    if s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err("bad_request".into())
    }
}
fn pin(p: &BlobPin) -> Result<(), String> {
    hex(&p.commit, 40)?;
    hex(&p.blob, 40)?;
    hex(&p.sha256, 64)
}
fn sort_pins(v: &mut [BlobPin]) {
    v.sort_by(|a, b| {
        (&a.commit, &a.path, &a.blob, &a.sha256).cmp(&(&b.commit, &b.path, &b.blob, &b.sha256))
    });
}
pub fn decode(bytes: &[u8]) -> Result<Envelope, String> {
    let raw = parse::document("card", bytes, &mut Vec::new()).ok_or("bad_request")?;
    let e: Envelope = serde_json::from_slice(bytes).map_err(|_| "bad_request")?;
    // Option fields are mandatory too. Compare all shapes to the typed serialization.
    fn shape(a: &Json, b: &Value) -> bool {
        match (a, b) {
            (Json::Object(a), Value::Object(b)) => {
                a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|w| shape(v, w)))
            }
            (Json::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(v, w)| shape(v, w))
            }
            _ => true,
        }
    }
    if !shape(&raw, &serde_json::to_value(&e).map_err(|_| "bad_request")?) {
        return Err("bad_request".into());
    }
    e.validate()?;
    Ok(e)
}
pub fn canonical_value(value: &Value) -> String {
    canonical(
        &parse::document(
            "internal",
            &serde_json::to_vec(value).expect("JSON value"),
            &mut vec![],
        )
        .expect("valid JSON value"),
    )
}
pub fn canonical(j: &Json) -> String {
    match j {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => n.to_string(),
        Json::String(s) => quote(s),
        Json::Array(a) => format!(
            "[{}]",
            a.iter().map(canonical).collect::<Vec<_>>().join(",")
        ),
        Json::Object(m) => {
            let mut m = m.iter().collect::<Vec<_>>();
            m.sort_by(|a, b| a.0.cmp(&b.0));
            format!(
                "{{{}}}",
                m.iter()
                    .map(|(k, v)| format!("{}:{}", quote(k), canonical(v)))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
