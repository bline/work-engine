//! Typed research seam. Controlled fixtures establish local mechanics only.
use super::*;
use retained_task_workflow::research_admission::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchNomination {
    pub task_id: String,
    pub subject_id: String,
    /// Explicit qualification input, never a production installation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controlled_fixture: Option<PathBuf>,
}
#[derive(Default)]
pub(crate) struct ResearchHost {
    #[cfg(feature = "controlled-proof")]
    fixture: Option<ControlledFixture>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ControlledFixture {
    pub version: u32,
    pub key: ResearchKey,
    pub selectors: BTreeSet<String>,
    pub bases: Vec<BasisBundle>,
    pub notices: Vec<(String, ChangeNotice)>,
}
#[cfg(feature = "controlled-proof")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareRequest {
    key: ResearchKey,
    expected_revision: Option<u64>,
    basis_digest: String,
}
#[cfg(feature = "controlled-proof")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoticeRequest {
    key: ResearchKey,
    notice_id: String,
    notice: ChangeNotice,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdoptionRead {
    command_id: String,
    payload_digest: String,
}

pub(crate) fn validate_nomination(startup: &HostStartup) -> Result<(), HostError> {
    if let Some(n) = &startup.research {
        let id = |s: &str| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        };
        if !id(&n.task_id)
            || !id(&n.subject_id)
            || n.subject_id != startup.lifecycle.subject_id
            || (!cfg!(feature = "controlled-proof") && n.controlled_fixture.is_some())
        {
            return Err(HostError::Configuration);
        }
    }
    Ok(())
}

pub(crate) fn install(startup: &HostStartup, store: &TaskStore) -> Result<ResearchHost, HostError> {
    let Some(nomination) = &startup.research else {
        return Ok(ResearchHost::default());
    };
    if nomination.subject_id != startup.lifecycle.subject_id {
        return Err(HostError::Configuration);
    }
    // Validate every fixture before even publishing the nomination guard.
    #[cfg(not(feature = "controlled-proof"))]
    if nomination.controlled_fixture.is_some() {
        return Err(HostError::Configuration);
    }
    #[cfg(feature = "controlled-proof")]
    let fixture = if let Some(path) = &nomination.controlled_fixture {
        validate_file(path)?;
        if fs::metadata(path)?.len() > 32 * 1024 * 1024 {
            return Err(HostError::Configuration);
        }
        let value = lifecycle_wire::parse_strict_json(&fs::read(path)?)
            .map_err(|_| HostError::Configuration)?;
        let f: ControlledFixture = serde_json::from_value(value)?;
        if f.version != 1
            || f.key.task_id != nomination.task_id
            || f.key.subject_id != nomination.subject_id
            || f.bases.is_empty()
            || f.bases.len() > 16
            || f.notices.len() > 64
            || f.selectors.is_empty()
            || f.selectors.len() > 256
            || f.selectors.iter().any(|s| {
                s.is_empty()
                    || s.len() > 128
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
                    || !(s.starts_with("research.") || s.starts_with("decision."))
            })
        {
            return Err(HostError::Configuration);
        }
        let mut notice_ids = BTreeSet::new();
        for (id, notice) in &f.notices {
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
                || !notice_ids.insert(id)
                || !f.selectors.contains(&notice.selector)
                || !valid_sha(&notice.source_revision)
                || notice.reason.is_empty()
                || notice.reason.len() > 8192
                || notice.reason.contains('\0')
            {
                return Err(HostError::Configuration);
            }
        }
        let preparer = store.controlled_research_producer(&f.key, ProducerRole::Preparer)?;
        for basis in &f.bases {
            if basis.key != f.key
                || basis.snapshot_ref != startup.approved_snapshot_ref
                || basis.dependencies.keys().cloned().collect::<BTreeSet<_>>() != f.selectors
            {
                return Err(HostError::Configuration);
            }
            store.controlled_verify_research_basis(basis.clone(), &preparer)?;
        }
        Some(f)
    } else {
        None
    };
    store.nominate_research_task(&nomination.task_id, &nomination.subject_id)?;
    #[cfg(feature = "controlled-proof")]
    if let Some(f) = &fixture {
        let p = store.controlled_research_producer(&f.key, ProducerRole::Preparer)?;
        store.register_research_key(&f.key, &f.selectors, &p)?;
    }
    Ok(ResearchHost {
        #[cfg(feature = "controlled-proof")]
        fixture,
    })
}
pub(crate) fn guard_submit(host: &Host, input: &SubmitTask) -> Result<(), HostError> {
    if host
        .startup
        .research
        .as_ref()
        .is_some_and(|n| n.task_id == input.task_id)
    {
        return Err(HostError::Task(TaskError::Unauthorized));
    }
    Ok(())
}
fn check_key(host: &Host, key: &ResearchKey) -> Result<(), HostError> {
    if !host
        .startup
        .research
        .as_ref()
        .is_some_and(|n| n.task_id == key.task_id && n.subject_id == key.subject_id)
    {
        return Err(HostError::Configuration);
    }
    Ok(())
}
pub(crate) fn dispatch(
    host: &Host,
    request: &Value,
    recovery_only: bool,
) -> Result<Value, HostError> {
    if request.as_object().is_none_or(|o| o.len() != 2) {
        return Err(HostError::Configuration);
    }
    match request["op"].as_str() {
        Some("research.read.v1") => {
            let key: ResearchKey = serde_json::from_value(request["payload"].clone())?;
            check_key(host, &key)?;
            Ok(
                json!({"status":"observed","preparation":host.store.read_research_preparation(&key)?,"reconsideration":host.store.read_research_reconsideration(&key)?}),
            )
        }
        Some("research.read_adoption.v1") => {
            let q: AdoptionRead = serde_json::from_value(request["payload"].clone())?;
            let saved = host
                .store
                .read_research_adoption(&q.command_id, &q.payload_digest)?;
            if let Some(a) = &saved {
                check_key(host, &a.key)?;
            }
            Ok(json!({"status":"observed","adoption":saved}))
        }
        Some("research.adopt.v1") => {
            let q: ResearchAdopt = serde_json::from_value(request["payload"].clone())?;
            check_key(host, &q.key)?;
            let _gate = host
                .admission_gate
                .lock()
                .map_err(|_| HostError::LifecycleUnavailable)?;
            if let Some(saved) = host
                .store
                .read_research_adoption(&q.submit.command_id, &q.payload_digest()?)?
            {
                // Historical duplicate: no new preparation, grant, context or command construction.
                let lifecycle = if recovery_only {
                    read_command_result(host, &saved.command_id).ok().flatten()
                } else if host.selected {
                    reconcile_admission(host, &saved.command, true)
                        .ok()
                        .flatten()
                } else {
                    read_command_result(host, &saved.command_id).ok().flatten()
                };
                return Ok(
                    json!({"status":admission_status(lifecycle.as_ref()),"adoption":saved,"lifecycle":lifecycle,"qualification":"controlled_only"}),
                );
            }
            if recovery_only {
                return Err(HostError::RecoveryOnly);
            }
            adopt_fresh(host, &q)
        }
        Some("research.prepare.v1" | "research.invalidate.v1") => {
            if recovery_only {
                return Err(HostError::RecoveryOnly);
            }
            mutate(host, request)
        }
        _ => Err(HostError::Configuration),
    }
}
#[cfg(not(feature = "controlled-proof"))]
fn adopt_fresh(_host: &Host, _q: &ResearchAdopt) -> Result<Value, HostError> {
    Err(HostError::Task(TaskError::Unavailable))
}
#[cfg(not(feature = "controlled-proof"))]
fn mutate(_host: &Host, _request: &Value) -> Result<Value, HostError> {
    let _ = &_host.research;
    Err(HostError::Task(TaskError::Unavailable))
}
#[cfg(feature = "controlled-proof")]
fn adopt_fresh(host: &Host, q: &ResearchAdopt) -> Result<Value, HostError> {
    let f = host
        .research
        .get()
        .and_then(|installed| installed.fixture.as_ref())
        .ok_or(TaskError::Unavailable)?;
    if q.key != f.key
        || !f.bases.iter().any(|b| {
            b.submit == q.submit
                && research_digest("basis", b).ok().as_ref() == Some(&q.prepared_basis_digest)
        })
    {
        return Err(HostError::Configuration);
    }
    let basis = admission_basis(host, &q.submit.subject_id)?;
    let producer = host
        .store
        .controlled_research_producer(&q.key, ProducerRole::Adopter)?;
    let (saved, _) = host
        .store
        .adopt_research_with_admission(q, &producer, &mut |text| {
            build_admission(host, &q.submit, &basis, text)
        })?;
    if let Some(root) = &host.startup.lifecycle.proof_barrier_dir {
        if root.join("admission_after_owner_commit.return").exists() {
            fs::write(
                root.join("admission_after_owner_commit.reached"),
                b"reached\n",
            )?;
            return Err(HostError::LifecycleUnavailable);
        }
        if root.join("admission_after_owner_commit.panic").exists() {
            fs::write(
                root.join("admission_after_owner_commit.reached"),
                b"reached\n",
            )?;
            panic!("controlled research cut after owner commit");
        }
    }
    let lifecycle = reconcile_admission(host, &saved.command, true)
        .ok()
        .flatten();
    Ok(
        json!({"status":admission_status(lifecycle.as_ref()),"adoption":saved,"lifecycle":lifecycle,"qualification":"controlled_only"}),
    )
}
#[cfg(feature = "controlled-proof")]
fn mutate(host: &Host, request: &Value) -> Result<Value, HostError> {
    let f = host
        .research
        .get()
        .and_then(|installed| installed.fixture.as_ref())
        .ok_or(TaskError::Unavailable)?;
    match request["op"].as_str() {
        Some("research.prepare.v1") => {
            let q: PrepareRequest = serde_json::from_value(request["payload"].clone())?;
            check_key(host, &q.key)?;
            if q.key != f.key {
                return Err(HostError::Configuration);
            }
            let basis = f
                .bases
                .iter()
                .find(|b| research_digest("basis", b).ok().as_ref() == Some(&q.basis_digest))
                .ok_or(TaskError::Unavailable)?;
            let p = host
                .store
                .controlled_research_producer(&q.key, ProducerRole::Preparer)?;
            let checked = host
                .store
                .controlled_verify_research_basis(basis.clone(), &p)?;
            let result = host
                .store
                .prepare_research(&q.key, q.expected_revision, checked, &p)?;
            Ok(json!({"status":"prepared","preparation":result,"qualification":"controlled_only"}))
        }
        Some("research.invalidate.v1") => {
            let q: NoticeRequest = serde_json::from_value(request["payload"].clone())?;
            check_key(host, &q.key)?;
            if q.key != f.key || !f.notices.contains(&(q.notice_id.clone(), q.notice.clone())) {
                return Err(HostError::Configuration);
            }
            let role = if q.notice.selector.starts_with("research.") {
                ProducerRole::ResearchNotice
            } else {
                ProducerRole::DecisionNotice
            };
            let p = host.store.controlled_research_producer(&q.key, role)?;
            Ok(
                json!({"status":"invalidated","receipt":host.store.invalidate_research(&q.key,&q.notice_id,&q.notice,&p)?,"qualification":"controlled_only"}),
            )
        }
        _ => Err(HostError::Configuration),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use retained_task_workflow::sha256;
    #[cfg(feature = "controlled-proof")]
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[cfg(feature = "controlled-proof")]
    fn fixture() -> ControlledFixture {
        let key = ResearchKey {
            task_id: "task-0".into(),
            subject_id: "subject-0".into(),
            request_digest: sha256(b"request"),
        };
        let delivery = DeliveryBinding {
            request_digest: key.request_digest.clone(),
            card_digest: sha256(b"card"),
            input_artifact: "input".into(),
            complete: true,
            advisory_task: "advisory".into(),
            advisory_subject: "advisory".into(),
            context: "context-0".into(),
            attempt: "attempt-0".into(),
        };
        let recommendation = RecommendationBinding {
            delivery_digest: research_digest("delivery", &delivery).unwrap(),
            request_digest: key.request_digest.clone(),
            target_digest: sha256(b"target"),
            context: "context-0".into(),
            result_artifact: "result".into(),
            action: "audit".into(),
        };
        let result_bytes = serde_json::to_vec(&json!({"dependencies":{"decision.target":sha256(b"target")},"text":"unknown facets; competing evidence"})).unwrap();
        let mut basis = BasisBundle {
            version: 1,
            key: key.clone(),
            submit: serde_json::from_value(crate::tests::admission_submit()).unwrap(),
            target_digest: sha256(b"target"),
            snapshot_ref: "snapshot-0".into(),
            dependencies: std::collections::BTreeMap::from([(
                "decision.target".into(),
                sha256(b"target"),
            )]),
            dependency_artifacts: std::collections::BTreeMap::from([(
                "decision.target".into(),
                "target".into(),
            )]),
            artifacts: vec![
                RetainedArtifact {
                    id: "input".into(),
                    digest: sha256(b"card bytes"),
                    bytes: b"card bytes".to_vec(),
                },
                RetainedArtifact {
                    id: "result".into(),
                    digest: sha256(&result_bytes),
                    bytes: result_bytes,
                },
            ],
            delivery,
            recommendation,
            source_bundle_digest: sha256(b"source"),
            reconstruction_digest: sha256(b"retained"),
            authorization_digest: sha256(b"synthetic authority"),
            installation_digest: sha256(b"controlled only"),
            configuration_digest: sha256(b"configuration"),
        };
        for (id, bytes) in [
            ("target", b"target".as_slice()),
            ("source", b"source"),
            ("retained", b"retained"),
            ("authority", b"synthetic authority"),
            ("installation", b"controlled only"),
            ("configuration", b"configuration"),
            ("card", b"card"),
        ] {
            basis.artifacts.push(RetainedArtifact {
                id: id.into(),
                digest: sha256(bytes),
                bytes: bytes.to_vec(),
            });
        }
        ControlledFixture {
            version: 1,
            key,
            selectors: basis.dependencies.keys().cloned().collect(),
            bases: vec![basis],
            notices: vec![(
                "notice-1".into(),
                ChangeNotice {
                    selector: "decision.target".into(),
                    source_revision: sha256(b"revision2"),
                    reason: "changed target".into(),
                },
            )],
        }
    }
    fn nominated(root: &Path) -> Host {
        let mut host = crate::tests::admission_host(root, &root.join("unused.sock"), false);
        host.startup.research = Some(ResearchNomination {
            task_id: "task-0".into(),
            subject_id: "subject-0".into(),
            controlled_fixture: None,
        });
        host.research = OnceLock::from(install(&host.startup, &host.store).unwrap());
        host
    }
    #[test]
    fn default_missing_installer_and_ordinary_bypass_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let host = nominated(root.path());
        let ordinary = crate::dispatch(
            &host,
            json!({"op":"task.submit.v1","payload":crate::tests::admission_submit()}),
        );
        assert_eq!(ordinary["code"], "unauthorized");
        let refused = crate::dispatch(
            &host,
            json!({"op":"research.prepare.v1","payload":{"producer":"trusted","basis_digest":sha256(b"forged")}}),
        );
        assert_eq!(refused["code"], "unavailable");
        assert!(host.store.observe("task-0").unwrap().is_none());
        let native = crate::dispatch(&host, json!({"op":"task.research.prepare.v1","payload":{}}));
        assert!(native.get("code").is_some());
        let unrelated = ResearchKey {
            task_id: "private-other".into(),
            subject_id: "other".into(),
            request_digest: sha256(b"other"),
        };
        let response = crate::dispatch(&host, json!({"op":"research.read.v1","payload":unrelated}));
        assert!(!response.to_string().contains("preparation"));
    }
    #[cfg(feature = "controlled-proof")]
    fn controlled(root: &Path) -> (Host, ControlledFixture) {
        let mut host = nominated(root);
        let f = fixture();
        let path = root.join("qualification.json");
        fs::write(&path, serde_json::to_vec(&f).unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        host.startup.research.as_mut().unwrap().controlled_fixture = Some(path);
        host.research = OnceLock::from(install(&host.startup, &host.store).unwrap());
        (host, f)
    }
    #[cfg(feature = "controlled-proof")]
    #[test]
    fn duplicate_fixture_dependency_keys_refuse_before_nomination() {
        let root = tempfile::tempdir().unwrap();
        let mut host =
            crate::tests::admission_host(root.path(), &root.path().join("unused.sock"), false);
        let f = fixture();
        let json = serde_json::to_string(&f).unwrap();
        let pin = format!("\"decision.target\":\"{}\"", sha256(b"target"));
        assert!(json.contains(&pin));
        let malformed = json.replacen(&pin, &format!("{pin},{pin}"), 1);
        let path = root.path().join("duplicate-fixture.json");
        fs::write(&path, malformed).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        host.startup.research = Some(ResearchNomination {
            task_id: f.key.task_id.clone(),
            subject_id: f.key.subject_id.clone(),
            controlled_fixture: Some(path),
        });
        assert!(install(&host.startup, &host.store).is_err());
        // Refusal precedes the durable nomination guard, not merely later preparation.
        assert!(host.store.submit(&f.bases[0].submit).is_ok());
    }
    #[cfg(feature = "controlled-proof")]
    #[test]
    fn unsupported_fixture_selector_refuses_without_nomination_or_key() {
        let root = tempfile::tempdir().unwrap();
        let mut host =
            crate::tests::admission_host(root.path(), &root.path().join("unused.sock"), false);
        let f = fixture();
        let path = root.path().join("unsupported.json");
        // Change every occurrence, including the checked recommendation's retained JSON.
        let mut value = serde_json::to_value(&f).unwrap();
        let basis = &mut value["bases"][0];
        basis["dependencies"] = json!({"other.target":sha256(b"target")});
        basis["dependency_artifacts"] = json!({"other.target":"target"});
        let bytes = serde_json::to_vec(&json!({"dependencies":{"other.target":sha256(b"target")},"text":"unknown facets; competing evidence"})).unwrap();
        basis["artifacts"][1]["digest"] = json!(sha256(&bytes));
        basis["artifacts"][1]["bytes"] = json!(bytes);
        value["selectors"] = json!(["other.target"]);
        value["notices"][0][1]["selector"] = json!("other.target");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        host.startup.research = Some(ResearchNomination {
            task_id: f.key.task_id.clone(),
            subject_id: f.key.subject_id.clone(),
            controlled_fixture: Some(path),
        });
        let durable = || {
            let path = &host.startup.task_store_path;
            (
                fs::read(path).unwrap(),
                fs::read(path.with_extension("sqlite-wal")).ok(),
            )
        };
        let before = durable();
        assert!(install(&host.startup, &host.store).is_err());
        assert_eq!(durable(), before);
        assert!(
            host.store
                .read_research_preparation(&f.key)
                .unwrap()
                .is_none()
        );
        // Failed startup has not installed the nomination; ordinary handler operation remains valid.
        host.startup.research = None;
        let sends = Arc::new(AtomicUsize::new(0));
        let service = crate::tests::fake_admission_service(
            &host.startup.lifecycle.socket_path,
            3,
            sends.clone(),
        );
        let reply = crate::dispatch(
            &host,
            json!({"op":"task.submit.v1","payload":f.bases[0].submit}),
        );
        service.join().unwrap();
        assert_eq!(reply["status"], "admitted", "{reply}");
        assert_eq!(sends.load(Ordering::SeqCst), 1);
    }
    #[cfg(feature = "controlled-proof")]
    #[test]
    fn controlled_adoption_keeps_same_saved_command_after_notice_and_crash_cut() {
        let root = tempfile::tempdir().unwrap();
        let (mut host, f) = controlled(root.path());
        let basis = &f.bases[0];
        let prep = crate::dispatch(
            &host,
            json!({"op":"research.prepare.v1","payload":{"key":f.key,"expected_revision":null,"basis_digest":research_digest("basis",basis).unwrap()}}),
        );
        assert_eq!(prep["status"], "prepared", "{prep}");
        let q = ResearchAdopt {
            submit: basis.submit.clone(),
            key: f.key.clone(),
            expected_preparation_revision: prep["preparation"]["revision"].as_u64().unwrap(),
            prepared_basis_digest: prep["preparation"]["basis_digest"].as_str().unwrap().into(),
        };
        let barrier = root.path().join("cut");
        fs::create_dir(&barrier).unwrap();
        fs::write(barrier.join("admission_after_owner_commit.return"), b"cut").unwrap();
        host.startup.lifecycle.proof_barrier_dir = Some(barrier.clone());
        // Only preflight occurs before the controlled post-commit cut.
        let sends = Arc::new(AtomicUsize::new(0));
        let service = crate::tests::fake_admission_service(
            &host.startup.lifecycle.socket_path,
            1,
            sends.clone(),
        );
        let reply = crate::dispatch(&host, json!({"op":"research.adopt.v1","payload":q}));
        assert_eq!(reply["code"], "invalid_or_unavailable");
        service.join().unwrap();
        assert_eq!(sends.load(Ordering::SeqCst), 0);
        let saved = host
            .store
            .read_research_adoption(&q.submit.command_id, &q.payload_digest().unwrap())
            .unwrap()
            .unwrap();
        let n = &f.notices[0];
        let notice = crate::dispatch(
            &host,
            json!({"op":"research.invalidate.v1","payload":{"key":f.key,"notice_id":n.0,"notice":n.1}}),
        );
        assert_eq!(notice["status"], "invalidated");
        fs::remove_file(&host.startup.lifecycle.socket_path).unwrap();
        // Reconciliation reads saved unknown, validates the original context, and sends exactly once.
        let service = crate::tests::fake_admission_service(
            &host.startup.lifecycle.socket_path,
            2,
            sends.clone(),
        );
        let retry = crate::dispatch(&host, json!({"op":"research.adopt.v1","payload":q}));
        assert_eq!(retry["status"], "admitted", "{retry}");
        service.join().unwrap();
        assert_eq!(retry["adoption"], serde_json::to_value(&saved).unwrap());
        assert_eq!(sends.load(Ordering::SeqCst), 1);
        assert_eq!(host.store.observe("task-0").unwrap().unwrap().revision, 1);
        host.recovery_only = OnceLock::from(true);
        fs::remove_file(&host.startup.lifecycle.socket_path).unwrap();
        let service = crate::tests::fake_admission_service(
            &host.startup.lifecycle.socket_path,
            1,
            sends.clone(),
        );
        let observed = crate::dispatch(&host, json!({"op":"research.adopt.v1","payload":q}));
        service.join().unwrap();
        assert_eq!(observed["adoption"], serde_json::to_value(saved).unwrap());
        assert_eq!(sends.load(Ordering::SeqCst), 1);
        let mutation = crate::dispatch(
            &host,
            json!({"op":"research.invalidate.v1","payload":{"key":f.key,"notice_id":n.0,"notice":n.1}}),
        );
        assert_eq!(mutation["code"], "recovery_only");
    }
    #[cfg(feature = "controlled-proof")]
    #[tokio::test]
    async fn actual_linux_host_handler_qualifies_framing_prepare_refusal_and_readback() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let root = tempfile::tempdir().unwrap();
        let (host, f) = controlled(root.path());
        let host = Arc::new(host);
        async fn wire(host: Arc<Host>, bytes: Vec<u8>) -> Value {
            let (mut client, server) = AsyncUnixStream::pair().unwrap();
            let task = tokio::spawn(crate::handle(server, host));
            client.write_all(&bytes).await.unwrap();
            let mut response = Vec::new();
            if let Err(error) = client.read_to_end(&mut response).await {
                assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
                assert!(!response.is_empty());
            }
            task.await.unwrap().unwrap();
            serde_json::from_slice(&response).unwrap()
        }
        let request = json!({"op":"research.prepare.v1","payload":{"key":f.key,"expected_revision":null,"basis_digest":research_digest("basis",&f.bases[0]).unwrap()}});
        let mut bytes = serde_json::to_vec(&request).unwrap();
        bytes.push(b'\n');
        let reply = wire(host.clone(), bytes).await;
        assert_eq!(reply["status"], "prepared");
        assert_eq!(reply["qualification"], "controlled_only");
        let mut bytes =
            serde_json::to_vec(&json!({"op":"research.read.v1","payload":f.key})).unwrap();
        bytes.push(b'\n');
        let observed = wire(host.clone(), bytes).await;
        assert_eq!(observed["preparation"]["revision"], 1);
        let duplicate_keys =
            b"{\"op\":\"research.read.v1\",\"op\":\"research.prepare.v1\",\"payload\":{}}\n"
                .to_vec();
        let refused = wire(host.clone(), duplicate_keys).await;
        assert_eq!(refused["code"], "invalid_request");
        let mut too_big = vec![b' '; MAX_FRAME + 1];
        too_big.push(b'\n');
        assert_eq!(wire(host, too_big).await["code"], "request_too_large");
    }
}
