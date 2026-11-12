//! Immutable research-use companions. Structural validity is neither scientific qualification
//! nor authenticated authorization. Publisher-supplied actor/owner strings never create permits.

use crate::{BlobAt, GitObjects, PinnedSnapshot, ValidRegister};
use gix::hash::ObjectId;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: &str = "research_use_v1";

#[derive(Debug, thiserror::Error)]
pub enum UseError {
    #[error("missing immutable input: {0}")]
    Missing(String),
    #[error("cannot read immutable input: {0}")]
    Unreadable(#[from] crate::Error),
    #[error("unsupported schema: {0}")]
    Unsupported(String),
    #[error("invalid research-use input: {0}")]
    Invalid(String),
}

type Result<T> = std::result::Result<T, UseError>;
fn invalid(message: impl Into<String>) -> UseError {
    UseError::Invalid(message.into())
}

/// SHA-256 of the exact stored bytes, including any trailing newline. No canonicalization.
pub fn revision(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlobPin {
    pub commit: String,
    pub path: String,
    pub blob: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshot {
    pub commit: String,
    pub register_path: String,
    pub register_tree: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// Path relative to the manifest's containing directory, at the manifest's commit.
    pub path: String,
    pub blob: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub source: SourceSnapshot,
    pub inputs: Vec<Input>,
    pub request: String,
    pub applicability: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacetState {
    NotAssessed,
    Unknown,
    Supported,
    Limited,
    Contested,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facet {
    pub state: FacetState,
    pub reason: String,
    pub evidence: Vec<String>,
}

/// Named facets preserve independent questions without combining them into a score.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facets {
    pub provenance: Facet,
    pub method_reliability: Facet,
    pub consistency: Facet,
    pub reproducibility: Facet,
    pub context_transfer: Facet,
    pub inferential_strength: Facet,
    pub completeness: Facet,
    pub temporal_relevance: Facet,
}
impl Facets {
    fn all(&self) -> [&Facet; 8] {
        [
            &self.provenance,
            &self.method_reliability,
            &self.consistency,
            &self.reproducibility,
            &self.context_transfer,
            &self.inferential_strength,
            &self.completeness,
            &self.temporal_relevance,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Supports,
    LimitedSupport,
    Contradicts,
    NotEstablished,
}

/// All variants are assertions. Even verified Git evidence means only exact byte availability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Record {
    Subject {
        finding: String,
        study: BlobPin,
        evidence: Vec<String>,
    },
    Evidence {
        source: EvidenceSource,
    },
    Assessment {
        subject: String,
        assessor: String,
        as_of_context: String,
        scope: String,
        limitations: Vec<String>,
        evidence: Vec<String>,
        facets: Box<Facets>,
        predecessors: Vec<String>,
        corrections: Vec<String>,
        competes_with: Vec<String>,
        qualification: Vec<String>,
    },
    Qualification {
        assessor: String,
        purpose: String,
        disposition: String,
        reviewer: String,
        exposure_boundary: String,
        capability_evidence: Vec<String>,
        authorization_evidence: Vec<String>,
        evidence: Vec<String>,
    },
    Request {
        request_id: String,
        decision: String,
        decision_revision: String,
        target: BlobPin,
        selector: String,
        purpose: String,
        alternatives: Vec<String>,
        conditions: Vec<String>,
        operating_context: String,
        owner: String,
        acceptance: Vec<String>,
    },
    Applicability {
        subject: String,
        request: String,
        decision: String,
        decision_revision: String,
        relation: Relation,
        bounds: String,
        owner: String,
        authorization_evidence: Vec<String>,
    },
}
impl Record {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Subject { .. } => "subject",
            Self::Evidence { .. } => "evidence",
            Self::Assessment { .. } => "assessment",
            Self::Qualification { .. } => "qualification",
            Self::Request { .. } => "request",
            Self::Applicability { .. } => "applicability",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidenceSource {
    Git {
        pin: BlobPin,
    },
    /// A locator and publisher claim only; no retrieval or verification is implied.
    Assertion {
        locator: String,
        asserted_sha256: Option<String>,
        limitation: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordDocument {
    pub schema: String,
    pub record: Record,
}

#[derive(Debug, Clone)]
pub struct StoredRecord {
    pin: BlobPin,
    bytes: Vec<u8>,
    record: Record,
}
impl StoredRecord {
    pub fn pin(&self) -> &BlobPin {
        &self.pin
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn record(&self) -> &Record {
        &self.record
    }
}

/// Private join proof. Not deserializable or constructible by callers, and not an execution
/// permit. Retains every input and checked Git evidence byte for exact reconstruction.
///
/// ```compile_fail
/// use research_register::use_bundle::ValidUseBundle;
/// let forged: ValidUseBundle = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug)]
pub struct ValidUseBundle {
    manifest_pin: BlobPin,
    manifest_bytes: Vec<u8>,
    source_files: BTreeMap<String, BlobPin>,
    manifest: Manifest,
    records: BTreeMap<String, StoredRecord>,
    retained_git_bytes: BTreeMap<String, Vec<u8>>,
}
impl ValidUseBundle {
    pub fn manifest_pin(&self) -> &BlobPin {
        &self.manifest_pin
    }
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }
    pub fn source_files(&self) -> &BTreeMap<String, BlobPin> {
        &self.source_files
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn records(&self) -> &BTreeMap<String, StoredRecord> {
        &self.records
    }
    /// Keys are exact byte digests. Includes source register files, subject studies, targets
    /// and verified Git evidence. Original locators remain in the manifest and records.
    pub fn retained_git_bytes(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.retained_git_bytes
    }
}

fn hex(value: &str, len: usize) -> Result<()> {
    if value.len() != len
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(format!(
            "expected {len} lowercase hex digits: {value}"
        )));
    }
    Ok(())
}
fn oid(value: &str) -> Result<ObjectId> {
    hex(value, 40)?;
    ObjectId::from_hex(value.as_bytes()).map_err(|e| invalid(e.to_string()))
}
fn path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains(['\\', ':'])
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|c| c.is_empty() || c == "." || c == ".." || c.eq_ignore_ascii_case(".git"))
    {
        return Err(invalid(format!("unsafe relative path: {value}")));
    }
    Ok(())
}
fn text(value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(invalid("empty required text"))
    } else {
        Ok(())
    }
}
fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(invalid("malformed identifier"));
    }
    Ok(())
}
fn texts(values: &[String]) -> Result<()> {
    for v in values {
        text(v)?;
    }
    Ok(())
}
fn nonempty(values: &[String]) -> Result<()> {
    if values.is_empty() {
        return Err(invalid("required list is empty"));
    }
    texts(values)
}

fn read(objects: &GitObjects, pin: &BlobPin) -> Result<Vec<u8>> {
    path(&pin.path)?;
    hex(&pin.blob, 40)?;
    hex(&pin.sha256, 64)?;
    let commit = oid(&pin.commit)?;
    match objects.blob_at(commit, &pin.path)? {
        BlobAt::Absent => Err(UseError::Missing(pin.path.clone())),
        BlobAt::NotABlob { .. } => Err(invalid(format!("not a regular file: {}", pin.path))),
        BlobAt::Blob { id, bytes } => {
            if id != oid(&pin.blob)? || revision(&bytes) != pin.sha256 {
                return Err(invalid(format!("pin mismatch: {}", pin.path)));
            }
            Ok(bytes)
        }
    }
}
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut issues = Vec::new();
    let parsed = crate::parse::document("companion", bytes, &mut issues);
    if parsed.is_none() || !issues.is_empty() {
        return Err(invalid(format!("strict JSON: {issues:?}")));
    }
    let schema = parsed
        .as_ref()
        .and_then(|j| j.get("schema"))
        .and_then(crate::Json::as_str)
        .ok_or_else(|| invalid("missing schema"))?;
    if schema != VERSION {
        return Err(UseError::Unsupported(schema.into()));
    }
    serde_json::from_slice(bytes).map_err(|e| invalid(e.to_string()))
}
fn link<'a>(
    records: &'a BTreeMap<String, StoredRecord>,
    id: &str,
    kind: &str,
) -> Result<&'a Record> {
    hex(id, 64)?;
    let r = records
        .get(id)
        .ok_or_else(|| UseError::Missing(format!("{kind} {id}")))?;
    if r.record.kind() != kind {
        return Err(invalid(format!(
            "expected {kind}, found {}",
            r.record.kind()
        )));
    }
    Ok(&r.record)
}
fn links(records: &BTreeMap<String, StoredRecord>, ids: &[String], kind: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(invalid("duplicate link"));
        }
        link(records, id, kind)?;
    }
    Ok(())
}

/// Validate an upstream-only manifest at a separately pinned companion commit. Both existing
/// register proof and source snapshot are mandatory, and must agree exactly.
pub fn validate_use_bundle(
    source: &PinnedSnapshot,
    register: &ValidRegister,
    manifest_pin: BlobPin,
) -> Result<ValidUseBundle> {
    let objects = source.objects();
    let manifest_bytes = read(objects, &manifest_pin)?;
    let manifest: Manifest = decode(&manifest_bytes)?;
    if register.commit() != source.commit()
        || register.register_tree() != source.register_tree()
        || register.register_path() != source.register_path()
        || oid(&manifest.source.commit)? != source.commit()
        || oid(&manifest.source.register_tree)? != source.register_tree()
        || manifest.source.register_path != source.register_path()
    {
        return Err(invalid("source snapshot / register proof mismatch"));
    }
    let parent = manifest_pin.path.rsplit_once('/').map(|(p, _)| p);
    let mut records = BTreeMap::new();
    let mut paths = BTreeSet::new();
    for input in &manifest.inputs {
        path(&input.path)?;
        if !paths.insert(&input.path) {
            return Err(invalid("duplicate input path"));
        }
        let pin = BlobPin {
            commit: manifest_pin.commit.clone(),
            path: parent.map_or_else(|| input.path.clone(), |p| format!("{p}/{}", input.path)),
            blob: input.blob.clone(),
            sha256: input.sha256.clone(),
        };
        if pin.path == manifest_pin.path {
            return Err(invalid("manifest cannot include itself"));
        }
        let bytes = read(objects, &pin)?;
        let doc: RecordDocument = decode(&bytes)?;
        if records
            .insert(
                input.sha256.clone(),
                StoredRecord {
                    pin,
                    bytes,
                    record: doc.record,
                },
            )
            .is_some()
        {
            return Err(invalid("duplicate record identity"));
        }
    }
    let mut retained_git_bytes: BTreeMap<String, Vec<u8>> = source
        .entries()
        .values()
        .filter_map(|e| Some((e.sha256()?.to_owned(), e.bytes()?.to_vec())))
        .collect();
    for stored in records.values() {
        validate_record(source, register, &records, &stored.record)?;
        match &stored.record {
            Record::Subject { study: pin, .. } | Record::Request { target: pin, .. } => {
                retained_git_bytes.insert(pin.sha256.clone(), read(objects, pin)?);
            }
            _ => {}
        }
        if let Record::Evidence {
            source: EvidenceSource::Git { pin },
        } = &stored.record
        {
            retained_git_bytes.insert(pin.sha256.clone(), read(objects, pin)?);
        }
    }
    predecessor_cycles(&records)?;
    link(&records, &manifest.request, "request")?;
    nonempty(&manifest.applicability)?;
    links(&records, &manifest.applicability, "applicability")?;
    for id in &manifest.applicability {
        if let Record::Applicability {
            subject, request, ..
        } = link(&records, id, "applicability")?
        {
            if request != &manifest.request {
                return Err(invalid("manifest request mismatch"));
            }
            if !records
                .values()
                .any(|r| matches!(&r.record,Record::Assessment {subject:s,..} if s==subject))
            {
                return Err(UseError::Missing(
                    "assessment for applicable subject".into(),
                ));
            }
        }
    }
    let source_files = source
        .entries()
        .iter()
        .filter_map(|(relative, e)| {
            Some((
                format!("{}/{}", source.register_path(), relative),
                BlobPin {
                    commit: source.commit().to_string(),
                    path: format!("{}/{}", source.register_path(), relative),
                    blob: e.id().to_string(),
                    sha256: e.sha256()?.to_owned(),
                },
            ))
        })
        .collect();
    Ok(ValidUseBundle {
        source_files,
        manifest_pin,
        manifest_bytes,
        manifest,
        records,
        retained_git_bytes,
    })
}

fn validate_record(
    source: &PinnedSnapshot,
    register: &ValidRegister,
    records: &BTreeMap<String, StoredRecord>,
    record: &Record,
) -> Result<()> {
    match record {
        Record::Subject {
            finding,
            study,
            evidence,
        } => {
            let (study_id, _) = finding
                .split_once('#')
                .ok_or_else(|| invalid("malformed finding ID"))?;
            let registered = register
                .records()
                .studies
                .iter()
                .find(|s| s.id == study_id && s.findings.iter().any(|f| f.id == *finding))
                .ok_or_else(|| invalid("finding is not registered"))?;
            if oid(&study.commit)? != source.commit()
                || study.path != format!("{}/{}", source.register_path(), registered.at.file)
            {
                return Err(invalid("subject study is not the registered source study"));
            }
            read(source.objects(), study)?;
            nonempty(evidence)?;
            links(records, evidence, "evidence")?;
        }
        Record::Evidence {
            source: EvidenceSource::Git { pin },
        } => {
            read(source.objects(), pin)?;
        }
        Record::Evidence {
            source:
                EvidenceSource::Assertion {
                    locator,
                    asserted_sha256,
                    limitation,
                },
        } => {
            text(locator)?;
            text(limitation)?;
            if let Some(d) = asserted_sha256 {
                hex(d, 64)?;
            }
        }
        Record::Assessment {
            subject,
            assessor,
            as_of_context,
            scope,
            limitations,
            evidence,
            facets,
            predecessors,
            corrections,
            competes_with,
            qualification,
        } => {
            link(records, subject, "subject")?;
            text(assessor)?;
            text(as_of_context)?;
            text(scope)?;
            texts(limitations)?;
            nonempty(evidence)?;
            links(records, evidence, "evidence")?;
            for facet in facets.all() {
                text(&facet.reason)?;
                links(records, &facet.evidence, "evidence")?;
                if facet.state != FacetState::NotAssessed {
                    nonempty(&facet.evidence)?;
                }
            }
            for ids in [predecessors, corrections, competes_with] {
                links(records, ids, "assessment")?;
                for id in ids {
                    if let Record::Assessment { subject: s, .. } = link(records, id, "assessment")?
                        && s != subject
                    {
                        return Err(invalid("assessment link changes subject"));
                    }
                }
            }
            nonempty(qualification)?;
            links(records, qualification, "qualification")?;
        }
        Record::Qualification {
            assessor,
            purpose,
            disposition,
            reviewer,
            exposure_boundary,
            capability_evidence,
            authorization_evidence,
            evidence,
        } => {
            for t in [assessor, purpose, disposition, reviewer, exposure_boundary] {
                text(t)?;
            }
            for ids in [capability_evidence, authorization_evidence, evidence] {
                nonempty(ids)?;
                links(records, ids, "evidence")?;
            }
        }
        Record::Request {
            request_id,
            decision,
            decision_revision,
            target,
            selector,
            purpose,
            alternatives,
            conditions,
            operating_context,
            owner,
            acceptance,
        } => {
            identifier(request_id)?;
            text(selector)?;
            text(purpose)?;
            text(operating_context)?;
            text(owner)?;
            nonempty(alternatives)?;
            nonempty(conditions)?;
            nonempty(acceptance)?;
            let d = register
                .records()
                .decisions
                .iter()
                .find(|d| &d.decision_id == decision)
                .ok_or_else(|| invalid("unregistered decision"))?;
            // M0 current requests only: exact registered revision spelling, no prefix substitution.
            if decision_revision != &d.revision.text {
                return Err(UseError::Unsupported(
                    "historical or different decision revision".into(),
                ));
            }
            if oid(&target.commit)? != source.commit() || target.path != d.document {
                return Err(invalid(
                    "target differs from registered decision at source snapshot",
                ));
            }
            read(source.objects(), target)?;
        }
        Record::Applicability {
            subject,
            request,
            decision,
            decision_revision,
            bounds,
            owner,
            authorization_evidence,
            ..
        } => {
            link(records, subject, "subject")?;
            text(bounds)?;
            text(owner)?;
            if let Record::Request {
                decision: d,
                decision_revision: r,
                ..
            } = link(records, request, "request")?
                && (d != decision || r != decision_revision)
            {
                return Err(invalid("applicability decision/request mismatch"));
            }
            nonempty(authorization_evidence)?;
            links(records, authorization_evidence, "evidence")?;
        }
    }
    Ok(())
}

fn predecessor_cycles(records: &BTreeMap<String, StoredRecord>) -> Result<()> {
    fn visit<'a>(
        id: &'a str,
        records: &'a BTreeMap<String, StoredRecord>,
        active: &mut BTreeSet<&'a str>,
        done: &mut BTreeSet<&'a str>,
    ) -> Result<()> {
        if done.contains(id) {
            return Ok(());
        }
        if !active.insert(id) {
            return Err(invalid("cyclic assessment predecessor/correction"));
        }
        if let Record::Assessment {
            predecessors,
            corrections,
            ..
        } = &records[id].record
        {
            for next in predecessors.iter().chain(corrections) {
                visit(next, records, active, done)?;
            }
        }
        active.remove(id);
        done.insert(id);
        Ok(())
    }
    let mut done = BTreeSet::new();
    let mut active = BTreeSet::new();
    for (id, r) in records {
        if matches!(r.record, Record::Assessment { .. }) {
            visit(id, records, &mut active, &mut done)?;
        }
    }
    Ok(())
}

/// Passive downstream records. These report publisher assertions about delivery/adoption;
/// reading them never admits work, checks currentness, or establishes a grant or live result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PassiveRecord {
    PreparedBasis {
        bundle: BlobPin,
        request: String,
        assessments: Vec<String>,
        applicability: Vec<String>,
        card: Box<BlobPin>,
        delivery: Box<BlobPin>,
        recommendation: Box<BlobPin>,
        context_check: BlobPin,
        policy: BlobPin,
    },
    Adoption {
        basis: BlobPin,
        preparation_revision: String,
        execution: String,
        admission: BlobPin,
        grant: BlobPin,
    },
    Feedback {
        adoption: BlobPin,
        execution: String,
        verification: BlobPin,
        costs: String,
        failures: Vec<String>,
        unknowns: Vec<String>,
        candidate_status: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveDocument {
    pub schema: String,
    pub record: PassiveRecord,
}

#[derive(Debug)]
pub struct ReadPassive {
    record: PassiveRecord,
    /// Includes root, intermediate references and exact opaque consumer artifacts.
    retained: BTreeMap<String, Vec<u8>>,
}
impl ReadPassive {
    pub fn record(&self) -> &PassiveRecord {
        &self.record
    }
    pub fn retained(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.retained
    }
}

/// Check backward references against an already validated bundle. Opaque consumer artifacts
/// are checked only for exact file/blob/digest availability, never their authority or meaning.
pub fn read_passive(
    objects: &GitObjects,
    bundle: &ValidUseBundle,
    pin: &BlobPin,
) -> Result<ReadPassive> {
    fn keep(
        objects: &GitObjects,
        pin: &BlobPin,
        out: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<Vec<u8>> {
        let bytes = read(objects, pin)?;
        out.insert(pin.sha256.clone(), bytes.clone());
        Ok(bytes)
    }
    fn basis(
        objects: &GitObjects,
        bundle: &ValidUseBundle,
        record: &PassiveRecord,
        out: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<()> {
        let PassiveRecord::PreparedBasis {
            bundle: bound,
            request,
            assessments,
            applicability,
            card,
            delivery,
            recommendation,
            context_check,
            policy,
        } = record
        else {
            return Err(invalid("expected prepared_basis"));
        };
        if bound != bundle.manifest_pin() || request != &bundle.manifest.request {
            return Err(invalid("prepared basis bundle/request mismatch"));
        }
        keep(objects, bound, out)?;
        nonempty(assessments)?;
        links(&bundle.records, assessments, "assessment")?;
        nonempty(applicability)?;
        links(&bundle.records, applicability, "applicability")?;
        for id in applicability {
            if !bundle.manifest.applicability.contains(id) {
                return Err(invalid("basis applicability not selected in manifest"));
            }
            if let Record::Applicability { subject, .. } =
                link(&bundle.records, id, "applicability")?
            {
                let covered = assessments.iter().any(|a| {
                    matches!(
                        &bundle.records[a].record,
                        Record::Assessment { subject: assessed, .. } if assessed == subject
                    )
                });
                if !covered {
                    return Err(invalid(
                        "basis assessment does not cover applicability subject",
                    ));
                }
            }
        }
        if let Record::Request { target, .. } = link(&bundle.records, request, "request")?
            && policy != target
        {
            return Err(invalid("basis policy differs from request target"));
        }
        for p in [
            card.as_ref(),
            delivery.as_ref(),
            recommendation.as_ref(),
            context_check,
            policy,
        ] {
            keep(objects, p, out)?;
        }
        Ok(())
    }
    fn adoption(
        objects: &GitObjects,
        bundle: &ValidUseBundle,
        record: &PassiveRecord,
        out: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<()> {
        let PassiveRecord::Adoption {
            basis: pin,
            preparation_revision,
            execution,
            admission,
            grant,
        } = record
        else {
            return Err(invalid("expected adoption"));
        };
        text(preparation_revision)?;
        text(execution)?;
        let doc: PassiveDocument = decode(&keep(objects, pin, out)?)?;
        basis(objects, bundle, &doc.record, out)?;
        keep(objects, admission, out)?;
        keep(objects, grant, out)?;
        Ok(())
    }
    let mut retained = BTreeMap::new();
    let doc: PassiveDocument = decode(&keep(objects, pin, &mut retained)?)?;
    match &doc.record {
        PassiveRecord::PreparedBasis { .. } => basis(objects, bundle, &doc.record, &mut retained)?,
        PassiveRecord::Adoption { .. } => adoption(objects, bundle, &doc.record, &mut retained)?,
        PassiveRecord::Feedback {
            adoption: pin,
            execution,
            verification,
            costs,
            failures,
            unknowns,
            candidate_status,
        } => {
            text(execution)?;
            text(costs)?;
            texts(failures)?;
            texts(unknowns)?;
            text(candidate_status)?;
            let prior: PassiveDocument = decode(&keep(objects, pin, &mut retained)?)?;
            adoption(objects, bundle, &prior.record, &mut retained)?;
            if !matches!(&prior.record,PassiveRecord::Adoption {execution:e,..} if e==execution) {
                return Err(invalid("feedback source execution mismatch"));
            }
            keep(objects, verification, &mut retained)?;
        }
    }
    Ok(ReadPassive {
        record: doc.record,
        retained,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // SHA identities make a real cyclic publication computationally unavailable. Exercise the
    // defensive graph check directly with synthetic identities, never a public join proof.
    #[test]
    fn required_predecessors_and_corrections_cannot_cycle_but_competition_can() {
        let facet = Facet {
            state: FacetState::NotAssessed,
            reason: "synthetic".into(),
            evidence: vec![],
        };
        let facets = Facets {
            provenance: facet.clone(),
            method_reliability: facet.clone(),
            consistency: facet.clone(),
            reproducibility: facet.clone(),
            context_transfer: facet.clone(),
            inferential_strength: facet.clone(),
            completeness: facet.clone(),
            temporal_relevance: facet,
        };
        let record = Record::Assessment {
            subject: "subject".into(),
            assessor: "actor".into(),
            as_of_context: "synthetic".into(),
            scope: "synthetic".into(),
            limitations: vec![],
            evidence: vec![],
            facets: Box::new(facets),
            predecessors: vec!["b".into()],
            corrections: vec![],
            competes_with: vec!["b".into()],
            qualification: vec![],
        };
        let stored = StoredRecord {
            pin: BlobPin {
                commit: String::new(),
                path: String::new(),
                blob: String::new(),
                sha256: String::new(),
            },
            bytes: vec![],
            record,
        };
        let mut records = BTreeMap::from([("a".into(), stored.clone()), ("b".into(), stored)]);
        if let Record::Assessment {
            predecessors,
            corrections,
            ..
        } = &mut records.get_mut("b").unwrap().record
        {
            predecessors.clear();
            corrections.push("a".into());
        }
        assert!(matches!(
            predecessor_cycles(&records),
            Err(UseError::Invalid(_))
        ));
        if let Record::Assessment {
            corrections,
            competes_with,
            ..
        } = &mut records.get_mut("b").unwrap().record
        {
            corrections.clear();
            *competes_with = vec!["a".into()];
        }
        assert!(predecessor_cycles(&records).is_ok());
    }
}
