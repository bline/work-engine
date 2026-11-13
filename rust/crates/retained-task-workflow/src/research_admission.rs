//! Task-input admission only. Production authority installation is deliberately unavailable.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_METADATA: usize = 128 * 1024;
/// Complete notice receipts leave room for the protected operator response
/// wrapper within its existing 128KiB frame; the trailing newline is framing.
pub const MAX_NOTICE_RECEIPT_BYTES: usize = 128 * 1024 - 256;
pub const MAX_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;
pub const RESERVED_PREFIX: &str = "research-";
const TABLES: &str = "
CREATE TABLE research_tasks(task_id TEXT PRIMARY KEY, subject_id TEXT NOT NULL);
CREATE TABLE research_keys(key_id TEXT PRIMARY KEY, task_id TEXT NOT NULL, key_json BLOB NOT NULL, selectors_json BLOB NOT NULL, head_json BLOB, preparation_count INTEGER NOT NULL DEFAULT 0);
CREATE TABLE research_history(key_id TEXT NOT NULL, revision INTEGER NOT NULL, head_json BLOB NOT NULL, PRIMARY KEY(key_id,revision));
CREATE TABLE research_notices(key_id TEXT NOT NULL, notice_id TEXT NOT NULL, payload_digest TEXT NOT NULL, notice_json BLOB NOT NULL, receipt_json BLOB NOT NULL, PRIMARY KEY(key_id,notice_id));
CREATE TABLE research_capacity(key_id TEXT PRIMARY KEY, notice_id TEXT NOT NULL, payload_digest TEXT NOT NULL, receipt_json BLOB NOT NULL);
CREATE TABLE research_basis(key_id TEXT NOT NULL, revision INTEGER NOT NULL, basis_digest TEXT NOT NULL, basis_json BLOB NOT NULL, PRIMARY KEY(key_id,revision));
CREATE TABLE research_adoptions(command_id TEXT PRIMARY KEY, task_id TEXT NOT NULL, key_id TEXT NOT NULL, payload_digest TEXT NOT NULL, receipt_json BLOB NOT NULL);
CREATE TABLE research_reconsideration(key_id TEXT NOT NULL, notice_id TEXT NOT NULL, command_id TEXT NOT NULL, receipt_json BLOB NOT NULL, PRIMARY KEY(key_id,notice_id,command_id));";

pub(crate) fn migrate_schema(db: &Connection, fail: bool) -> Result<(), TaskError> {
    db.execute_batch("BEGIN IMMEDIATE;")?;
    let result = (|| {
        validate_owner_schema(db, false)?;
        db.execute_batch(TABLES)?;
        db.execute_batch(
            "ALTER TABLE store_identity RENAME TO old_store_identity;
CREATE TABLE store_identity (
                 singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                 store_id TEXT NOT NULL UNIQUE,
                 schema_version INTEGER NOT NULL CHECK(schema_version = 3)
               );
INSERT INTO store_identity SELECT singleton,store_id,3 FROM old_store_identity;
DROP TABLE old_store_identity; PRAGMA user_version=3;",
        )?;
        if fail {
            return Err(TaskError::Unavailable);
        }
        validate_schema(db)
    })();
    match result {
        Ok(()) => {
            db.execute_batch("COMMIT;")?;
            Ok(())
        }
        Err(e) => {
            db.execute_batch("ROLLBACK;")?;
            Err(e)
        }
    }
}
pub(crate) fn validate_schema(db: &Connection) -> Result<(), TaskError> {
    let expected = Connection::open_in_memory()?;
    expected
        .execute_batch(&OWNER_SCHEMA_SQL.replace("schema_version = 2", "schema_version = 3"))?;
    expected.execute_batch(TABLES)?;
    // Preserve token boundaries: TEXT NOT NULL must not collide with TEXTNOTNULL.
    let normalize = |rows: Vec<SchemaObject>| {
        rows.into_iter()
            .map(|(a, b, c, d)| {
                (
                    a,
                    b,
                    c,
                    d.map(|s| s.split_whitespace().collect::<Vec<_>>().join(" ")),
                )
            })
            .collect::<Vec<_>>()
    };
    if normalize(schema_objects(db)?) != normalize(schema_objects(&expected)?) {
        return Err(TaskError::Invalid);
    }
    Ok(())
}
pub(crate) fn is_pristine(db: &Connection) -> Result<bool, TaskError> {
    Ok(
        db.query_row("SELECT count(*) FROM research_tasks", [], |r| {
            r.get::<_, i64>(0)
        })? == 0,
    )
}
pub(crate) fn guard_ordinary(db: &Connection, task: &str) -> Result<(), TaskError> {
    if db
        .query_row(
            "SELECT 1 FROM research_tasks WHERE task_id=?1",
            [task],
            |_| Ok(()),
        )
        .optional()?
        .is_some()
    {
        return Err(TaskError::Unauthorized);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchKey {
    pub task_id: String,
    pub subject_id: String,
    pub request_digest: String,
}
impl ResearchKey {
    fn validate(&self) -> Result<(), TaskError> {
        if !valid_id(&self.task_id)
            || !valid_id(&self.subject_id)
            || !valid_digest(&self.request_digest)
        {
            return Err(TaskError::Invalid);
        }
        Ok(())
    }
    fn id(&self) -> Result<String, TaskError> {
        research_digest("key", self)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProducerRole {
    Preparer,
    ResearchNotice,
    DecisionNotice,
    Adopter,
}
/// Not serializable or constructible by an operation DTO. Trusted startup
/// installation binds store/key/role/exact scope under its external caller contract.
pub struct ResearchProducer {
    store_id: String,
    key: ResearchKey,
    role: ProducerRole,
    admission: Option<ResearchAdmissionAssociation>,
}
impl ResearchProducer {
    fn check(
        &self,
        db: &Connection,
        key: &ResearchKey,
        role: ProducerRole,
    ) -> Result<(), TaskError> {
        let store: String = db.query_row(
            "SELECT store_id FROM store_identity WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        if self.store_id != store || self.key != *key || self.role != role {
            return Err(TaskError::Unauthorized);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedArtifact {
    pub id: String,
    pub digest: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryBinding {
    pub request_digest: String,
    pub card_digest: String,
    pub input_artifact: String,
    pub complete: bool,
    pub advisory_task: String,
    pub advisory_subject: String,
    pub context: String,
    pub attempt: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecommendationBinding {
    pub delivery_digest: String,
    pub request_digest: String,
    pub target_digest: String,
    pub context: String,
    pub result_artifact: String,
    pub action: String,
}
/// Controlled retained result format; these bindings provide evidence, never authority.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckedRecommendation {
    #[serde(deserialize_with = "unique_dependency_map")]
    dependencies: BTreeMap<String, String>,
    text: String,
}
impl CheckedRecommendation {
    fn decode(bytes: &[u8]) -> Result<Self, TaskError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}
fn unique_dependency_map<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct UniqueMap;
    impl<'de> serde::de::Visitor<'de> for UniqueMap {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a dependency map with unique selectors")
        }
        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate dependency selector"));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(UniqueMap)
}
/// Controlled qualification DTO. It is evidence data, never authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasisBundle {
    pub version: u32,
    pub key: ResearchKey,
    pub submit: SubmitTask,
    pub target_digest: String,
    pub snapshot_ref: String,
    pub dependencies: BTreeMap<String, String>,
    pub dependency_artifacts: BTreeMap<String, String>,
    pub artifacts: Vec<RetainedArtifact>,
    pub delivery: DeliveryBinding,
    pub recommendation: RecommendationBinding,
    pub source_bundle_digest: String,
    pub reconstruction_digest: String,
    pub authorization_digest: String,
    pub installation_digest: String,
    pub configuration_digest: String,
}
/// Checked by a trusted preparer. Caller labels and hashes cannot construct this value.
pub struct VerifiedBasis {
    basis: BasisBundle,
    digest: String,
}
impl BasisBundle {
    fn validate(&self) -> Result<(), TaskError> {
        self.key.validate()?;
        validate_submit(&self.submit)?;
        if self
            .submit
            .facts
            .iter()
            .any(|f| f.id.starts_with(RESERVED_PREFIX))
        {
            return Err(TaskError::Invalid);
        }
        if self.version != 1
            || self.key.task_id != self.submit.task_id
            || self.key.subject_id != self.submit.subject_id
            || self.snapshot_ref != self.submit.snapshot_ref
            || self.dependencies.is_empty()
            || self.dependencies.len() > 256
            || self
                .dependencies
                .iter()
                .any(|(k, v)| !valid_id(k) || !valid_digest(v))
            || self.artifacts.is_empty()
            || self.artifacts.len() > 128
            || [
                &self.target_digest,
                &self.source_bundle_digest,
                &self.reconstruction_digest,
                &self.authorization_digest,
                &self.installation_digest,
                &self.configuration_digest,
                &self.delivery.card_digest,
            ]
            .into_iter()
            .any(|s| !valid_digest(s))
            || self.delivery.request_digest != self.key.request_digest
            || !self.delivery.complete
            || self.delivery.advisory_task == self.key.task_id
            || self.delivery.advisory_subject == self.key.subject_id
            || [
                &self.delivery.advisory_task,
                &self.delivery.advisory_subject,
                &self.delivery.context,
                &self.delivery.attempt,
                &self.recommendation.action,
            ]
            .into_iter()
            .any(|s| !valid_id(s))
            || self.recommendation.delivery_digest != research_digest("delivery", &self.delivery)?
            || self.recommendation.request_digest != self.key.request_digest
            || self.recommendation.target_digest != self.target_digest
            || self.recommendation.context != self.delivery.context
        {
            return Err(TaskError::Invalid);
        }
        let mut ids = BTreeSet::new();
        let mut total = 0usize;
        for artifact in &self.artifacts {
            total = total
                .checked_add(artifact.bytes.len())
                .ok_or(TaskError::Capacity)?;
            if !valid_id(&artifact.id)
                || !ids.insert(&artifact.id)
                || artifact.bytes.is_empty()
                || sha256(&artifact.bytes) != artifact.digest
            {
                return Err(TaskError::Invalid);
            }
        }
        if total > MAX_ARTIFACT_BYTES {
            return Err(TaskError::Capacity);
        }
        if !ids.contains(&self.delivery.input_artifact)
            || !ids.contains(&self.recommendation.result_artifact)
        {
            return Err(TaskError::Unavailable);
        }
        if self.dependency_artifacts.keys().collect::<BTreeSet<_>>()
            != self.dependencies.keys().collect()
            || self
                .dependency_artifacts
                .values()
                .any(|id| !ids.contains(id))
        {
            return Err(TaskError::Unavailable);
        }
        for (selector, id) in &self.dependency_artifacts {
            let artifact = self
                .artifacts
                .iter()
                .find(|a| &a.id == id)
                .ok_or(TaskError::Unavailable)?;
            if self.dependencies.get(selector) != Some(&artifact.digest) {
                return Err(TaskError::Stale);
            }
        }
        let result = self
            .artifacts
            .iter()
            .find(|a| a.id == self.recommendation.result_artifact)
            .ok_or(TaskError::Unavailable)?;
        let checked = CheckedRecommendation::decode(&result.bytes)?;
        if checked.dependencies != self.dependencies
            || checked.text.is_empty()
            || checked.text.contains('\0')
        {
            return Err(TaskError::Stale);
        }
        let retained: BTreeSet<&str> = self.artifacts.iter().map(|a| a.digest.as_str()).collect();
        if [
            &self.target_digest,
            &self.source_bundle_digest,
            &self.reconstruction_digest,
            &self.authorization_digest,
            &self.installation_digest,
            &self.configuration_digest,
            &self.delivery.card_digest,
        ]
        .into_iter()
        .any(|d| !retained.contains(d.as_str()))
        {
            return Err(TaskError::Unavailable);
        }
        let delivered = self
            .artifacts
            .iter()
            .find(|a| a.id == self.delivery.input_artifact)
            .ok_or(TaskError::Unavailable)?;
        let card = self
            .artifacts
            .iter()
            .find(|a| a.digest == self.delivery.card_digest)
            .ok_or(TaskError::Unavailable)?;
        if !delivered
            .bytes
            .windows(card.bytes.len())
            .any(|window| window == card.bytes)
        {
            return Err(TaskError::Unavailable);
        }
        let mut metadata = self.clone();
        for a in &mut metadata.artifacts {
            a.bytes.clear();
        }
        if serde_json::to_vec(&metadata)?.len() > MAX_METADATA
            || serde_json::to_vec(&self.delivery)?.len() > MAX_METADATA
            || serde_json::to_vec(&self.recommendation)?.len() > MAX_METADATA
        {
            return Err(TaskError::Capacity);
        }
        Ok(())
    }
}
pub fn research_digest<T: Serialize>(kind: &str, value: &T) -> Result<String, TaskError> {
    let mut bytes = format!("work-engine-research-{kind}-v1\0").into_bytes();
    bytes.extend(serde_json::to_vec(value)?);
    Ok(sha256(&bytes))
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preparation {
    pub key: ResearchKey,
    pub revision: u64,
    pub ready: bool,
    pub capacity_blocked: bool,
    pub basis_digest: Option<String>,
    pub dependencies: BTreeMap<String, String>,
    pub unresolved: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeNotice {
    pub selector: String,
    pub source_revision: String,
    pub reason: String,
}
/// Downstream current-startup/operation association. Data, never an authority permit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchAdmissionAssociation {
    pub qualification: String,
    pub producer_role: String,
    pub operation: String,
    pub startup_digest: String,
    pub installation_digest: String,
    pub configuration_digest: String,
    pub operation_manifest_digest: String,
    pub authority_admission_digest: String,
    pub operation_admission_digest: String,
    pub caller_evidence_digest: String,
    pub payload_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_source_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_evidence_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_realization_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_host_identity_digest: Option<String>,
    /// Complete actual pre-mutation observation, retained with this outcome.
    /// Data only; never projected into native material or tool instructions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currentness_evidence: Option<RetainedArtifact>,
}
impl ResearchAdmissionAssociation {
    fn validate(&self) -> Result<(), TaskError> {
        if !matches!(
            self.qualification.as_str(),
            "controlled_only" | "owner_admitted_startup"
        ) || self.operation != "research.invalidate.v1"
            || !matches!(
                self.producer_role.as_str(),
                "research_notice" | "decision_notice"
            )
            || [
                &self.startup_digest,
                &self.installation_digest,
                &self.configuration_digest,
                &self.operation_manifest_digest,
                &self.authority_admission_digest,
                &self.operation_admission_digest,
                &self.caller_evidence_digest,
                &self.payload_digest,
            ]
            .into_iter()
            .any(|s| !valid_digest(s))
        {
            return Err(TaskError::Invalid);
        }
        if let Some(evidence) = &self.currentness_evidence {
            if evidence.bytes.is_empty()
                || evidence.bytes.len() > MAX_METADATA
                || evidence.id != "native-currentness-v1"
                || sha256(&evidence.bytes) != evidence.digest
            {
                return Err(TaskError::Invalid);
            }
        }
        if self.qualification == "owner_admitted_startup"
            && [
                &self.selection_digest,
                &self.operator_source_digest,
                &self.protection_evidence_digest,
                &self.native_realization_digest,
                &self.current_host_identity_digest,
            ]
            .into_iter()
            .any(|s| s.as_ref().is_none_or(|s| !valid_digest(s)))
        {
            return Err(TaskError::Unauthorized);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NoticeReceipt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission: Option<ResearchAdmissionAssociation>,
    pub key: ResearchKey,
    pub notice_id: String,
    pub revision: u64,
    pub payload_digest: String,
    pub capacity_blocked: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchAdopt {
    pub submit: SubmitTask,
    pub key: ResearchKey,
    pub expected_preparation_revision: u64,
    pub prepared_basis_digest: String,
}
impl ResearchAdopt {
    pub fn payload_digest(&self) -> Result<String, TaskError> {
        research_digest("adopt", self)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdoptionReceipt {
    pub command_id: String,
    pub key: ResearchKey,
    pub preparation_revision: u64,
    pub basis_digest: String,
    pub payload_digest: String,
    pub input_id: String,
    pub owner: TaskCommit,
    pub command: AdmissionCommand,
    pub producer_store_id: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Reconsideration {
    pub notice: NoticeReceipt,
    pub adoption: AdoptionReceipt,
}

fn head(db: &Connection, key: &ResearchKey) -> Result<Option<Preparation>, TaskError> {
    let bytes: Option<Option<Vec<u8>>> = db
        .query_row(
            "SELECT head_json FROM research_keys WHERE key_id=?1",
            [key.id()?],
            |r| r.get(0),
        )
        .optional()?;
    bytes
        .flatten()
        .map(|b| serde_json::from_slice(&b).map_err(TaskError::from))
        .transpose()
}
fn save_head(
    tx: &rusqlite::Transaction<'_>,
    key: &ResearchKey,
    p: &Preparation,
) -> Result<(), TaskError> {
    let bytes = serde_json::to_vec(p)?;
    tx.execute(
        "UPDATE research_keys SET head_json=?2 WHERE key_id=?1",
        params![key.id()?, bytes],
    )?;
    Ok(())
}
fn next_revision(p: Option<&Preparation>) -> Result<u64, TaskError> {
    p.map_or(Some(1), |p| p.revision.checked_add(1))
        .filter(|r| *r <= i64::MAX as u64)
        .ok_or(TaskError::Capacity)
}
fn bounded_rows<T: serde::de::DeserializeOwned>(
    db: &Connection,
    sql: &str,
    args: impl rusqlite::Params,
) -> Result<Vec<T>, TaskError> {
    let mut stmt = db.prepare(sql)?;
    let rows = stmt.query_map(args, |r| r.get::<_, Vec<u8>>(0))?;
    let bytes = rows.collect::<Result<Vec<_>, _>>()?;
    if bytes.len() > 64 {
        return Err(TaskError::Capacity);
    }
    bytes
        .into_iter()
        .map(|b| serde_json::from_slice(&b).map_err(TaskError::from))
        .collect()
}
fn nominate_in(db: &Connection, task: &str, subject: &str) -> Result<(), TaskError> {
    if !valid_id(task) || !valid_id(subject) {
        return Err(TaskError::Invalid);
    }
    let prior: Option<String> = db
        .query_row(
            "SELECT subject_id FROM research_tasks WHERE task_id=?1",
            [task],
            |r| r.get(0),
        )
        .optional()?;
    if prior.as_deref().is_some_and(|s| s != subject) {
        return Err(TaskError::Conflict);
    }
    if let Some(record) = read_task(db, task)?
        && record.subject_id != subject
    {
        return Err(TaskError::Conflict);
    }
    db.execute(
        "INSERT OR IGNORE INTO research_tasks VALUES(?1,?2)",
        params![task, subject],
    )?;
    Ok(())
}
fn register_in(
    db: &Connection,
    key: &ResearchKey,
    selectors: &BTreeSet<String>,
    producer: &ResearchProducer,
) -> Result<(), TaskError> {
    key.validate()?;
    if selectors.is_empty()
        || selectors.len() > 256
        || selectors
            .iter()
            .any(|s| !valid_id(s) || !(s.starts_with("research.") || s.starts_with("decision.")))
    {
        return Err(TaskError::Invalid);
    }
    producer.check(db, key, ProducerRole::Preparer)?;
    let subject: String = db
        .query_row(
            "SELECT subject_id FROM research_tasks WHERE task_id=?1",
            [&key.task_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(TaskError::Unauthorized)?;
    if subject != key.subject_id {
        return Err(TaskError::Unauthorized);
    }
    let old: Option<Vec<u8>> = db
        .query_row(
            "SELECT selectors_json FROM research_keys WHERE key_id=?1",
            [key.id()?],
            |r| r.get(0),
        )
        .optional()?;
    let bytes = serde_json::to_vec(selectors)?;
    if let Some(old) = old {
        return if old == bytes {
            Ok(())
        } else {
            Err(TaskError::Conflict)
        };
    }
    let count: i64 = db.query_row(
        "SELECT count(*) FROM research_keys WHERE task_id=?1",
        [&key.task_id],
        |r| r.get(0),
    )?;
    if count >= 16 {
        return Err(TaskError::Capacity);
    }
    db.execute(
        "INSERT INTO research_keys(key_id,task_id,key_json,selectors_json) VALUES(?1,?2,?3,?4)",
        params![key.id()?, key.task_id, serde_json::to_vec(key)?, bytes],
    )?;
    Ok(())
}
impl TaskStore {
    /// Trusted host nomination adds a guard only; it grants no producer authority.
    pub fn nominate_research_task(&self, task: &str, subject: &str) -> Result<(), TaskError> {
        if !valid_id(task) || !valid_id(subject) {
            return Err(TaskError::Invalid);
        }
        let mut db = self.lock()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        nominate_in(&tx, task, subject)?;
        tx.commit()?;
        Ok(())
    }
    #[cfg(feature = "controlled-proof")]
    pub fn controlled_research_producer(
        &self,
        key: &ResearchKey,
        role: ProducerRole,
    ) -> Result<ResearchProducer, TaskError> {
        key.validate()?;
        Ok(ResearchProducer {
            store_id: self.store_id()?,
            key: key.clone(),
            role,
            admission: None,
        })
    }
    /// Trusted host-composition entry. The caller MUST hold the externally
    /// established, current owner-admitted startup/source/protection consequence.
    /// Record deserialization is not that consequence. This internal Rust API is
    /// not an authentication service and is never exposed over task/native RPC.
    /// Only exact notices are supported; no production basis is constructed.
    pub fn install_notice_from_admitted_startup(
        &self,
        key: &ResearchKey,
        role: ProducerRole,
        admission: ResearchAdmissionAssociation,
    ) -> Result<ResearchProducer, TaskError> {
        admission.validate()?;
        let expected = match role {
            ProducerRole::ResearchNotice => "research_notice",
            ProducerRole::DecisionNotice => "decision_notice",
            _ => return Err(TaskError::Unavailable),
        };
        if admission.producer_role != expected {
            return Err(TaskError::Unauthorized);
        }
        key.validate()?;
        Ok(ResearchProducer {
            store_id: self.store_id()?,
            key: key.clone(),
            role,
            admission: Some(admission),
        })
    }
    /// Trusted startup setup, after complete external admission validation. The
    /// host owns source/caller/protection admission; this owner keeps setup atomic.
    pub fn register_key_from_admitted_startup(
        &self,
        key: &ResearchKey,
        selectors: &BTreeSet<String>,
    ) -> Result<(), TaskError> {
        key.validate()?;
        let registrar = ResearchProducer {
            store_id: self.store_id()?,
            key: key.clone(),
            role: ProducerRole::Preparer,
            admission: None,
        };
        let mut db = self.lock()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        nominate_in(&tx, &key.task_id, &key.subject_id)?;
        register_in(&tx, key, selectors, &registrar)?;
        tx.commit()?;
        Ok(())
    }
    /// Synthetic qualification of an installed exact intent. This is deliberately
    /// unavailable in the default product and does not authenticate owner sources.
    #[cfg(feature = "controlled-proof")]
    pub fn controlled_scoped_research_producer(
        &self,
        key: &ResearchKey,
        role: ProducerRole,
        admission: ResearchAdmissionAssociation,
    ) -> Result<ResearchProducer, TaskError> {
        admission.validate()?;
        if !matches!(
            role,
            ProducerRole::ResearchNotice | ProducerRole::DecisionNotice
        ) {
            return Err(TaskError::Unavailable);
        }
        let expected = match role {
            ProducerRole::ResearchNotice => "research_notice",
            ProducerRole::DecisionNotice => "decision_notice",
            _ => return Err(TaskError::Unavailable),
        };
        if admission.producer_role != expected {
            return Err(TaskError::Unauthorized);
        }
        let mut producer = self.controlled_research_producer(key, role)?;
        producer.admission = Some(admission);
        Ok(producer)
    }
    #[cfg(feature = "controlled-proof")]
    pub fn controlled_verify_research_basis(
        &self,
        basis: BasisBundle,
        producer: &ResearchProducer,
    ) -> Result<VerifiedBasis, TaskError> {
        basis.validate()?;
        producer.check(&*self.lock()?, &basis.key, ProducerRole::Preparer)?;
        let digest = research_digest("basis", &basis)?;
        Ok(VerifiedBasis { basis, digest })
    }
    pub fn register_research_key(
        &self,
        key: &ResearchKey,
        selectors: &BTreeSet<String>,
        producer: &ResearchProducer,
    ) -> Result<(), TaskError> {
        let mut db = self.lock()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        register_in(&tx, key, selectors, producer)?;
        tx.commit()?;
        Ok(())
    }
    /// One controlled owner transaction prevents a failed installed-key setup
    /// from publishing a partial nomination guard.
    #[cfg(feature = "controlled-proof")]
    pub fn controlled_install_research_key(
        &self,
        key: &ResearchKey,
        selectors: &BTreeSet<String>,
    ) -> Result<(), TaskError> {
        self.register_key_from_admitted_startup(key, selectors)
    }
    pub fn prepare_research(
        &self,
        key: &ResearchKey,
        expected: Option<u64>,
        basis: VerifiedBasis,
        producer: &ResearchProducer,
    ) -> Result<Preparation, TaskError> {
        key.validate()?;
        basis.basis.validate()?;
        let mut db = self.lock()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        producer.check(&tx, key, ProducerRole::Preparer)?;
        if basis.basis.key != *key {
            return Err(TaskError::Conflict);
        }
        let (selectors, count): (Vec<u8>, i64) = tx
            .query_row(
                "SELECT selectors_json,preparation_count FROM research_keys WHERE key_id=?1",
                [key.id()?],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or(TaskError::Unavailable)?;
        let selectors: BTreeSet<String> = serde_json::from_slice(&selectors)?;
        if selectors != basis.basis.dependencies.keys().cloned().collect() {
            return Err(TaskError::Invalid);
        }
        let old = head(&tx, key)?;
        if old.as_ref().map(|p| p.revision) != expected {
            return Err(TaskError::Stale);
        }
        if count >= 256 || old.as_ref().is_some_and(|p| p.capacity_blocked) {
            return Err(TaskError::Capacity);
        }
        if let Some(old) = old.as_ref() {
            if old.unresolved.iter().any(|(k, v)| {
                basis.basis.dependencies.get(k) != Some(v) || old.dependencies.get(k) == Some(v)
            }) {
                return Err(TaskError::Stale);
            }
            if !old.unresolved.is_empty() {
                if old.basis_digest.as_ref() == Some(&basis.digest) {
                    return Err(TaskError::Stale);
                }
                if let Some(digest) = &old.basis_digest {
                    let bytes:Vec<u8>=tx.query_row("SELECT basis_json FROM research_basis WHERE key_id=?1 AND basis_digest=?2 ORDER BY revision DESC LIMIT 1",params![key.id()?,digest],|r|r.get(0))?;
                    let previous: BasisBundle = serde_json::from_slice(&bytes)?;
                    let artifact_bytes = |b: &BasisBundle, id: &str| {
                        b.artifacts
                            .iter()
                            .find(|a| a.id == id)
                            .map(|a| a.digest.clone())
                    };
                    for selector in old.unresolved.keys() {
                        if artifact_bytes(&previous, &previous.dependency_artifacts[selector])
                            == artifact_bytes(
                                &basis.basis,
                                &basis.basis.dependency_artifacts[selector],
                            )
                        {
                            return Err(TaskError::Stale);
                        }
                    }
                    if artifact_bytes(&previous, &previous.recommendation.result_artifact)
                        == artifact_bytes(&basis.basis, &basis.basis.recommendation.result_artifact)
                    {
                        return Err(TaskError::Stale);
                    }
                }
            }
        }
        let p = Preparation {
            key: key.clone(),
            revision: next_revision(old.as_ref())?,
            ready: true,
            capacity_blocked: false,
            basis_digest: Some(basis.digest.clone()),
            dependencies: basis.basis.dependencies.clone(),
            unresolved: BTreeMap::new(),
        };
        tx.execute(
            "INSERT INTO research_basis VALUES(?1,?2,?3,?4)",
            params![
                key.id()?,
                p.revision,
                basis.digest,
                serde_json::to_vec(&basis.basis)?
            ],
        )?;
        save_head(&tx, key, &p)?;
        tx.execute(
            "INSERT INTO research_history VALUES(?1,?2,?3)",
            params![key.id()?, p.revision, serde_json::to_vec(&p)?],
        )?;
        tx.execute(
            "UPDATE research_keys SET preparation_count=preparation_count+1 WHERE key_id=?1",
            [key.id()?],
        )?;
        tx.commit()?;
        Ok(p)
    }
    /// Exact historical notice readback does not construct or require a fresh
    /// producer. Changed payload under the same notice identity conflicts.
    pub fn read_research_notice(
        &self,
        key: &ResearchKey,
        notice_id: &str,
        payload_digest: &str,
    ) -> Result<Option<NoticeReceipt>, TaskError> {
        key.validate()?;
        if !valid_id(notice_id) || !valid_digest(payload_digest) {
            return Err(TaskError::Invalid);
        }
        let db = self.lock()?;
        for table in ["research_notices", "research_capacity"] {
            let row:Option<(String,Vec<u8>)>=db.query_row(&format!("SELECT payload_digest,receipt_json FROM {table} WHERE key_id=?1 AND notice_id=?2"),params![key.id()?,notice_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            if let Some((digest, bytes)) = row {
                if digest != payload_digest {
                    return Err(TaskError::Conflict);
                }
                return Ok(Some(serde_json::from_slice(&bytes)?));
            }
        }
        Ok(None)
    }
    pub fn invalidate_research(
        &self,
        key: &ResearchKey,
        notice_id: &str,
        notice: &ChangeNotice,
        producer: &ResearchProducer,
    ) -> Result<NoticeReceipt, TaskError> {
        key.validate()?;
        if !valid_id(notice_id)
            || !valid_id(&notice.selector)
            || !valid_digest(&notice.source_revision)
            || !valid_text(&notice.reason)
        {
            return Err(TaskError::Invalid);
        }
        let digest = research_digest("notice", &(key, notice_id, notice))?;
        if let Some(admission) = &producer.admission {
            admission.validate()?;
            if admission.payload_digest != digest {
                return Err(TaskError::Unauthorized);
            }
        }
        let mut db = self.lock()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !matches!(
            producer.role,
            ProducerRole::ResearchNotice | ProducerRole::DecisionNotice
        ) {
            return Err(TaskError::Unauthorized);
        }
        producer.check(&tx, key, producer.role)?;
        if (notice.selector.starts_with("research.")
            && producer.role != ProducerRole::ResearchNotice)
            || (notice.selector.starts_with("decision.")
                && producer.role != ProducerRole::DecisionNotice)
        {
            return Err(TaskError::Unauthorized);
        }
        for table in ["research_notices", "research_capacity"] {
            let old:Option<(String,Vec<u8>)>=tx.query_row(&format!("SELECT payload_digest,receipt_json FROM {table} WHERE key_id=?1 AND notice_id=?2"),params![key.id()?,notice_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            if let Some((old, receipt)) = old {
                return if old == digest {
                    Ok(serde_json::from_slice(&receipt)?)
                } else {
                    Err(TaskError::Conflict)
                };
            }
        }
        let selectors: Vec<u8> = tx
            .query_row(
                "SELECT selectors_json FROM research_keys WHERE key_id=?1",
                [key.id()?],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(TaskError::Unavailable)?;
        if !serde_json::from_slice::<BTreeSet<String>>(&selectors)?.contains(&notice.selector) {
            return Err(TaskError::Unauthorized);
        }
        let old = head(&tx, key)?;
        if old.as_ref().is_some_and(|p| p.capacity_blocked) {
            return Err(TaskError::Capacity);
        }
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM research_notices WHERE key_id=?1",
            [key.id()?],
            |r| r.get(0),
        )?;
        let revision = next_revision(old.as_ref()).unwrap_or(i64::MAX as u64);
        let blocked = count >= 1024 || old.as_ref().is_some_and(|p| p.revision >= i64::MAX as u64);
        let mut p = old.unwrap_or(Preparation {
            key: key.clone(),
            revision: 0,
            ready: false,
            capacity_blocked: false,
            basis_digest: None,
            dependencies: BTreeMap::new(),
            unresolved: BTreeMap::new(),
        });
        p.revision = revision;
        p.ready = false;
        p.capacity_blocked = blocked;
        p.unresolved
            .insert(notice.selector.clone(), notice.source_revision.clone());
        let receipt = NoticeReceipt {
            admission: producer.admission.clone(),
            key: key.clone(),
            notice_id: notice_id.into(),
            revision,
            payload_digest: digest.clone(),
            capacity_blocked: blocked,
        };
        // Validate the actual encoded receipt, including byte-array expansion,
        // before the first outcome mutation. Never commit an unreadable result.
        let receipt_bytes = serde_json::to_vec(&receipt)?;
        if receipt_bytes.len() > MAX_NOTICE_RECEIPT_BYTES {
            return Err(TaskError::Capacity);
        }
        save_head(&tx, key, &p)?;
        if blocked {
            tx.execute(
                "INSERT INTO research_capacity VALUES(?1,?2,?3,?4)",
                params![key.id()?, notice_id, digest, serde_json::to_vec(&receipt)?],
            )?;
        } else {
            tx.execute(
                "INSERT INTO research_notices VALUES(?1,?2,?3,?4,?5)",
                params![
                    key.id()?,
                    notice_id,
                    digest,
                    serde_json::to_vec(notice)?,
                    serde_json::to_vec(&receipt)?
                ],
            )?;
        }
        // At most 64 adoptions by owner bound. Do not truncate reconsideration obligations.
        let adoptions: Vec<AdoptionReceipt> = bounded_rows(
            &tx,
            "SELECT receipt_json FROM research_adoptions WHERE key_id=?1 LIMIT 65",
            [key.id()?],
        )?;
        for adoption in adoptions {
            let row = Reconsideration {
                notice: receipt.clone(),
                adoption,
            };
            tx.execute(
                "INSERT INTO research_reconsideration VALUES(?1,?2,?3,?4)",
                params![
                    key.id()?,
                    notice_id,
                    row.adoption.command_id,
                    serde_json::to_vec(&row)?
                ],
            )?;
        }
        tx.commit()?;
        Ok(receipt)
    }
    pub fn read_research_preparation(
        &self,
        key: &ResearchKey,
    ) -> Result<Option<Preparation>, TaskError> {
        key.validate()?;
        head(&*self.lock()?, key)
    }
    pub fn read_research_basis(
        &self,
        key: &ResearchKey,
        revision: u64,
        digest: &str,
        producer: &ResearchProducer,
    ) -> Result<Option<BasisBundle>, TaskError> {
        key.validate()?;
        let db = self.lock()?;
        producer.check(&db, key, ProducerRole::Preparer)?;
        let row:Option<Vec<u8>>=db.query_row("SELECT basis_json FROM research_basis WHERE key_id=?1 AND revision=?2 AND basis_digest=?3",params![key.id()?,revision,digest],|r|r.get(0)).optional()?;
        row.map(|bytes| {
            let basis: BasisBundle = serde_json::from_slice(&bytes)?;
            basis.validate()?;
            if research_digest("basis", &basis)? != digest {
                return Err(TaskError::Uncertain);
            }
            Ok(basis)
        })
        .transpose()
    }
    pub fn read_research_adoption(
        &self,
        command_id: &str,
        payload_digest: &str,
    ) -> Result<Option<AdoptionReceipt>, TaskError> {
        let db = self.lock()?;
        read_adoption(&db, command_id, payload_digest)
    }
    pub fn read_research_reconsideration(
        &self,
        key: &ResearchKey,
    ) -> Result<Vec<Reconsideration>, TaskError> {
        key.validate()?;
        bounded_rows(
            &*self.lock()?,
            "SELECT receipt_json FROM research_reconsideration WHERE key_id=?1 ORDER BY notice_id,command_id LIMIT 65",
            [key.id()?],
        )
    }
    pub fn adopt_research_with_admission(
        &self,
        request: &ResearchAdopt,
        producer: &ResearchProducer,
        build: &mut AdmissionBuilder<'_>,
    ) -> Result<(AdoptionReceipt, bool), TaskError> {
        request.key.validate()?;
        validate_submit(&request.submit)?;
        let digest = request.payload_digest()?;
        let mut db = self.lock()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        producer.check(&tx, &request.key, ProducerRole::Adopter)?;
        if let Some(old) = read_adoption(&tx, &request.submit.command_id, &digest)? {
            return Ok((old, false));
        }
        if command_row(&tx, &request.submit.command_id)?.is_some() {
            return Err(TaskError::Conflict);
        }
        let p = head(&tx, &request.key)?.ok_or(TaskError::Unavailable)?;
        if !p.ready
            || p.capacity_blocked
            || p.revision != request.expected_preparation_revision
            || p.basis_digest.as_ref() != Some(&request.prepared_basis_digest)
        {
            return Err(TaskError::Stale);
        }
        let bytes:Vec<u8>=tx.query_row("SELECT basis_json FROM research_basis WHERE key_id=?1 AND revision=?2 AND basis_digest=?3",params![request.key.id()?,p.revision,request.prepared_basis_digest],|r|r.get(0))?;
        let basis: BasisBundle = serde_json::from_slice(&bytes)?;
        basis.validate()?;
        if basis.submit != request.submit
            || basis.key != request.key
            || research_digest("basis", &basis)? != request.prepared_basis_digest
        {
            return Err(TaskError::Conflict);
        }
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM research_adoptions WHERE task_id=?1",
            [&request.key.task_id],
            |r| r.get(0),
        )?;
        if count >= 64 {
            return Err(TaskError::Capacity);
        }
        let result = basis
            .artifacts
            .iter()
            .find(|a| a.id == basis.recommendation.result_artifact)
            .ok_or(TaskError::Unavailable)?;
        let checked = CheckedRecommendation::decode(&result.bytes)?;
        let recommendation_text = checked.text;
        let canonical = serde_json::to_string(
            &serde_json::json!({"basis_digest":request.prepared_basis_digest,"preparation_revision":p.revision,"key":request.key,"delivery":basis.delivery,"recommendation":basis.recommendation,"recommendation_text":recommendation_text,"recommendation_sha256":result.digest,"dependencies":basis.dependencies,"dependency_artifacts":basis.dependency_artifacts,"target_digest":basis.target_digest,"snapshot_ref":basis.snapshot_ref,"source_bundle_digest":basis.source_bundle_digest,"reconstruction_digest":basis.reconstruction_digest,"authorization_digest":basis.authorization_digest,"installation_digest":basis.installation_digest,"configuration_digest":basis.configuration_digest}),
        )?;
        if canonical.len() > MAX_TEXT {
            return Err(TaskError::Capacity);
        }
        let source_id = format!("research-{}", &request.prepared_basis_digest);
        let owner = publish_submit(
            &tx,
            &request.submit,
            "research_adopt",
            &digest,
            Some(build),
            Some((&source_id, &canonical)),
        )?;
        let command:AdmissionCommand=tx.query_row("SELECT command_id,request_digest,command_json FROM admission_commands WHERE command_id=?1",[&request.submit.command_id],|r|Ok(AdmissionCommand{command_id:r.get(0)?,request_digest:r.get(1)?,command_json:r.get(2)?}))?;
        let receipt = AdoptionReceipt {
            command_id: request.submit.command_id.clone(),
            key: request.key.clone(),
            preparation_revision: p.revision,
            basis_digest: request.prepared_basis_digest.clone(),
            payload_digest: digest.clone(),
            input_id: format!("input:{}", request.submit.command_id),
            owner,
            command,
            producer_store_id: producer.store_id.clone(),
        };
        tx.execute(
            "INSERT INTO research_adoptions VALUES(?1,?2,?3,?4,?5)",
            params![
                receipt.command_id,
                request.key.task_id,
                request.key.id()?,
                digest,
                serde_json::to_vec(&receipt)?
            ],
        )?;
        tx.commit()?;
        Ok((receipt, true))
    }
}
fn read_adoption(
    db: &Connection,
    id: &str,
    digest: &str,
) -> Result<Option<AdoptionReceipt>, TaskError> {
    if !valid_id(id) || !valid_digest(digest) {
        return Err(TaskError::Invalid);
    }
    let row: Option<(String, Vec<u8>)> = db
        .query_row(
            "SELECT payload_digest,receipt_json FROM research_adoptions WHERE command_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(old, b)| {
        if old != digest {
            Err(TaskError::Conflict)
        } else {
            Ok(serde_json::from_slice(&b)?)
        }
    })
    .transpose()
}

#[cfg(test)]
pub(crate) fn remove_test_tables(db: &Connection) {
    db.execute_batch("DROP TABLE research_tasks; DROP TABLE research_keys; DROP TABLE research_history; DROP TABLE research_notices; DROP TABLE research_capacity; DROP TABLE research_basis; DROP TABLE research_adoptions; DROP TABLE research_reconsideration;").unwrap();
}

#[cfg(all(test, feature = "controlled-proof"))]
mod tests {
    use super::*;
    fn submit() -> SubmitTask {
        SubmitTask {
            command_id: "command-0".into(),
            task_id: "task-0".into(),
            subject_id: "subject-0".into(),
            kind: SubjectKind::RepositoryAnalysis,
            objective: "Audit fixed source".into(),
            engineering_question: "What is supported?".into(),
            snapshot_ref: "snapshot-0".into(),
            facts: vec![],
        }
    }
    fn bundle() -> BasisBundle {
        let key = ResearchKey {
            task_id: "task-0".into(),
            subject_id: "subject-0".into(),
            request_digest: sha256(b"request"),
        };
        let delivery = DeliveryBinding {
            request_digest: key.request_digest.clone(),
            card_digest: sha256(b"card"),
            input_artifact: "delivered".into(),
            complete: true,
            advisory_task: "advice-task".into(),
            advisory_subject: "advice-subject".into(),
            context: "context-0".into(),
            attempt: "attempt-0".into(),
        };
        let recommendation = RecommendationBinding {
            delivery_digest: research_digest("delivery", &delivery).unwrap(),
            request_digest: key.request_digest.clone(),
            target_digest: sha256(b"target"),
            context: delivery.context.clone(),
            result_artifact: "result".into(),
            action: "audit".into(),
        };
        let result_bytes = serde_json::to_vec(&CheckedRecommendation {
            dependencies: BTreeMap::from([("decision.target".into(), sha256(b"target"))]),
            text: "alternatives and eight uncertainty facets remain uncertain".into(),
        })
        .unwrap();
        BasisBundle {
            version: 1,
            key,
            submit: submit(),
            target_digest: sha256(b"target"),
            snapshot_ref: "snapshot-0".into(),
            dependencies: BTreeMap::from([("decision.target".into(), sha256(b"target"))]),
            dependency_artifacts: BTreeMap::from([("decision.target".into(), "target".into())]),
            artifacts: vec![
                RetainedArtifact {
                    id: "delivered".into(),
                    digest: sha256(b"exact card bytes"),
                    bytes: b"exact card bytes".to_vec(),
                },
                RetainedArtifact {
                    id: "result".into(),
                    digest: sha256(&result_bytes),
                    bytes: result_bytes,
                },
            ],
            delivery,
            recommendation,
            source_bundle_digest: sha256(b"upstream"),
            reconstruction_digest: sha256(b"reconstruction"),
            authorization_digest: sha256(b"authority"),
            installation_digest: sha256(b"controlled-only"),
            configuration_digest: sha256(b"fixture-config"),
        }
    }
    fn reassess(b: &mut BasisBundle) {
        let result = b.artifacts.iter_mut().find(|a| a.id == "result").unwrap();
        result.bytes = serde_json::to_vec(&CheckedRecommendation {
            dependencies: b.dependencies.clone(),
            text: "reconsidered recommendation with uncertainty retained".into(),
        })
        .unwrap();
        result.digest = sha256(&result.bytes);
    }
    fn replace_target(b: &mut BasisBundle, bytes: &[u8]) {
        let target = b.artifacts.iter_mut().find(|a| a.id == "target").unwrap();
        target.bytes = bytes.to_vec();
        target.digest = sha256(bytes);
        b.target_digest = target.digest.clone();
        b.recommendation.target_digest = target.digest.clone();
    }
    fn setup() -> (tempfile::TempDir, TaskStore, BasisBundle) {
        let root = tempfile::tempdir().unwrap();
        let store = TaskStore::open(&root.path().join("owner.sqlite"), "human".into()).unwrap();
        let mut b = bundle();
        for (id, bytes) in [
            ("target", b"target".as_slice()),
            ("upstream", b"upstream"),
            ("reconstruction", b"reconstruction"),
            ("authority", b"authority"),
            ("installation", b"controlled-only"),
            ("configuration", b"fixture-config"),
            ("card", b"card"),
        ] {
            b.artifacts.push(RetainedArtifact {
                id: id.into(),
                digest: sha256(bytes),
                bytes: bytes.to_vec(),
            });
        }
        store
            .nominate_research_task(&b.key.task_id, &b.key.subject_id)
            .unwrap();
        let p = producer(&store, &b.key, ProducerRole::Preparer);
        store
            .register_research_key(&b.key, &b.dependencies.keys().cloned().collect(), &p)
            .unwrap();
        (root, store, b)
    }
    fn producer(s: &TaskStore, k: &ResearchKey, r: ProducerRole) -> ResearchProducer {
        s.controlled_research_producer(k, r).unwrap()
    }
    fn prepare(
        s: &TaskStore,
        b: &BasisBundle,
        expected: Option<u64>,
    ) -> Result<Preparation, TaskError> {
        let p = producer(s, &b.key, ProducerRole::Preparer);
        let v = s.controlled_verify_research_basis(b.clone(), &p)?;
        s.prepare_research(&b.key, expected, v, &p)
    }
    fn request(b: &BasisBundle, p: &Preparation) -> ResearchAdopt {
        ResearchAdopt {
            submit: b.submit.clone(),
            key: b.key.clone(),
            expected_preparation_revision: p.revision,
            prepared_basis_digest: p.basis_digest.clone().unwrap(),
        }
    }
    fn command(q: &ResearchAdopt, text: &str) -> AdmissionCommand {
        let digest = "a".repeat(64);
        AdmissionCommand{command_id:q.submit.command_id.clone(),request_digest:digest.clone(),command_json:serde_json::json!({"protocol_version":2,"principal_ref":"principal-0","command_id":q.submit.command_id,"subject_id":q.submit.subject_id,"context_generation":"context-0","build_id":"build-0","proof_run_id":"proof-0","grant_ref":"grant-0","expected_revision":"1","request_digest":digest,"kind":"enqueue_input","payload":{"input_id":format!("input:{}",q.submit.command_id),"producer_ref":"producer-0","text":text,"text_digest":"b".repeat(64)}}).to_string()}
    }
    fn notice() -> ChangeNotice {
        ChangeNotice {
            selector: "decision.target".into(),
            source_revision: sha256(b"target-2"),
            reason: "target changed".into(),
        }
    }
    #[test]
    fn installed_setup_rolls_back_and_scoped_notice_retry_keeps_original_admission() {
        let root = tempfile::tempdir().unwrap();
        let store = TaskStore::open(&root.path().join("owner.sqlite"), "human".into()).unwrap();
        let b = bundle();
        assert!(
            store
                .controlled_install_research_key(&b.key, &BTreeSet::from(["unsupported".into()]))
                .is_err()
        );
        assert_eq!(
            store
                .lock()
                .unwrap()
                .query_row("SELECT count(*) FROM research_tasks", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        store
            .controlled_install_research_key(
                &b.key,
                &BTreeSet::from(["research.source".into(), "decision.target".into()]),
            )
            .unwrap();
        let mut notice = notice();
        notice.selector = "research.source".into();
        let payload = research_digest("notice", &(&b.key, "installed-notice", &notice)).unwrap();
        let mut association = ResearchAdmissionAssociation {
            qualification: "controlled_only".into(),
            producer_role: "research_notice".into(),
            operation: "research.invalidate.v1".into(),
            startup_digest: sha256(b"controlled startup"),
            installation_digest: sha256(b"I"),
            configuration_digest: sha256(b"C"),
            operation_manifest_digest: sha256(b"O"),
            authority_admission_digest: sha256(b"RI"),
            operation_admission_digest: sha256(b"RO"),
            caller_evidence_digest: sha256(b"synthetic caller"),
            payload_digest: payload,
            selection_digest: None,
            operator_source_digest: None,
            protection_evidence_digest: None,
            native_realization_digest: None,
            current_host_identity_digest: None,
            currentness_evidence: None,
        };
        assert!(
            store
                .controlled_scoped_research_producer(
                    &b.key,
                    ProducerRole::DecisionNotice,
                    association.clone()
                )
                .is_err()
        );
        // Old association JSON remains decodable without the additive field.
        let historical_json = serde_json::to_vec(&association).unwrap();
        assert!(!String::from_utf8_lossy(&historical_json).contains("currentness_evidence"));
        let old: ResearchAdmissionAssociation = serde_json::from_slice(&historical_json).unwrap();
        assert!(old.currentness_evidence.is_none());
        let artifact = |bytes: Vec<u8>| RetainedArtifact {
            id: "native-currentness-v1".into(),
            digest: sha256(&bytes),
            bytes,
        };
        for invalid in [
            RetainedArtifact {
                digest: "0".repeat(64),
                ..artifact(vec![1])
            },
            artifact(Vec::new()),
            artifact(vec![1; MAX_METADATA + 1]),
        ] {
            let mut bad = association.clone();
            bad.currentness_evidence = Some(invalid);
            assert!(
                store
                    .controlled_scoped_research_producer(&b.key, ProducerRole::ResearchNotice, bad)
                    .is_err()
            );
        }
        // Raw bytes fit the metadata cap, but JSON byte-array expansion makes
        // the complete outcome unreadable. Refusal precedes all owner writes.
        let mut too_large = association.clone();
        too_large.currentness_evidence = Some(artifact(vec![255; 40_000]));
        let producer = store
            .controlled_scoped_research_producer(&b.key, ProducerRole::ResearchNotice, too_large)
            .unwrap();
        assert!(matches!(
            store.invalidate_research(&b.key, "installed-notice", &notice, &producer),
            Err(TaskError::Capacity)
        ));
        let db = store.lock().unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM research_notices", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM research_history", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(
            db.query_row("SELECT head_json FROM research_keys", [], |r| r
                .get::<_, Option<Vec<u8>>>(0))
                .unwrap()
                .is_none()
        );
        drop(db);
        association.currentness_evidence = Some(artifact(vec![255; 32_000]));
        let producer = store
            .controlled_scoped_research_producer(
                &b.key,
                ProducerRole::ResearchNotice,
                association.clone(),
            )
            .unwrap();
        let receipt = store
            .invalidate_research(&b.key, "installed-notice", &notice, &producer)
            .unwrap();
        assert!(serde_json::to_vec(&receipt).unwrap().len() <= MAX_NOTICE_RECEIPT_BYTES);
        let response = serde_json::json!({"status":"invalidated","receipt":receipt,"qualification":"owner_admitted_startup"});
        assert!(serde_json::to_vec(&response).unwrap().len() <= 128 * 1024);
        // Native canonical material reads task sources/effects, never this
        // separate admission receipt table; the raw artifact stays owner-only.
        assert!(store.observe_subject(&b.key.subject_id).unwrap().is_none());
        assert_eq!(receipt.admission, Some(association.clone()));
        association.caller_evidence_digest = sha256(b"later synthetic caller");
        let later = store
            .controlled_scoped_research_producer(
                &b.key,
                ProducerRole::ResearchNotice,
                association.clone(),
            )
            .unwrap();
        assert_eq!(
            store
                .invalidate_research(&b.key, "installed-notice", &notice, &later)
                .unwrap(),
            receipt
        );
        let mut changed = notice.clone();
        changed.reason.push('x');
        assert!(matches!(
            store.invalidate_research(&b.key, "installed-notice", &changed, &later),
            Err(TaskError::Unauthorized)
        ));
        drop(store);
        let reopened = TaskStore::open(&root.path().join("owner.sqlite"), "human".into()).unwrap();
        let later = reopened
            .controlled_scoped_research_producer(&b.key, ProducerRole::ResearchNotice, association)
            .unwrap();
        assert_eq!(
            reopened
                .invalidate_research(&b.key, "installed-notice", &notice, &later)
                .unwrap(),
            receipt
        );
    }
    #[test]
    fn owner_guard_all_variants_reserved_sources_and_unrelated_control() {
        let (_root, s, b) = setup();
        assert!(matches!(s.submit(&b.submit), Err(TaskError::Unauthorized)));
        assert!(matches!(
            s.submit_with_status(&b.submit),
            Err(TaskError::Unauthorized)
        ));
        assert!(matches!(
            s.submit_with_admission(&b.submit, &mut |_| panic!("unchecked builder")),
            Err(TaskError::Unauthorized)
        ));
        let mut unrelated = b.submit.clone();
        unrelated.task_id = "ordinary".into();
        unrelated.subject_id = "ordinary".into();
        s.submit(&unrelated).unwrap();
        s.nominate_research_task("ordinary", "ordinary").unwrap();
        assert!(!s.submit_with_status(&unrelated).unwrap().1);
        unrelated.command_id = "fresh-append".into();
        assert!(matches!(s.submit(&unrelated), Err(TaskError::Unauthorized)));
        let mut forged = b.submit.clone();
        forged.command_id = "forged-command".into();
        forged.task_id = "forged".into();
        forged.subject_id = "forged".into();
        forged.facts = vec![FactInput {
            id: "research-forged".into(),
            kind: FactKind::CanonicalRecord,
            text: "trusted".into(),
            current: true,
        }];
        assert!(matches!(s.submit(&forged), Err(TaskError::Invalid)));
    }
    #[test]
    fn preprepare_notice_cas_and_changed_dependencies() {
        let (_root, s, mut b) = setup();
        let n = notice();
        let p = producer(&s, &b.key, ProducerRole::DecisionNotice);
        let receipt = s.invalidate_research(&b.key, "notice-1", &n, &p).unwrap();
        assert_eq!(receipt.revision, 1);
        assert_eq!(
            s.invalidate_research(&b.key, "notice-1", &n, &p).unwrap(),
            receipt
        );
        let mut bad = n.clone();
        bad.reason = "different".into();
        assert!(matches!(
            s.invalidate_research(&b.key, "notice-1", &bad, &p),
            Err(TaskError::Conflict)
        ));
        assert!(matches!(prepare(&s, &b, None), Err(TaskError::Stale)));
        assert!(matches!(prepare(&s, &b, Some(1)), Err(TaskError::Stale)));
        b.dependencies
            .insert(n.selector.clone(), n.source_revision.clone());
        assert!(prepare(&s, &b, Some(1)).is_err());
        assert!(!s.read_research_preparation(&b.key).unwrap().unwrap().ready);
        replace_target(&mut b, b"target-2");
        assert!(prepare(&s, &b, Some(1)).is_err()); // Former recommendation is still stale.
        reassess(&mut b);
        let ready = prepare(&s, &b, Some(1)).unwrap();
        assert_eq!(ready.revision, 2);
        assert!(ready.ready);
        assert!(matches!(prepare(&s, &b, Some(1)), Err(TaskError::Stale)));
    }
    #[test]
    fn invalidation_serializes_and_historical_adoption_survives_reopen() {
        let (root, s, b) = setup();
        let p = prepare(&s, &b, None).unwrap();
        let q = request(&b, &p);
        assert_eq!(
            s.read_research_basis(
                &b.key,
                p.revision,
                p.basis_digest.as_ref().unwrap(),
                &producer(&s, &b.key, ProducerRole::Preparer)
            )
            .unwrap(),
            Some(b.clone())
        );
        let adopter = producer(&s, &b.key, ProducerRole::Adopter);
        let (a, first) = s
            .adopt_research_with_admission(&q, &adopter, &mut |t| Ok(command(&q, t)))
            .unwrap();
        assert!(first);
        assert_eq!(s.admission_commands().unwrap(), vec![a.command.clone()]);
        assert_eq!(
            s.lookup_admission_command(&a.command_id).unwrap(),
            Some(a.command.clone())
        );
        assert_eq!(a.owner.owner_revision, 1);
        assert_eq!(a.command.command_id, b.submit.command_id);
        let record = s.observe("task-0").unwrap().unwrap();
        let canonical = record
            .sources
            .iter()
            .find(|s| s.id.starts_with(RESERVED_PREFIX))
            .unwrap();
        let canonical: serde_json::Value = serde_json::from_slice(&canonical.bytes).unwrap();
        assert_eq!(
            canonical["recommendation_text"],
            "alternatives and eight uncertainty facets remain uncertain"
        );
        assert_eq!(canonical["source_bundle_digest"], b.source_bundle_digest);
        let n = notice();
        s.invalidate_research(
            &b.key,
            "notice-1",
            &n,
            &producer(&s, &b.key, ProducerRole::DecisionNotice),
        )
        .unwrap();
        let rows = s.read_research_reconsideration(&b.key).unwrap();
        assert_eq!(rows[0].adoption, a);
        assert_eq!(s.observe("task-0").unwrap().unwrap(), record);
        drop(s);
        let s = TaskStore::open(&root.path().join("owner.sqlite"), "human".into()).unwrap();
        let a2 = s
            .adopt_research_with_admission(
                &q,
                &producer(&s, &b.key, ProducerRole::Adopter),
                &mut |_| panic!("must not rebase"),
            )
            .unwrap();
        assert_eq!(a2, (a.clone(), false));
        assert_eq!(s.admission_commands().unwrap(), vec![a.command.clone()]);
        let mut conflict = q.clone();
        conflict.expected_preparation_revision += 1;
        assert!(matches!(
            s.adopt_research_with_admission(
                &conflict,
                &producer(&s, &b.key, ProducerRole::Adopter),
                &mut |_| panic!("conflict")
            ),
            Err(TaskError::Conflict)
        ));
        assert_eq!(
            s.read_research_adoption(&a.command_id, &a.payload_digest)
                .unwrap(),
            Some(a)
        );
    }
    #[test]
    fn admission_enumeration_refuses_non_admission_kinds_and_orphans() {
        for kind in [Some("answer"), Some("replacement"), Some("unknown"), None] {
            let (_root, s, b) = setup();
            let p = prepare(&s, &b, None).unwrap();
            let q = request(&b, &p);
            let (adoption, _) = s
                .adopt_research_with_admission(
                    &q,
                    &producer(&s, &b.key, ProducerRole::Adopter),
                    &mut |text| Ok(command(&q, text)),
                )
                .unwrap();
            assert_eq!(s.admission_commands().unwrap(), vec![adoption.command]);
            let db = s.lock().unwrap();
            match kind {
                Some(kind) => {
                    db.execute(
                        "UPDATE commands SET kind=?1 WHERE command_id=?2",
                        params![kind, q.submit.command_id],
                    )
                    .unwrap();
                }
                None => {
                    db.execute(
                        "DELETE FROM commands WHERE command_id=?1",
                        [&q.submit.command_id],
                    )
                    .unwrap();
                }
            }
            drop(db);
            assert!(
                matches!(s.admission_commands(), Err(TaskError::Invalid)),
                "kind={kind:?}"
            );
        }
    }
    #[test]
    fn before_adoption_notice_and_builder_failure_publish_nothing() {
        let (_root, s, b) = setup();
        let p = prepare(&s, &b, None).unwrap();
        let q = request(&b, &p);
        let a = producer(&s, &b.key, ProducerRole::Adopter);
        assert!(matches!(
            s.adopt_research_with_admission(&q, &a, &mut |_| Err(TaskError::Unavailable)),
            Err(TaskError::Unavailable)
        ));
        assert!(s.observe("task-0").unwrap().is_none());
        assert!(s.lookup_admission_command("command-0").unwrap().is_none());
        assert!(
            s.read_research_adoption("command-0", &q.payload_digest().unwrap())
                .unwrap()
                .is_none()
        );
        s.invalidate_research(
            &b.key,
            "notice-1",
            &notice(),
            &producer(&s, &b.key, ProducerRole::DecisionNotice),
        )
        .unwrap();
        assert!(matches!(
            s.adopt_research_with_admission(&q, &a, &mut |_| panic!("stale")),
            Err(TaskError::Stale)
        ));
    }
    fn duplicate_result(b: &mut BasisBundle, first: &str) {
        let last = &b.dependencies["decision.target"];
        let bytes = format!(r#"{{"dependencies":{{"decision.target":"{first}","decision.target":"{last}"}},"text":"advisory uncertainty retained"}}"#).into_bytes();
        let result = b.artifacts.iter_mut().find(|a| a.id == "result").unwrap();
        result.digest = sha256(&bytes);
        result.bytes = bytes;
    }
    #[test]
    fn duplicate_retained_dependency_keys_refuse_before_publication() {
        let mut outcomes = Vec::new();
        for first in [sha256(b"target"), sha256(b"conflicting earlier revision")] {
            let (_root, s, mut b) = setup();
            duplicate_result(&mut b, &first);
            let prepared = prepare(&s, &b, None);
            let mut adopted = false;
            if let Ok(p) = &prepared {
                let q = request(&b, p);
                adopted = s
                    .adopt_research_with_admission(
                        &q,
                        &producer(&s, &b.key, ProducerRole::Adopter),
                        &mut |text| Ok(command(&q, text)),
                    )
                    .is_ok();
            } else {
                assert!(s.read_research_preparation(&b.key).unwrap().is_none());
                assert!(s.observe(&b.key.task_id).unwrap().is_none());
                assert!(
                    s.lookup_admission_command(&b.submit.command_id)
                        .unwrap()
                        .is_none()
                );
                let db = s.lock().unwrap();
                for table in ["research_history", "research_basis", "research_adoptions"] {
                    assert_eq!(
                        db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                            .get::<_, i64>(0))
                            .unwrap(),
                        0
                    );
                }
            }
            outcomes.push((prepared.is_ok(), adopted));
        }
        assert_eq!(
            outcomes,
            vec![(false, false), (false, false)],
            "identical/conflicting embedded duplicates must not qualify or publish"
        );
        let (_root, s, b) = setup();
        let p = prepare(&s, &b, None).unwrap();
        let q = request(&b, &p);
        s.adopt_research_with_admission(
            &q,
            &producer(&s, &b.key, ProducerRole::Adopter),
            &mut |text| Ok(command(&q, text)),
        )
        .unwrap();
    }
    #[test]
    fn duplicate_retained_keys_refuse_in_prepare_and_saved_basis_adoption() {
        for first in [sha256(b"target"), sha256(b"conflicting earlier revision")] {
            let (_root, s, mut b) = setup();
            duplicate_result(&mut b, &first);
            let digest = research_digest("basis", &b).unwrap();
            let forged = VerifiedBasis {
                basis: b.clone(),
                digest,
            };
            assert!(
                s.prepare_research(
                    &b.key,
                    None,
                    forged,
                    &producer(&s, &b.key, ProducerRole::Preparer)
                )
                .is_err()
            );
            assert!(s.read_research_preparation(&b.key).unwrap().is_none());

            let (_root, s, mut b) = setup();
            let mut p = prepare(&s, &b, None).unwrap();
            duplicate_result(&mut b, &first);
            let digest = research_digest("basis", &b).unwrap();
            p.basis_digest = Some(digest.clone());
            // Corrupt the saved fixture consistently, so refusal must inspect retained bytes.
            {
                let mut db = s.lock().unwrap();
                let tx = db
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .unwrap();
                tx.execute("UPDATE research_basis SET basis_digest=?1,basis_json=?2 WHERE key_id=?3 AND revision=?4",
                    params![digest, serde_json::to_vec(&b).unwrap(), b.key.id().unwrap(), p.revision]).unwrap();
                save_head(&tx, &b.key, &p).unwrap();
                tx.commit().unwrap();
            }
            let q = request(&b, &p);
            assert!(
                s.adopt_research_with_admission(
                    &q,
                    &producer(&s, &b.key, ProducerRole::Adopter),
                    &mut |_| panic!(
                        "ambiguous retained evidence must refuse before command construction"
                    )
                )
                .is_err()
            );
            assert!(s.observe(&b.key.task_id).unwrap().is_none());
            assert!(
                s.lookup_admission_command(&b.submit.command_id)
                    .unwrap()
                    .is_none()
            );
            assert!(
                s.read_research_adoption(&q.submit.command_id, &q.payload_digest().unwrap())
                    .unwrap()
                    .is_none()
            );
        }
        // Typed top-level fields remain unique too; nested values cannot hide object maps.
        for bytes in [br#"{"dependencies":{},"dependencies":{},"text":"advice"}"#.as_slice(),
            br#"{"dependencies":{},"text":"first","text":"last"}"#,
            br#"{"dependencies":{"decision.target":{"revision":"first","revision":"last"}},"text":"advice"}"#] {
            assert!(CheckedRecommendation::decode(bytes).is_err());
        }
    }
    #[test]
    fn exact_basis_and_capability_scope_controls() {
        let (_root, s, b) = setup();
        let p = producer(&s, &b.key, ProducerRole::Preparer);
        let mut cases = Vec::new();
        let mut x = b.clone();
        x.key.request_digest = sha256(b"wrong");
        cases.push(x);
        let mut x = b.clone();
        x.snapshot_ref = "wrong".into();
        cases.push(x);
        let mut x = b.clone();
        x.recommendation.target_digest = sha256(b"wrong");
        cases.push(x);
        let mut x = b.clone();
        x.delivery.complete = false;
        cases.push(x);
        let mut x = b.clone();
        x.recommendation.delivery_digest = sha256(b"wrong");
        cases.push(x);
        let mut x = b.clone();
        x.artifacts[0].bytes.push(1);
        cases.push(x);
        let mut x = b.clone();
        x.artifacts.push(x.artifacts[0].clone());
        cases.push(x);
        let mut x = b.clone();
        x.version = 2;
        cases.push(x);
        for x in cases {
            assert!(s.controlled_verify_research_basis(x, &p).is_err());
            assert!(s.read_research_preparation(&b.key).unwrap().is_none());
        }
        assert!(matches!(
            s.controlled_verify_research_basis(
                b.clone(),
                &producer(&s, &b.key, ProducerRole::Adopter)
            ),
            Err(TaskError::Unauthorized)
        ));
        let other = tempfile::tempdir().unwrap();
        let other = TaskStore::open(&other.path().join("other.sqlite"), "human".into()).unwrap();
        assert!(matches!(
            other.controlled_verify_research_basis(b, &p),
            Err(TaskError::Unauthorized)
        ));
    }
    #[test]
    fn full_notice_capacity_commits_block_and_marker_before_acknowledgment() {
        let (_root, s, b) = setup();
        let p = prepare(&s, &b, None).unwrap();
        let q = request(&b, &p);
        let authority = producer(&s, &b.key, ProducerRole::DecisionNotice);
        for i in 0..1024 {
            let n = ChangeNotice {
                source_revision: sha256(i.to_string().as_bytes()),
                ..notice()
            };
            s.invalidate_research(&b.key, &format!("notice-{i}"), &n, &authority)
                .unwrap();
        }
        let marker = s
            .invalidate_research(&b.key, "overflow", &notice(), &authority)
            .unwrap();
        assert!(marker.capacity_blocked);
        assert_eq!(
            s.invalidate_research(&b.key, "overflow", &notice(), &authority)
                .unwrap(),
            marker
        );
        let head = s.read_research_preparation(&b.key).unwrap().unwrap();
        assert!(!head.ready);
        assert!(head.capacity_blocked);
        assert!(matches!(
            s.adopt_research_with_admission(
                &q,
                &producer(&s, &b.key, ProducerRole::Adopter),
                &mut |_| panic!("blocked")
            ),
            Err(TaskError::Stale)
        ));
        assert!(matches!(
            prepare(&s, &b, Some(head.revision)),
            Err(TaskError::Capacity)
        ));
        let db = s.lock().unwrap();
        let count: i64 = db
            .query_row("SELECT count(*) FROM research_notices", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1024);
        let marker_count: i64 = db
            .query_row("SELECT count(*) FROM research_capacity", [], |r| r.get(0))
            .unwrap();
        assert_eq!(marker_count, 1);
    }
    #[test]
    fn basis_limits_and_complete_query_refusal() {
        let (_root, s, b) = setup();
        let p = producer(&s, &b.key, ProducerRole::Preparer);
        let mut x = b.clone();
        x.artifacts[0].bytes = vec![
            1;
            MAX_ARTIFACT_BYTES
                - x.artifacts
                    .iter()
                    .skip(1)
                    .map(|a| a.bytes.len())
                    .sum::<usize>()
        ];
        x.artifacts[0].bytes[..4].copy_from_slice(b"card");
        x.artifacts[0].digest = sha256(&x.artifacts[0].bytes);
        assert!(s.controlled_verify_research_basis(x.clone(), &p).is_ok());
        x.artifacts[0].bytes.push(1);
        x.artifacts[0].digest = sha256(&x.artifacts[0].bytes);
        assert!(matches!(
            s.controlled_verify_research_basis(x, &p),
            Err(TaskError::Capacity)
        ));
        let db = s.lock().unwrap();
        db.execute_batch("CREATE TEMP TABLE complete_control(bytes BLOB);")
            .unwrap();
        assert!(
            bounded_rows::<ChangeNotice>(&db, "SELECT bytes FROM complete_control LIMIT 65", [])
                .unwrap()
                .is_empty()
        );
        for _ in 0..65 {
            db.execute(
                "INSERT INTO complete_control VALUES(?1)",
                [serde_json::to_vec(&notice()).unwrap()],
            )
            .unwrap();
        }
        assert!(matches!(
            bounded_rows::<ChangeNotice>(&db, "SELECT bytes FROM complete_control LIMIT 65", []),
            Err(TaskError::Capacity)
        ));
    }
    #[test]
    fn schema_token_boundaries_refuse_and_whitespace_layout_passes() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(&OWNER_SCHEMA_SQL.replace("schema_version = 2", "schema_version = 3"))
            .unwrap();
        db.execute_batch(&TABLES.replace("TEXT NOT NULL", "TEXT\n  NOT\tNULL"))
            .unwrap();
        validate_schema(&db).unwrap();
        db.execute_batch("DROP TABLE research_tasks; CREATE TABLE research_tasks(task_id TEXT PRIMARY KEY, subject_id TEXTNOTNULL);").unwrap();
        assert!(matches!(validate_schema(&db), Err(TaskError::Invalid)));
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("altered.sqlite");
        drop(TaskStore::open(&path, "human".into()).unwrap());
        let db = Connection::open(&path).unwrap();
        db.execute_batch("DROP TABLE research_tasks; CREATE TABLE research_tasks(task_id TEXT PRIMARY KEY, subject_id TEXTNOTNULL);").unwrap();
        drop(db);
        assert!(TaskStore::open(&path, "human".into()).is_err());
    }
    #[test]
    fn schema2_migration_preserves_bytes_and_interruption_rolls_back() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(OWNER_SCHEMA_SQL).unwrap();
        db.execute(
            "INSERT INTO store_identity VALUES(1,?1,2)",
            ["a".repeat(32)],
        )
        .unwrap();
        db.execute_batch("PRAGMA user_version=2;").unwrap();
        for (sql, bytes) in [
            (
                "INSERT INTO tasks VALUES('t','s',?1)",
                b"opaque\0task".as_slice(),
            ),
            (
                "INSERT INTO commands VALUES('c','submit','digest','t',?1)",
                b"opaque\0command".as_slice(),
            ),
            (
                "INSERT INTO operations VALUES('o','kind','digest','entry',?1)",
                b"opaque\0operation".as_slice(),
            ),
        ] {
            db.execute(sql, [bytes]).unwrap();
        }
        let old_schema = schema_objects(&db).unwrap();
        assert!(matches!(
            migrate_schema(&db, true),
            Err(TaskError::Unavailable)
        ));
        assert_eq!(schema_objects(&db).unwrap(), old_schema);
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        migrate_schema(&db, false).unwrap();
        assert_eq!(
            db.query_row("SELECT store_id FROM store_identity", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "a".repeat(32)
        );
        for (table, column, expected) in [
            ("tasks", "record_json", b"opaque\0task".as_slice()),
            ("commands", "commit_json", b"opaque\0command".as_slice()),
            ("operations", "commit_json", b"opaque\0operation".as_slice()),
        ] {
            assert_eq!(
                db.query_row(&format!("SELECT {column} FROM {table}"), [], |r| r
                    .get::<_, Vec<u8>>(0))
                    .unwrap(),
                expected
            );
        }
    }
    #[test]
    fn key_preparation_and_adoption_capacities_do_not_partially_publish() {
        let (_root, s, mut b) = setup();
        for i in 1..16 {
            let mut key = b.key.clone();
            key.request_digest = sha256(i.to_string().as_bytes());
            let p = producer(&s, &key, ProducerRole::Preparer);
            s.register_research_key(&key, &b.dependencies.keys().cloned().collect(), &p)
                .unwrap();
        }
        let mut key = b.key.clone();
        key.request_digest = sha256(b"seventeenth");
        assert!(matches!(
            s.register_research_key(
                &key,
                &b.dependencies.keys().cloned().collect(),
                &producer(&s, &key, ProducerRole::Preparer)
            ),
            Err(TaskError::Capacity)
        ));
        let mut expected = None;
        for i in 0..64 {
            b.submit.command_id = format!("research-command-{i}");
            let p = prepare(&s, &b, expected).unwrap();
            expected = Some(p.revision);
            let q = request(&b, &p);
            s.adopt_research_with_admission(
                &q,
                &producer(&s, &b.key, ProducerRole::Adopter),
                &mut |t| Ok(command(&q, t)),
            )
            .unwrap();
        }
        b.submit.command_id = "sixty-fifth".into();
        let p = prepare(&s, &b, expected).unwrap();
        let q = request(&b, &p);
        let old = s.observe("task-0").unwrap();
        assert!(matches!(
            s.adopt_research_with_admission(
                &q,
                &producer(&s, &b.key, ProducerRole::Adopter),
                &mut |_| panic!("capacity before builder")
            ),
            Err(TaskError::Capacity)
        ));
        assert_eq!(s.observe("task-0").unwrap(), old);
        assert!(s.lookup_admission_command("sixty-fifth").unwrap().is_none());
        expected = Some(p.revision);
        for _ in 65..256 {
            expected = Some(prepare(&s, &b, expected).unwrap().revision);
        }
        assert!(matches!(
            prepare(&s, &b, expected),
            Err(TaskError::Capacity)
        ));
        assert_eq!(
            s.read_research_preparation(&b.key)
                .unwrap()
                .unwrap()
                .revision,
            256
        );
    }
    #[test]
    fn concurrent_notice_and_adoption_share_owner_ordering() {
        for _ in 0..8 {
            let (_root, s, b) = setup();
            let prepared = prepare(&s, &b, None).unwrap();
            let q = request(&b, &prepared);
            let s = std::sync::Arc::new(s);
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
            let other = s.clone();
            let key = b.key.clone();
            let sync = barrier.clone();
            let worker = std::thread::spawn(move || {
                let p = producer(&other, &key, ProducerRole::DecisionNotice);
                sync.wait();
                other
                    .invalidate_research(&key, "race-notice", &notice(), &p)
                    .unwrap()
            });
            let p = producer(&s, &b.key, ProducerRole::Adopter);
            barrier.wait();
            let adopted = s.adopt_research_with_admission(&q, &p, &mut |t| Ok(command(&q, t)));
            worker.join().unwrap();
            match adopted {
                Ok((receipt, true)) => assert_eq!(
                    s.read_research_reconsideration(&b.key).unwrap()[0].adoption,
                    receipt
                ),
                Err(TaskError::Stale) => {
                    assert!(s.observe("task-0").unwrap().is_none());
                    assert!(s.lookup_admission_command("command-0").unwrap().is_none());
                }
                other => panic!("unexpected serialized outcome: {other:?}"),
            }
            assert!(!s.read_research_preparation(&b.key).unwrap().unwrap().ready);
        }
    }
    #[test]
    fn notice_role_scope_and_revision_overflow_fail_closed() {
        let (_root, s, b) = setup();
        let ready = prepare(&s, &b, None).unwrap();
        assert!(matches!(
            s.invalidate_research(
                &b.key,
                "forged-role",
                &notice(),
                &producer(&s, &b.key, ProducerRole::ResearchNotice)
            ),
            Err(TaskError::Unauthorized)
        ));
        let mut p = ready.clone();
        p.revision = i64::MAX as u64;
        {
            let db = s.lock().unwrap();
            db.execute(
                "UPDATE research_keys SET head_json=?1",
                [serde_json::to_vec(&p).unwrap()],
            )
            .unwrap();
        }
        let receipt = s
            .invalidate_research(
                &b.key,
                "overflow-revision",
                &notice(),
                &producer(&s, &b.key, ProducerRole::DecisionNotice),
            )
            .unwrap();
        assert!(receipt.capacity_blocked);
        let head = s.read_research_preparation(&b.key).unwrap().unwrap();
        assert!(!head.ready);
        assert!(head.capacity_blocked);
        assert!(matches!(
            prepare(&s, &b, Some(head.revision)),
            Err(TaskError::Capacity)
        ));
    }
    #[test]
    fn changed_pin_only_cannot_clear_notice_and_final_insert_failure_rolls_back() {
        let (_root, s, mut b) = setup();
        let first = prepare(&s, &b, None).unwrap();
        let n = notice();
        let notice = s
            .invalidate_research(
                &b.key,
                "changed",
                &n,
                &producer(&s, &b.key, ProducerRole::DecisionNotice),
            )
            .unwrap();
        b.dependencies
            .insert(n.selector.clone(), n.source_revision.clone());
        assert!(matches!(
            prepare(&s, &b, Some(notice.revision)),
            Err(TaskError::Stale)
        ));
        replace_target(&mut b, b"target-2");
        assert!(matches!(
            prepare(&s, &b, Some(notice.revision)),
            Err(TaskError::Stale)
        ));
        reassess(&mut b);
        let ready = prepare(&s, &b, Some(notice.revision)).unwrap();
        assert!(ready.revision > first.revision);
        let q = request(&b, &ready);
        {
            let db = s.lock().unwrap();
            db.execute_batch("CREATE TEMP TRIGGER fail_final_adoption BEFORE INSERT ON research_adoptions BEGIN SELECT RAISE(ABORT,'controlled final insert failure'); END;").unwrap();
        }
        assert!(matches!(
            s.adopt_research_with_admission(
                &q,
                &producer(&s, &b.key, ProducerRole::Adopter),
                &mut |t| Ok(command(&q, t))
            ),
            Err(TaskError::Storage(_))
        ));
        assert!(s.observe("task-0").unwrap().is_none());
        assert!(s.lookup_admission_command("command-0").unwrap().is_none());
        assert!(
            s.lookup_command_receipt(
                "research_adopt",
                "task-0",
                "command-0",
                &q.payload_digest().unwrap()
            )
            .unwrap()
            .is_none()
        );
        assert!(
            s.read_research_adoption("command-0", &q.payload_digest().unwrap())
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn exact_member_dependency_and_metadata_boundaries() {
        let (_root, s, b) = setup();
        let p = producer(&s, &b.key, ProducerRole::Preparer);
        let mut x = b.clone();
        while x.artifacts.len() < 128 {
            let i = x.artifacts.len();
            let bytes = b"member bytes";
            x.artifacts.push(RetainedArtifact {
                id: format!("member-{i:03}{}", "x".repeat(118)),
                digest: sha256(bytes),
                bytes: bytes.to_vec(),
            });
        }
        assert!(s.controlled_verify_research_basis(x.clone(), &p).is_ok());
        let mut over = x.clone();
        over.artifacts.push(RetainedArtifact {
            id: "member-over".into(),
            digest: sha256(b"over"),
            bytes: b"over".to_vec(),
        });
        assert!(s.controlled_verify_research_basis(over, &p).is_err());
        while x.dependencies.len() < 256 {
            let i = x.dependencies.len();
            let key = format!("decision.{i:03}{}", "x".repeat(116));
            x.dependencies.insert(key.clone(), sha256(b"target"));
            x.dependency_artifacts.insert(key, "target".into());
        }
        reassess(&mut x);
        assert!(s.controlled_verify_research_basis(x.clone(), &p).is_ok());
        let mut over = x.clone();
        over.dependencies
            .insert("decision.over".into(), sha256(b"over"));
        over.dependency_artifacts
            .insert("decision.over".into(), "target".into());
        assert!(s.controlled_verify_research_basis(over, &p).is_err());
        for i in 0..7 {
            x.submit.facts.push(FactInput {
                id: format!("metadata-{i}"),
                kind: FactKind::CanonicalRecord,
                text: "x".repeat(8192),
                current: true,
            });
        }
        let metadata_size = |b: &BasisBundle| {
            let mut m = b.clone();
            for a in &mut m.artifacts {
                a.bytes.clear();
            }
            serde_json::to_vec(&m).unwrap().len()
        };
        let mut excess = metadata_size(&x)
            .checked_sub(MAX_METADATA)
            .expect("fixture exceeds metadata bound");
        for fact in x.submit.facts.iter_mut().rev() {
            let remove = excess.min(fact.text.len() - 1);
            fact.text.truncate(fact.text.len() - remove);
            excess -= remove;
        }
        assert_eq!(excess, 0);
        assert_eq!(metadata_size(&x), MAX_METADATA);
        assert!(s.controlled_verify_research_basis(x.clone(), &p).is_ok());
        x.submit
            .facts
            .iter_mut()
            .find(|f| f.text.len() < 8192)
            .unwrap()
            .text
            .push('x');
        assert_eq!(metadata_size(&x), MAX_METADATA + 1);
        assert!(matches!(
            s.controlled_verify_research_basis(x, &p),
            Err(TaskError::Capacity)
        ));
        assert!(s.read_research_preparation(&b.key).unwrap().is_none());
    }
}
