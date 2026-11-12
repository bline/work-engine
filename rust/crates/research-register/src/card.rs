//! Current-use projection. Register validity, anchors and authority remain separate proofs.
use crate::{
    Json, PinnedSnapshot, ValidRegister,
    card_admission::AdmittedUse,
    card_wire::*,
    model::{self, AuthStatus, EventKind},
    use_bundle::{Record, ValidUseBundle, revision},
};
use std::collections::{BTreeMap, BTreeSet};
pub const PENDING: &str = "Proposed relationship only. Not owner-authorized. Excluded from delivered evidence and applicability judgments.";
pub const DECLINED: &str = "Applicability relationship declined. Not authorized for the requested decision revision. This does not determine the underlying finding's validity or applicability elsewhere.";
pub const CROSS: &str = "These records belong to other decisions. Their relationship to another decision does not establish applicability to the requested decision.";
pub const DECISION_FINDING: &str = "listed on the decision record; not an applicability judgment";
/// In-process proof owned by preparation, never reconstructed from a structural report DTO.
pub(crate) struct AdmittedReadBatch {
    pub anchors: crate::AnchorReport,
    pub objects: Vec<(gix::hash::ObjectId, gix::objs::Kind, Vec<u8>)>,
}
pub(crate) fn request_check(valid: &ValidRegister, q: &CurrentCardRequest) -> Result<(), String> {
    let d = valid
        .records()
        .decisions
        .iter()
        .find(|d| d.decision_id == q.decision_id)
        .ok_or("not_registered")?;
    if q.decision_revision == d.revision.text {
        return Ok(());
    }
    if d.revisions
        .as_ref()
        .is_some_and(|rs| rs.iter().any(|r| r.revision.text == q.decision_revision))
    {
        return Err("historical_unsupported".into());
    }
    Err("revision_unknown".into())
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn project_current(
    snapshot: &PinnedSnapshot,
    valid: &ValidRegister,
    bundle: &ValidUseBundle,
    q: &CurrentCardRequest,
    admitted: &AdmittedUse,
    batch: &AdmittedReadBatch,
    producer: &str,
    read_scope: &str,
) -> Result<Card, String> {
    request_check(valid, q)?;
    if admitted.bundle_sha256 != bundle.manifest_pin().sha256
        || admitted.request != *q
        || snapshot.commit() != valid.commit()
        || batch.anchors.pinned_commit != snapshot.commit()
        || batch.anchors.register_tree != snapshot.register_tree()
        || batch.anchors.register_path != snapshot.register_path()
    {
        return Err("authority_mismatch".into());
    }
    let records = valid.records();
    let d = records
        .decisions
        .iter()
        .find(|d| d.decision_id == q.decision_id)
        .ok_or("not_registered")?;
    let anchor = batch
        .anchors
        .entries
        .iter()
        .find(|a| a.decision_id == d.decision_id)
        .filter(|a| a.class.is_some())
        .ok_or("anchor_gap")?;
    if !admitted.scope.decision_ids.contains(&q.decision_id) {
        return Err("coverage_unavailable".into());
    }
    let mut selected: BTreeMap<String, Vec<Membership>> = BTreeMap::new();
    let mut add = |id: &str, kind: &str, via: &str| {
        selected.entry(id.into()).or_default().push(Membership {
            kind: kind.into(),
            via: via.into(),
        });
    };
    for f in &d.findings {
        add(f, "decision_finding", &d.decision_id);
    }
    for a in &records.applicability {
        if a.decision_id == d.decision_id
            && a.decision_revision.text == d.revision.text
            && a.authorization.status == AuthStatus::Authorized
        {
            add(
                &a.finding,
                "recorded_authorized_applicability",
                &a.applicability_id,
            );
        }
    }
    for id in &bundle.manifest().applicability {
        if let Record::Applicability { subject, .. } = bundle.records()[id].record()
            && let Record::Subject { finding, .. } = bundle.records()[subject].record()
        {
            add(finding, "admitted_companion", id);
        }
    }
    loop {
        let mut additions = vec![];
        for e in &records.events {
            if e.kind == EventKind::Correction
                && let Some(by) = &e.by_finding
                && (selected.contains_key(&e.subject_finding) || selected.contains_key(by))
            {
                for f in [&e.subject_finding, by] {
                    if !selected.contains_key(f) {
                        additions.push((f.clone(), e.event_id.clone()));
                    }
                }
            }
        }
        if additions.is_empty() {
            break;
        }
        for (f, via) in additions {
            selected.entry(f).or_default().push(Membership {
                kind: "correction".into(),
                via,
            });
        }
    }
    if selected
        .keys()
        .any(|id| !admitted.scope.finding_ids.contains(id))
    {
        return Err("coverage_unavailable".into());
    }
    // Material event relationships remain visible; a scope gap cannot be hidden by
    // omitting the objection or exposing its out-of-scope record instead.
    if records.events.iter().any(|e| {
        (selected.contains_key(&e.subject_finding)
            || e.by_finding
                .as_ref()
                .is_some_and(|f| selected.contains_key(f)))
            && (!admitted.scope.finding_ids.contains(&e.subject_finding)
                || e.by_finding
                    .as_ref()
                    .is_some_and(|f| !admitted.scope.finding_ids.contains(f)))
    }) {
        return Err("coverage_unavailable".into());
    }
    let mut delivered = vec![];
    for (finding, membership) in &selected {
        let study = records
            .studies
            .iter()
            .find(|s| s.findings.iter().any(|f| &f.id == finding))
            .ok_or("invalid_register")?;
        let parsed = raw_document(snapshot, &study.at.file)?;
        let raw_finding = array(parsed.get("findings").ok_or("invalid_register")?)?
            .iter()
            .find(|f| f.get("id").and_then(Json::as_str) == Some(finding))
            .ok_or("invalid_register")?;
        let study_path = format!("{}/{}", snapshot.register_path(), study.at.file);
        let pin = bundle
            .source_files()
            .get(&study_path)
            .ok_or("unreadable")?
            .clone();
        let events = records
            .events
            .iter()
            .filter(|e| &e.subject_finding == finding || e.by_finding.as_ref() == Some(finding))
            .map(|e| {
                Ok(Event {
                    id: e.event_id.clone(),
                    utf8: raw_row(snapshot, &e.at.file, e.at.line)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let judgments = records
            .judgments
            .iter()
            .filter(|j| &j.finding == finding)
            .map(|j| raw_row(snapshot, &j.at.file, j.at.line))
            .collect::<Result<Vec<_>, _>>()?;
        delivered.push(Delivered {
            finding: finding.clone(),
            study: pin,
            finding_utf8: canonical(raw_finding),
            legacy_applicability: {
                let mut rows = records
                    .applicability
                    .iter()
                    .filter(|a| {
                        a.finding == *finding
                            && a.decision_id == q.decision_id
                            && a.decision_revision.text == q.decision_revision
                            && a.authorization.status == AuthStatus::Authorized
                    })
                    .collect::<Vec<_>>();
                rows.sort_by(|a, b| a.applicability_id.cmp(&b.applicability_id));
                rows.into_iter()
                    .map(|a| raw_row(snapshot, &a.at.file, a.at.line))
                    .collect::<Result<Vec<_>, _>>()?
            },
            membership: membership.clone(),
            events,
            judgments,
        });
    }
    let mut nd = NotDelivered {
        pending: vec![],
        declined: vec![],
        other_revision: vec![],
        cross_decision: vec![],
    };
    for a in &records.applicability {
        let same = a.decision_id == q.decision_id;
        if !same && !selected.contains_key(&a.finding) {
            continue;
        }
        if !same
            && (!admitted.scope.cross_decision_metadata
                || !admitted.scope.decision_ids.contains(&a.decision_id))
        {
            return Err("coverage_unavailable".into());
        }
        let mut row = Disclosure {
            id: a.applicability_id.clone(),
            r#type: "applicability".into(),
            decision_id: a.decision_id.clone(),
            decision_revision: a.decision_revision.text.clone(),
            finding: a.finding.clone(),
            relation: a.relation.as_str().into(),
            authorization_status: a.authorization.status.as_str().into(),
            owner_state: OwnerState {
                owner: a.authorization.owner.clone(),
                status: a.authorization.status.as_str().into(),
            },
            notice: None,
        };
        if !same {
            nd.cross_decision.push(row);
        } else if a.decision_revision.text != q.decision_revision {
            row.notice = Some(format!(
                "Recorded for decision revision {} (status: {}). This relationship is not carried forward to the requested revision.",
                a.decision_revision.text,
                a.authorization.status.as_str()
            ));
            nd.other_revision.push(row);
        } else {
            match a.authorization.status {
                AuthStatus::Pending => {
                    row.notice = Some(PENDING.into());
                    nd.pending.push(row);
                }
                AuthStatus::Declined => {
                    row.notice = Some(DECLINED.into());
                    nd.declined.push(row);
                }
                AuthStatus::Authorized => {}
            }
        }
    }
    let materialized_rows = records
        .materialized
        .iter()
        .filter(|m| selected.contains_key(&m.finding))
        .map(|m| raw_row(snapshot, &m.at.file, m.at.line))
        .collect::<Result<Vec<_>, _>>()?;
    let count = materialized_rows.len() as u64;
    let historical_utf8 = {
        let dec = raw_document(snapshot, model::DECISIONS)?;
        array(&dec)?
            .iter()
            .find(|x| x.get("decision_id").and_then(Json::as_str) == Some(&q.decision_id))
            .and_then(|x| x.get("historical"))
            .map(canonical)
    };
    let mut include = BTreeSet::from([q.request_sha256.clone()]);
    for (id, s) in bundle.records() {
        if matches!(s.record(),Record::Subject{finding,..} if selected.contains_key(finding))
            || matches!(s.record(),Record::Assessment{subject,..} if matches!(bundle.records()[subject].record(),Record::Subject{finding,..} if selected.contains_key(finding)))
            || bundle.manifest().applicability.contains(id)
        {
            include.insert(id.clone());
        }
    }
    loop {
        let old = include.len();
        for id in include.clone() {
            refs(bundle.records()[&id].record(), &mut include);
        }
        if old == include.len() {
            break;
        }
    }
    let mut copies = vec![];
    let (mut assessments, mut qualifications, mut applicability) = (vec![], vec![], vec![]);
    for id in include {
        let s = &bundle.records()[&id];
        match s.record() {
            Record::Subject { finding, .. } if !admitted.scope.finding_ids.contains(finding) => {
                return Err("coverage_unavailable".into());
            }
            Record::Request { decision, .. } | Record::Applicability { decision, .. }
                if !admitted.scope.decision_ids.contains(decision)
                    || (decision != &q.decision_id && !admitted.scope.cross_decision_metadata) =>
            {
                return Err("coverage_unavailable".into());
            }
            _ => {}
        }
        match s.record() {
            Record::Assessment { .. } => assessments.push(id.clone()),
            Record::Qualification { .. } => qualifications.push(id.clone()),
            Record::Applicability { .. } => applicability.push(id.clone()),
            _ => {}
        }
        copies.push(RecordCopy {
            revision: id,
            kind: s.record().kind().into(),
            utf8: String::from_utf8(s.bytes().to_vec()).map_err(|_| "unreadable")?,
        });
    }
    let objects = batch
        .objects
        .iter()
        .map(|(id, kind, bytes)| Object {
            oid: id.to_string(),
            kind: crate::objects::kind_name(*kind).into(),
            sha256: revision(bytes),
            size: bytes.len() as u64,
        })
        .collect();
    Ok(Card { source:Source{commit:snapshot.commit().to_string(),register_path:snapshot.register_path().into(),register_tree:snapshot.register_tree().to_string(),inputs:bundle.source_files().values().cloned().collect()},bundle:bundle.manifest_pin().clone(),admission:Admission{profile_sha256:admitted.profile_sha256.clone(),receipt_pins:admitted.receipt_pins.clone()},producer_sha256:producer.into(),records:copies,assessments,qualifications,applicability,anchor:Anchor{report_sha256:revision(canonical_value(&batch.anchors.to_json()).as_bytes()),row_utf8:canonical_value(anchor.json()),objects},delivered,not_delivered:nd,cross_statement:CROSS.into(),decision_finding_statement:DECISION_FINDING.into(),historical_utf8,claims:Claims{scope:"delivered_findings".into(),row_count:count,status:if count==0{"empty"}else{"unavailable"}.into(),materialized_rows,exact_copies:if count==0{Some(vec![])}else{None}},coverage:Coverage{selection:"current_links_and_corrections".into(),truncated:false,read_scope_sha256:read_scope.into(),disclosure_scope_sha256:revision(&serde_json::to_vec(&admitted.scope).map_err(|_|"coverage_unavailable")?),limitations:vec!["Synthetic installed authority only; no production authentication or execution authority.".into(),"Native claims were not consulted; claims scope is delivered findings only.".into()]}})
}
fn refs(record: &Record, set: &mut BTreeSet<String>) {
    let mut add = |ids: &[String]| set.extend(ids.iter().cloned());
    match record {
        Record::Subject { evidence, .. } => add(evidence),
        Record::Evidence { .. } | Record::Request { .. } => {}
        Record::Qualification {
            capability_evidence,
            authorization_evidence,
            evidence,
            ..
        } => {
            add(capability_evidence);
            add(authorization_evidence);
            add(evidence);
        }
        Record::Applicability {
            subject,
            request,
            authorization_evidence,
            ..
        } => {
            add(std::slice::from_ref(subject));
            add(std::slice::from_ref(request));
            add(authorization_evidence);
        }
        Record::Assessment {
            subject,
            evidence,
            facets,
            predecessors,
            corrections,
            competes_with,
            qualification,
            ..
        } => {
            add(std::slice::from_ref(subject));
            add(evidence);
            add(predecessors);
            add(corrections);
            add(competes_with);
            add(qualification);
            for f in [
                &facets.provenance,
                &facets.method_reliability,
                &facets.consistency,
                &facets.reproducibility,
                &facets.context_transfer,
                &facets.inferential_strength,
                &facets.completeness,
                &facets.temporal_relevance,
            ] {
                add(&f.evidence);
            }
        }
    }
}
fn raw_bytes(b: &[u8]) -> Result<Json, String> {
    crate::parse::document("copy", b, &mut vec![]).ok_or("unreadable".into())
}
fn raw_document(s: &PinnedSnapshot, file: &str) -> Result<Json, String> {
    raw_bytes(s.entry(file).and_then(|e| e.bytes()).ok_or("unreadable")?)
}
fn array(j: &Json) -> Result<&[Json], String> {
    if let Json::Array(a) = j {
        Ok(a)
    } else {
        Err("invalid_register".into())
    }
}
fn raw_row(s: &PinnedSnapshot, file: &str, line: Option<u64>) -> Result<String, String> {
    let b = s.entry(file).and_then(|e| e.bytes()).ok_or("unreadable")?;
    let raw = crate::parse::lines(file, b, &mut vec![])
        .into_iter()
        .find(|(n, _)| Some(*n) == line)
        .ok_or("unreadable")?;
    Ok(canonical(&raw.1))
}
