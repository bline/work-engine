use crate::codec::{canonical, digest, raw_sha256};
use crate::contract::*;
use crate::{Error, Result, git, lock::Guard, validation};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

const DB: &str = "workspace-publication.sqlite";
const MARKER: &str = ".workspace-publication-v1";
const REPOSITORY_OWNER: &str = ".work-engine-publication-owner-v1";
type OperationRow = (
    String,
    Option<Vec<u8>>,
    String,
    i64,
    Option<i64>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
);
const SCHEMA: &str = r#"
CREATE TABLE identity (singleton INTEGER PRIMARY KEY CHECK(singleton=1), root_id TEXT NOT NULL, repository_id TEXT NOT NULL, policy_digest TEXT NOT NULL, config_digest TEXT NOT NULL);
CREATE TABLE records (record_id TEXT PRIMARY KEY, kind TEXT NOT NULL, sha256 TEXT NOT NULL, payload BLOB NOT NULL);
CREATE TABLE operations (operation_id TEXT PRIMARY KEY, request_id TEXT NOT NULL UNIQUE, request_digest TEXT NOT NULL, prepared_ref BLOB, state TEXT NOT NULL, revision INTEGER NOT NULL, generation INTEGER, admission_ref BLOB, attempt_ref BLOB, result_ref BLOB);
CREATE TABLE generation (singleton INTEGER PRIMARY KEY CHECK(singleton=1), value INTEGER NOT NULL);
INSERT INTO generation VALUES(1,0);
"#;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    root_id: String,
    repository_id: String,
    policy_digest: String,
    config_digest: String,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RepositoryOwner {
    root: PathBuf,
    root_id: String,
    repository_id: String,
}

fn repository_owner(
    common_dir: &Path,
    root: &Path,
    root_id: &str,
    repository_id: &str,
    install: bool,
) -> Result<()> {
    let expected = RepositoryOwner {
        root: fs::canonicalize(root).map_err(|e| Error::Io(e.to_string()))?,
        root_id: root_id.into(),
        repository_id: repository_id.into(),
    };
    let path = common_dir.join(REPOSITORY_OWNER);
    if let Ok(bytes) = fs::read(&path) {
        let saved: RepositoryOwner =
            serde_json::from_slice(&bytes).map_err(|e| Error::Integrity(e.to_string()))?;
        if saved != expected || bytes != canonical(&saved)? {
            return Err(Error::Conflict(
                "repository already bound to different workspace ledger".into(),
            ));
        }
        return Ok(());
    }
    if !install {
        return Err(Error::Integrity(
            "repository workspace owner binding absent".into(),
        ));
    }
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| Error::Io(e.to_string()))?;
    f.write_all(&canonical(&expected)?)
        .and_then(|_| f.sync_all())
        .map_err(|e| Error::Io(e.to_string()))?;
    File::open(common_dir)
        .and_then(|d| d.sync_all())
        .map_err(|e| Error::Io(e.to_string()))?;
    Ok(())
}

pub struct Workspace {
    config: WorkspaceConfig,
    root_id: String,
    repository_id: String,
    common_dir: PathBuf,
    conn: Connection,
}
fn random_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| Error::Io(e.to_string()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn sql<T>(v: rusqlite::Result<T>) -> Result<T> {
    v.map_err(|e| Error::Io(e.to_string()))
}
fn state_name(s: &OperationState) -> &'static str {
    match s {
        OperationState::Prepared => "prepared",
        OperationState::Admitted => "admitted",
        OperationState::AttemptStarted => "attempt_started",
        OperationState::ClosedNoEffect => "closed_no_effect",
        OperationState::PublishedObserved => "published_observed",
        OperationState::PublishedUnconfirmed => "published_unconfirmed",
        OperationState::Unresolved => "unresolved",
    }
}
fn parse_state(s: &str) -> Result<OperationState> {
    Ok(match s {
        "prepared" => OperationState::Prepared,
        "admitted" => OperationState::Admitted,
        "attempt_started" => OperationState::AttemptStarted,
        "closed_no_effect" => OperationState::ClosedNoEffect,
        "published_observed" => OperationState::PublishedObserved,
        "published_unconfirmed" => OperationState::PublishedUnconfirmed,
        "unresolved" => OperationState::Unresolved,
        _ => return Err(Error::Integrity("unknown operation state".into())),
    })
}
fn cfg_digest(config: &WorkspaceConfig, repository_id: &str) -> Result<String> {
    digest(
        "installed-config",
        &serde_json::json!({"repositoryId":repository_id,"refs":config.canonical_refs,
      "validator":config.validation_program,"policy":config.validation_policy_digest,"mode":config.authority_mode.label()}),
    )
}
impl Workspace {
    pub fn initialize(config: WorkspaceConfig) -> Result<Self> {
        if !config.root.is_absolute()
            || !config.repository.is_absolute()
            || config.canonical_refs.is_empty()
            || config.validation_program.is_empty()
        {
            return Err(Error::Invalid(
                "absolute roots, refs and validator required".into(),
            ));
        }
        let repository_id = git::repository_id(&config.repository)?;
        let common_dir = git::common_dir(&config.repository)?;
        fs::create_dir(&config.root).map_err(|e| Error::Io(e.to_string()))?;
        fs::set_permissions(&config.root, fs::Permissions::from_mode(0o700))
            .map_err(|e| Error::Io(e.to_string()))?;
        let _guard = Guard::acquire(&common_dir)?;
        if common_dir.join(REPOSITORY_OWNER).exists() {
            return Err(Error::Conflict(
                "repository already has workspace owner".into(),
            ));
        }
        let marker = Marker {
            root_id: random_id()?,
            repository_id: repository_id.clone(),
            policy_digest: config.validation_policy_digest.clone(),
            config_digest: cfg_digest(&config, &repository_id)?,
        };
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(config.root.join(MARKER))
            .map_err(|e| Error::Io(e.to_string()))?;
        f.write_all(&canonical(&marker)?)
            .and_then(|_| f.sync_all())
            .map_err(|e| Error::Io(e.to_string()))?;
        let conn = sql(Connection::open(config.root.join(DB)))?;
        fs::set_permissions(config.root.join(DB), fs::Permissions::from_mode(0o600))
            .map_err(|e| Error::Io(e.to_string()))?;
        configure(&conn)?;
        sql(conn.execute_batch(SCHEMA))?;
        sql(conn.execute(
            "INSERT INTO identity VALUES(1,?1,?2,?3,?4)",
            params![
                marker.root_id,
                marker.repository_id,
                marker.policy_digest,
                marker.config_digest
            ],
        ))?;
        repository_owner(
            &common_dir,
            &config.root,
            &marker.root_id,
            &repository_id,
            true,
        )?;
        Ok(Self {
            config,
            root_id: marker.root_id,
            repository_id,
            common_dir,
            conn,
        })
    }
    pub fn open(config: WorkspaceConfig) -> Result<Self> {
        let bytes = fs::read(config.root.join(MARKER)).map_err(|e| Error::Io(e.to_string()))?;
        let marker: Marker =
            serde_json::from_slice(&bytes).map_err(|e| Error::Integrity(e.to_string()))?;
        let repository_id = git::repository_id(&config.repository)?;
        if marker.repository_id != repository_id
            || marker.policy_digest != config.validation_policy_digest
            || marker.config_digest != cfg_digest(&config, &repository_id)?
        {
            return Err(Error::Conflict("installed workspace config changed".into()));
        }
        let conn = sql(Connection::open(config.root.join(DB)))?;
        configure(&conn)?;
        let saved:(String,String,String,String)=sql(conn.query_row("SELECT root_id,repository_id,policy_digest,config_digest FROM identity WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))))?;
        if saved
            != (
                marker.root_id.clone(),
                marker.repository_id.clone(),
                marker.policy_digest.clone(),
                marker.config_digest.clone(),
            )
        {
            return Err(Error::Integrity("workspace identity differs".into()));
        }
        let common_dir = git::common_dir(&config.repository)?;
        let _guard = Guard::acquire(&common_dir)?;
        repository_owner(
            &common_dir,
            &config.root,
            &marker.root_id,
            &repository_id,
            false,
        )?;
        Ok(Self {
            config,
            root_id: marker.root_id,
            repository_id,
            common_dir,
            conn,
        })
    }
    pub fn root_id(&self) -> &str {
        &self.root_id
    }
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }
    fn guard(&self) -> Result<Guard> {
        let guard = Guard::acquire(&self.common_dir)?;
        repository_owner(
            &self.common_dir,
            &self.config.root,
            &self.root_id,
            &self.repository_id,
            false,
        )?;
        Ok(guard)
    }
    /// Fixture-only enrolled checkout actor. It shares the Git common-directory
    /// guard and refuses unresolved prior admission before changing topology.
    pub fn managed_checkout_fixture(&self, target: &str, path: &Path) -> Result<()> {
        if self.config.authority_mode != AuthorityMode::FixtureOperator
            || !self.config.canonical_refs.iter().any(|r| r == target)
        {
            return Err(Error::Invalid(
                "managed checkout outside fixture/allowlist".into(),
            ));
        }
        let namespace = self.config.root.join("managed-checkouts");
        if path.parent() != Some(namespace.as_path()) || path.exists() {
            return Err(Error::Invalid(
                "checkout path outside operation namespace".into(),
            ));
        }
        let guard = self.guard()?;
        let active:i64=sql(self.conn.query_row("SELECT COUNT(*) FROM operations WHERE state IN ('admitted','attempt_started','unresolved','published_unconfirmed')",[],|r|r.get(0)))?;
        if active != 0 {
            return Err(Error::Unresolved(
                "active or unresolved publication excludes checkout".into(),
            ));
        }
        fs::create_dir_all(&namespace).map_err(|e| Error::Io(e.to_string()))?;
        git::command_guarded(
            &self.config.repository,
            &[
                "worktree",
                "add",
                path.to_str()
                    .ok_or_else(|| Error::Invalid("checkout path encoding".into()))?,
                target,
            ],
            None,
            guard.raw_fd(),
        )?;
        Ok(())
    }
    pub fn checkpoint_parent(&self, commit: &str) -> Result<String> {
        Ok(git::commit_parent(
            &self.config.repository,
            &GitOid {
                algorithm: "sha1".into(),
                hex: commit.into(),
            },
        )?
        .hex)
    }
    pub fn inspect_candidate(&self, candidate: &PublicationCandidate) -> Result<String> {
        git::verify_candidate(&self.config.repository, candidate)?;
        if candidate.review_outcomes.is_empty() || candidate.gate_receipts.is_empty() {
            return Err(Error::Conflict("review or gate evidence missing".into()));
        }
        digest("publication-candidate", candidate)
    }
    fn put_record<T: Serialize>(&self, kind: &str, id: &str, payload: &T) -> Result<RecordRef> {
        let bytes = canonical(payload)?;
        let sha = digest(kind, payload)?;
        let existing: Option<(String, String, Vec<u8>)> = sql(self
            .conn
            .query_row(
                "SELECT kind,sha256,payload FROM records WHERE record_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional())?;
        if let Some((k, s, b)) = existing {
            if k != kind || s != sha || b != bytes {
                return Err(Error::Conflict(
                    "record ID reused with different bytes".into(),
                ));
            }
        } else {
            sql(self.conn.execute(
                "INSERT INTO records VALUES(?1,?2,?3,?4)",
                params![id, kind, sha, bytes],
            ))?;
        }
        Ok(RecordRef {
            kind: kind.into(),
            schema_version: 1,
            root_id: self.root_id.clone(),
            record_id: id.into(),
            sha256: sha,
        })
    }
    pub fn read_record<T: DeserializeOwned + Serialize>(&self, r: &RecordRef) -> Result<T> {
        if r.schema_version != 1 || r.root_id != self.root_id {
            return Err(Error::Conflict("record owner/version".into()));
        }
        let row: Option<(String, String, Vec<u8>)> = sql(self
            .conn
            .query_row(
                "SELECT kind,sha256,payload FROM records WHERE record_id=?1",
                [&r.record_id],
                |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?)),
            )
            .optional())?;
        let (kind, sha, bytes) = row.ok_or_else(|| Error::Unresolved("record absent".into()))?;
        if kind != r.kind || sha != r.sha256 {
            return Err(Error::Integrity("record reference differs".into()));
        }
        let value: T =
            serde_json::from_slice(&bytes).map_err(|e| Error::Integrity(e.to_string()))?;
        if canonical(&value)? != bytes || digest(&kind, &value)? != sha {
            return Err(Error::Integrity("record bytes/digest differ".into()));
        }
        Ok(value)
    }
    pub fn inspect(&self, id: &str) -> Result<WorkspaceOperation> {
        let row:OperationRow=sql(self.conn.query_row("SELECT operation_id,prepared_ref,state,revision,generation,admission_ref,attempt_ref,result_ref FROM operations WHERE operation_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))))?;
        let read_ref = |b: Option<Vec<u8>>| -> Result<Option<RecordRef>> {
            b.map(|v| serde_json::from_slice(&v).map_err(|e| Error::Integrity(e.to_string())))
                .transpose()
        };
        Ok(WorkspaceOperation {
            operation_id: row.0,
            prepared_ref: read_ref(row.1)?,
            state: parse_state(&row.2)?,
            revision: row.3 as u64,
            generation: row.4.map(|n| n as u64),
            admission_ref: read_ref(row.5)?,
            attempt_ref: read_ref(row.6)?,
            result_ref: read_ref(row.7)?,
        })
    }
    pub fn prepare<A: AuthorityReadback>(
        &mut self,
        authority: &A,
        eligibility_ref: &RecordRef,
        target: &str,
        message: &str,
        request_id: &str,
    ) -> Result<(RecordRef, PreparedPublication)> {
        if !self.config.canonical_refs.iter().any(|r| r == target)
            || message.trim().is_empty()
            || request_id.trim().is_empty()
        {
            return Err(Error::Invalid("target/message/request ID".into()));
        }
        let eligibility = authority.check_eligibility(eligibility_ref)?;
        if !eligibility.active
            || eligibility.scope != "publication-only"
            || eligibility.acceptance_ref.root_id != eligibility_ref.root_id
        {
            return Err(Error::Conflict("eligibility inactive or unbound".into()));
        }
        let candidate = authority.read_candidate(&eligibility.candidate_digest)?;
        if self.inspect_candidate(&candidate)? != eligibility.candidate_digest {
            return Err(Error::Conflict("candidate digest changed".into()));
        }
        let request_digest = digest(
            "preparation-request",
            &serde_json::json!({"eligibility":eligibility_ref,"target":target,"message":message,"requestId":request_id}),
        )?;
        let _guard = self.guard()?;
        let existing: Option<(String, String)> = sql(self
            .conn
            .query_row(
                "SELECT operation_id,request_digest FROM operations WHERE request_id=?1",
                [request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional())?;
        if let Some((id, prior)) = existing {
            if prior != request_digest {
                return Err(Error::Conflict("preparation request ID reused".into()));
            }
            let op = self.inspect(&id)?;
            let r = op
                .prepared_ref
                .ok_or_else(|| Error::Unresolved("preparation allocated but incomplete".into()))?;
            let p = self.read_record(&r)?;
            return Ok((r, p));
        }
        let operation_id =
            raw_sha256(format!("{}:{request_id}:{request_digest}", self.root_id).as_bytes())[..32]
                .to_string();
        sql(self.conn.execute("INSERT INTO operations(operation_id,request_id,request_digest,state,revision) VALUES(?1,?2,?3,'prepared',1)",params![operation_id,request_id,request_digest]))?;
        let expected = git::ref_oid(&self.config.repository, target)?;
        let artifact = self.config.root.join("operations").join(&operation_id);
        fs::create_dir_all(artifact.parent().unwrap()).map_err(|e| Error::Io(e.to_string()))?;
        let tree = git::integrate(&self.config.repository, &artifact, &expected, &candidate)?;
        let expected_tree = git::commit_tree(&self.config.repository, &expected)?;
        git::verify_manifest(
            &self.config.repository,
            &expected_tree,
            &tree,
            &candidate.manifest,
        )?;
        let manifest_digest = digest("attributed-manifest", &candidate.manifest)?;
        let validation = validation::run(
            &self.config,
            &artifact,
            &tree,
            &self.root_id,
            &format!("validation-{operation_id}"),
        )?;
        let validation_ref =
            self.put_record("tree-validation", &validation.record_id, &validation)?;
        let preparation_digest = digest(
            "preparation-binding",
            &serde_json::json!({"profile":PROFILE,"version":1,"rootId":self.root_id,"operationId":operation_id,
          "eligibility":eligibility_ref,"repositoryId":self.repository_id,"canonicalRef":target,"expectedParent":expected,
          "manifestDigest":manifest_digest,"integratedTree":tree,"validationPolicyDigest":self.config.validation_policy_digest,
          "treeValidation":validation_ref,"message":message}),
        )?;
        let full_message = format!(
            "{message}\n\nWork-Engine-Operation: {operation_id}\nWork-Engine-Preparation: {preparation_digest}\n"
        );
        let commit = git::create_commit(&self.config.repository, &tree, &expected, &full_message)?;
        let prepared = PreparedPublication {
            operation_id: operation_id.clone(),
            preparation_digest,
            eligibility_ref: eligibility_ref.clone(),
            repository_id: self.repository_id.clone(),
            canonical_ref: target.into(),
            expected_parent: expected,
            manifest_digest,
            integrated_tree: tree,
            prepared_commit: commit,
            proposed_message: message.into(),
            message: full_message,
            validation_policy_digest: self.config.validation_policy_digest.clone(),
            validation_ref,
            artifact_identity: raw_sha256(artifact.to_string_lossy().as_bytes()),
        };
        let prepared_ref = self.put_record(
            "prepared-publication",
            &format!("prepared-{operation_id}"),
            &prepared,
        )?;
        sql(self.conn.execute("UPDATE operations SET prepared_ref=?2,revision=revision+1 WHERE operation_id=?1 AND prepared_ref IS NULL",params![operation_id,canonical(&prepared_ref)?]))?;
        Ok((prepared_ref, prepared))
    }
    fn update_state(
        &mut self,
        op: &WorkspaceOperation,
        next: OperationState,
        generation: Option<u64>,
        admission: Option<&RecordRef>,
        attempt: Option<&RecordRef>,
        result: Option<&RecordRef>,
    ) -> Result<()> {
        let n=sql(self.conn.execute("UPDATE operations SET state=?2,revision=revision+1,generation=COALESCE(?3,generation),admission_ref=COALESCE(?4,admission_ref),attempt_ref=COALESCE(?5,attempt_ref),result_ref=COALESCE(?6,result_ref) WHERE operation_id=?1 AND revision=?7 AND state=?8",
            params![op.operation_id,state_name(&next),generation.map(|x|x as i64),admission.map(canonical).transpose()?,attempt.map(canonical).transpose()?,result.map(canonical).transpose()?,op.revision as i64,state_name(&op.state)]))?;
        if n != 1 {
            return Err(Error::Conflict("operation CAS lost".into()));
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn receipt(
        &self,
        p: &PreparedPublication,
        request_digest: String,
        state: OperationState,
        generation: Option<u64>,
        admission: Option<RecordRef>,
        admission_digest: Option<String>,
        closure: Option<RecordRef>,
        observed: Option<GitOid>,
        limitation: &str,
    ) -> PublicationReceipt {
        PublicationReceipt {
            operation_id: p.operation_id.clone(),
            request_digest,
            preparation_digest: p.preparation_digest.clone(),
            admission_digest,
            resources: vec![
                p.canonical_ref.clone(),
                self.repository_id.clone(),
                p.artifact_identity.clone(),
            ],
            generation,
            admission_ref: admission,
            closure_ref: closure,
            commit: p.prepared_commit.clone(),
            parent: p.expected_parent.clone(),
            tree: p.integrated_tree.clone(),
            message: p.message.clone(),
            observed_target: observed,
            state,
            limitation: limitation.into(),
            recovery_locator: p.operation_id.clone(),
            authority_mode: self.config.authority_mode.label().into(),
        }
    }
    fn verify_prepared(
        &self,
        op: &WorkspaceOperation,
        request: &PublicationRequest,
    ) -> Result<PreparedPublication> {
        if op.operation_id != request.operation_id
            || op.prepared_ref.as_ref() != Some(&request.prepared_ref)
        {
            return Err(Error::Conflict("operation/preparation differs".into()));
        }
        let p: PreparedPublication = self.read_record(&request.prepared_ref)?;
        if p.operation_id != op.operation_id
            || p.eligibility_ref != request.eligibility_ref
            || p.repository_id != self.repository_id
            || p.validation_policy_digest != self.config.validation_policy_digest
        {
            return Err(Error::Conflict("prepared authority/policy differs".into()));
        }
        let binding = digest(
            "preparation-binding",
            &serde_json::json!({"profile":PROFILE,"version":1,"rootId":self.root_id,"operationId":p.operation_id,
          "eligibility":p.eligibility_ref,"repositoryId":self.repository_id,"canonicalRef":p.canonical_ref,"expectedParent":p.expected_parent,
          "manifestDigest":p.manifest_digest,"integratedTree":p.integrated_tree,"validationPolicyDigest":p.validation_policy_digest,
          "treeValidation":p.validation_ref,"message":p.proposed_message}),
        )?;
        let full = format!(
            "{}\n\nWork-Engine-Operation: {}\nWork-Engine-Preparation: {}\n",
            p.proposed_message, p.operation_id, binding
        );
        if p.preparation_digest != binding || p.message != full {
            return Err(Error::Integrity(
                "acyclic preparation binding differs".into(),
            ));
        }
        let v: ValidationReceipt = self.read_record(&p.validation_ref)?;
        if v.exit_code != 0
            || v.tree != p.integrated_tree
            || v.policy_digest != p.validation_policy_digest
            || git::commit_tree(&self.config.repository, &p.prepared_commit)? != p.integrated_tree
            || git::commit_parent(&self.config.repository, &p.prepared_commit)? != p.expected_parent
        {
            return Err(Error::Integrity(
                "prepared Git/validation binding differs".into(),
            ));
        }
        if git::commit_message(&self.config.repository, &p.prepared_commit)? != p.message.as_bytes()
        {
            return Err(Error::Integrity("prepared message differs".into()));
        }
        Ok(p)
    }
    pub fn close_no_effect<A: AuthorityReadback>(
        &mut self,
        authority: &A,
        request: &PublicationRequest,
    ) -> Result<PublicationReceipt> {
        let _guard = self.guard()?;
        let create = authority.check_create_consumption(request)?;
        let op = self.inspect(&request.operation_id)?;
        if op.prepared_ref.as_ref() != Some(&request.prepared_ref) {
            return Err(Error::Conflict("closure preparation differs".into()));
        }
        if op.state == OperationState::ClosedNoEffect {
            return self.read_record(
                op.result_ref
                    .as_ref()
                    .ok_or_else(|| Error::Integrity("closure receipt absent".into()))?,
            );
        }
        let p = self.verify_prepared(&op, request)?;
        if !matches!(
            op.state,
            OperationState::Prepared | OperationState::Admitted
        ) {
            return Err(Error::Unresolved(
                "effect may have started; no-effect closure refused".into(),
            ));
        }
        let closure=self.put_record("no-effect-closure",&format!("closure-{}",p.operation_id),&serde_json::json!({"operationId":p.operation_id,"preparation":request.prepared_ref,"consumption":create.consumption_ref,"state":state_name(&op.state)}))?;
        let receipt = self.receipt(
            &p,
            digest("publication-request", request)?,
            OperationState::ClosedNoEffect,
            op.generation,
            op.admission_ref.clone(),
            None,
            Some(closure),
            git::ref_oid(&self.config.repository, &p.canonical_ref).ok(),
            "guarded closure before physical attempt",
        );
        let r = self.put_record(
            "publication-receipt",
            &format!("result-{}", p.operation_id),
            &receipt,
        )?;
        self.update_state(
            &op,
            OperationState::ClosedNoEffect,
            None,
            None,
            None,
            Some(&r),
        )?;
        Ok(receipt)
    }
    pub fn publish<A: AuthorityReadback>(
        &mut self,
        authority: &A,
        request: &PublicationRequest,
    ) -> Result<PublicationReceipt> {
        if self.config.authority_mode == AuthorityMode::Production {
            return Err(Error::Conflict(
                "production repository custody is not installed".into(),
            ));
        }
        // This read may race closure. Admission rechecks the tombstone after taking the same OS guard.
        authority.check_create_consumption(request)?;
        #[cfg(feature = "test-faults")]
        crate::fault_cut("before_admission");
        let _guard = self.guard()?;
        let create = authority.check_create_consumption(request)?;
        let op = self.inspect(&request.operation_id)?;
        if op.prepared_ref.as_ref() != Some(&request.prepared_ref) {
            return Err(Error::Conflict("publication preparation differs".into()));
        }
        if op.state == OperationState::ClosedNoEffect {
            return self.read_record(
                op.result_ref
                    .as_ref()
                    .ok_or_else(|| Error::Integrity("closure receipt absent".into()))?,
            );
        }
        let eligibility = authority.check_eligibility(&request.eligibility_ref)?;
        if !eligibility.active {
            return Err(Error::Conflict("eligibility revoked".into()));
        }
        if op.state != OperationState::Prepared {
            return Err(Error::Unresolved(format!(
                "operation already {:?}; reconcile only",
                op.state
            )));
        }
        let p = self.verify_prepared(&op, request)?;
        let candidate = authority.read_candidate(&eligibility.candidate_digest)?;
        if self.inspect_candidate(&candidate)? != eligibility.candidate_digest
            || digest("attributed-manifest", &candidate.manifest)? != p.manifest_digest
        {
            return Err(Error::Conflict("accepted manifest changed".into()));
        }
        let other_active:i64=sql(self.conn.query_row("SELECT COUNT(*) FROM operations WHERE operation_id<>?1 AND state IN ('admitted','attempt_started','unresolved','published_unconfirmed')",[&p.operation_id],|r|r.get(0)))?;
        if other_active != 0 {
            return Err(Error::Unresolved(
                "prior admitted operation requires reconciliation".into(),
            ));
        }
        if !self.config.canonical_refs.contains(&p.canonical_ref) {
            return Err(Error::Conflict("canonical ref not installed".into()));
        }
        let current = git::ref_oid(&self.config.repository, &p.canonical_ref)?;
        if current != p.expected_parent || checked_out(&self.config.repository, &p.canonical_ref)? {
            return self.close_under_guard(
                &op,
                &p,
                request,
                &create.consumption_ref,
                "expected parent changed or target checked out",
            );
        }
        let tx = sql(self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate))?;
        let previous: i64 = sql(tx.query_row(
            "SELECT value FROM generation WHERE singleton=1",
            [],
            |r| r.get::<_, i64>(0),
        ))?;
        let generation = previous + 1;
        sql(tx.execute(
            "UPDATE generation SET value=?1 WHERE singleton=1",
            [generation],
        ))?;
        sql(tx.commit())?;
        let admission_digest = digest(
            "admission-binding",
            &serde_json::json!({"operationId":p.operation_id,"preparationDigest":p.preparation_digest,"prepared":request.prepared_ref,
          "offer":request.offer_ref,"createDecision":create.decision_ref,"campaignConsumption":create.consumption_ref,
          "resources":[p.canonical_ref,self.repository_id,p.artifact_identity],"generation":generation}),
        )?;
        let admission=self.put_record("admission-binding",&format!("admission-{}",p.operation_id),&serde_json::json!({"digest":admission_digest,"request":request,"decision":create.decision_ref,"consumption":create.consumption_ref,"generation":generation}))?;
        self.update_state(
            &op,
            OperationState::Admitted,
            Some(generation as u64),
            Some(&admission),
            None,
            None,
        )?;
        #[cfg(feature = "test-faults")]
        crate::fault_cut("after_admission");
        let admitted = self.inspect(&p.operation_id)?;
        // Only this guarded call can make AttemptStarted. A resumed call sees Admitted and stops.
        if git::ref_oid(&self.config.repository, &p.canonical_ref)? != p.expected_parent {
            return Err(Error::Unresolved(
                "parent changed after admission; close via reconciliation".into(),
            ));
        }
        let attempt=self.put_record("attempt-started",&format!("attempt-{}",p.operation_id),&serde_json::json!({"admission":admission,"request":request,"holderPid":std::process::id(),"generation":generation}))?;
        self.update_state(
            &admitted,
            OperationState::AttemptStarted,
            None,
            None,
            Some(&attempt),
            None,
        )?;
        #[cfg(feature = "test-faults")]
        crate::fault_cut("after_attempt_started");
        let effect = git::compare_and_swap(
            &self.config.repository,
            &p.canonical_ref,
            &p.expected_parent,
            &p.prepared_commit,
        );
        #[cfg(feature = "test-faults")]
        crate::fault_cut("after_ref_cas");
        if let Err(effect) = effect {
            return Err(Error::Unresolved(format!(
                "physical ref attempt uncertain: {}",
                effect
            )));
        }
        #[cfg(feature = "test-faults")]
        crate::fault_cut("before_result");
        self.reconcile_locked(authority, request)
    }
    fn close_under_guard(
        &mut self,
        op: &WorkspaceOperation,
        p: &PreparedPublication,
        request: &PublicationRequest,
        consumption: &RecordRef,
        reason: &str,
    ) -> Result<PublicationReceipt> {
        let closure=self.put_record("no-effect-closure",&format!("closure-{}",p.operation_id),&serde_json::json!({"operationId":p.operation_id,"preparation":request.prepared_ref,"consumption":consumption,"reason":reason}))?;
        let receipt = self.receipt(
            p,
            digest("publication-request", request)?,
            OperationState::ClosedNoEffect,
            None,
            None,
            None,
            Some(closure),
            git::ref_oid(&self.config.repository, &p.canonical_ref).ok(),
            reason,
        );
        let rr = self.put_record(
            "publication-receipt",
            &format!("result-{}", p.operation_id),
            &receipt,
        )?;
        self.update_state(
            op,
            OperationState::ClosedNoEffect,
            None,
            None,
            None,
            Some(&rr),
        )?;
        Ok(receipt)
    }
    pub fn reconcile<A: AuthorityReadback>(
        &mut self,
        authority: &A,
        request: &PublicationRequest,
    ) -> Result<PublicationReceipt> {
        let _guard = self.guard()?;
        self.reconcile_locked(authority, request)
    }
    fn reconcile_locked<A: AuthorityReadback>(
        &mut self,
        authority: &A,
        request: &PublicationRequest,
    ) -> Result<PublicationReceipt> {
        let op = self.inspect(&request.operation_id)?;
        if let Some(r) = &op.result_ref {
            return self.read_record(r);
        }
        let p = self.verify_prepared(&op, request)?;
        if op.state == OperationState::Prepared || op.state == OperationState::Admitted {
            return Err(Error::Unresolved(
                "effect not started; closure required before NoEffect".into(),
            ));
        }
        let current = git::ref_oid(&self.config.repository, &p.canonical_ref)?;
        let checked = authority.check_create_consumption(request);
        let admission_record = op
            .admission_ref
            .as_ref()
            .and_then(|r| self.read_record::<serde_json::Value>(r).ok());
        let admission_digest = admission_record
            .as_ref()
            .and_then(|v| v["digest"].as_str())
            .map(str::to_string);
        let admission_ok = match (&checked, &admission_record, op.generation) {
            (Ok(create), Some(saved), Some(generation)) => {
                let expected = digest(
                    "admission-binding",
                    &serde_json::json!({"operationId":p.operation_id,"preparationDigest":p.preparation_digest,"prepared":request.prepared_ref,
                  "offer":request.offer_ref,"createDecision":create.decision_ref,"campaignConsumption":create.consumption_ref,
                  "resources":[p.canonical_ref,self.repository_id,p.artifact_identity],"generation":generation}),
                )?;
                saved["digest"] == expected
                    && saved["request"]
                        == serde_json::to_value(request)
                            .map_err(|e| Error::Integrity(e.to_string()))?
                    && saved["decision"]
                        == serde_json::to_value(&create.decision_ref)
                            .map_err(|e| Error::Integrity(e.to_string()))?
                    && saved["consumption"]
                        == serde_json::to_value(&create.consumption_ref)
                            .map_err(|e| Error::Integrity(e.to_string()))?
            }
            _ => false,
        };
        let attempt_ok = op
            .attempt_ref
            .as_ref()
            .is_some_and(|r| self.read_record::<serde_json::Value>(r).is_ok());
        let state = if current == p.prepared_commit && admission_ok && attempt_ok {
            OperationState::PublishedObserved
        } else if current == p.prepared_commit {
            OperationState::PublishedUnconfirmed
        } else {
            OperationState::Unresolved
        };
        let receipt = self.receipt(
            &p,
            digest("publication-request", request)?,
            state.clone(),
            op.generation,
            op.admission_ref.clone(),
            admission_digest,
            None,
            Some(current),
            if state == OperationState::PublishedObserved {
                "observed under enrolled fixture custody only"
            } else {
                "historical effect uncertain; no retry"
            },
        );
        let rr = self.put_record(
            "publication-receipt",
            &format!("result-{}", p.operation_id),
            &receipt,
        )?;
        self.update_state(&op, state, None, None, None, Some(&rr))?;
        Ok(receipt)
    }
}
fn checked_out(repo: &Path, target: &str) -> Result<bool> {
    let bytes = git::command(repo, &["worktree", "list", "--porcelain"], None)?;
    Ok(String::from_utf8_lossy(&bytes)
        .lines()
        .any(|line| line == format!("branch {target}")))
}
fn configure(conn: &Connection) -> Result<()> {
    sql(conn.busy_timeout(Duration::from_secs(30)))?;
    sql(conn
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;"))
}
