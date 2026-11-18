//! Controlled-proof derivation of the A4 material (`request.json`, `card.json`,
//! `reconstruction.json`, `target.txt`) over real register objects. The source closure is
//! exported from a non-linked checkout and written into a dedicated repository, where the use
//! bundle and approval events are committed. Approval validity here is custody only; the card
//! host never authenticates who wrote the events.
#![cfg(feature = "controlled-proof")]
use research_register::{
    card_admission::{ApprovalEvent, DisclosureScope},
    card_host::{self, CardRequest},
    card_wire::{self, CurrentCardRequest},
    read_host::{HostProfile, ReadRequest},
    use_bundle::{BlobPin, VERSION, revision},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Input file named by `WE_A4_DERIVE_INPUT`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeriveInput {
    /// Non-linked checkout holding the source commit (closure export only).
    source_repository: PathBuf,
    source_commit: String,
    register_path: String,
    decision_id: String,
    /// Dedicated repository; must not exist. Keep it short and fixed: the reconstruction embeds it twice.
    repository: PathBuf,
    /// Directory receiving the four material files.
    out: PathBuf,
    producer: String,
    issuers: BTreeSet<String>,
    delegation: String,
    purpose: String,
    scope: DisclosureScope,
    material: Material,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum Material {
    /// Operator-written bytes, committed verbatim and checked against the approved digests.
    Operator {
        /// Holds `manifest.json` and every record it names.
        bundle_dir: PathBuf,
        /// Holds `qualification.json` and `applicability.json`.
        approvals_dir: PathBuf,
        manifest_sha256: String,
        qualification_sha256: String,
        applicability_sha256: String,
    },
    /// Synthetic records and approvals for size measurement only; not operator acts.
    SyntheticMeasurement { finding: String },
}

fn git(repo: &Path, args: &[&str], stdin: Option<&[u8]>) -> Vec<u8> {
    use std::io::Write;
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=a4-derive",
            "-c",
            "user.email=a4-derive@invalid",
        ])
        .args(args)
        .env("GIT_AUTHOR_DATE", "2026-11-18T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-11-18T00:00:00Z")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.unwrap_or_default())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "git {args:?}");
    out.stdout
}
fn git_line(repo: &Path, args: &[&str]) -> String {
    String::from_utf8(git(repo, args, None))
        .unwrap()
        .trim()
        .to_string()
}
fn pin(repo: &Path, commit: &str, path: &str) -> BlobPin {
    let exact = git(
        repo,
        &["cat-file", "blob", &format!("{commit}:{path}")],
        None,
    );
    BlobPin {
        commit: commit.into(),
        path: path.into(),
        blob: git_line(repo, &["rev-parse", &format!("{commit}:{path}")]),
        sha256: revision(&exact),
    }
}
fn record_bytes(record: Value) -> Vec<u8> {
    let mut b = serde_json::to_vec(&json!({"schema":VERSION,"record":record})).unwrap();
    b.push(b'\n');
    b
}

/// Writes synthetic bundle records, manifest and approval events into `repo` (uncommitted).
fn write_synthetic_measurement(input: &DeriveInput, finding: &str) {
    let (repo, source) = (&input.repository, input.source_commit.as_str());
    let register = input.register_path.as_str();
    let decisions: Value = serde_json::from_slice(&git(
        repo,
        &[
            "cat-file",
            "blob",
            &format!("{source}:{register}/decisions.json"),
        ],
        None,
    ))
    .unwrap();
    let decision = decisions
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["decision_id"] == input.decision_id.as_str())
        .expect("decision registered at source");
    let document = decision["document"].as_str().unwrap();
    let decision_revision = decision["revision"].as_str().unwrap();
    let study_id = finding.split_once('#').unwrap().0;
    let target = pin(repo, source, document);
    let study = pin(repo, source, &format!("{register}/studies/{study_id}.json"));
    let mut records: Vec<(&str, Vec<u8>)> = vec![];
    let mut add = |name: &'static str, record: Value| {
        let bytes = record_bytes(record);
        let id = revision(&bytes);
        records.push((name, bytes));
        id
    };
    let evidence = add(
        "evidence",
        json!({"kind":"evidence","source":{"kind":"git","pin":target}}),
    );
    let assertion = add(
        "assertion",
        json!({"kind":"evidence","source":{"kind":"assertion","locator":"synthetic://measurement-only","asserted_sha256":null,"limitation":"SYNTHETIC measurement record; not an operator act"}}),
    );
    let subject = add(
        "subject",
        json!({"kind":"subject","finding":finding,"study":study,"evidence":[evidence]}),
    );
    let qualification = add(
        "qualification",
        json!({"kind":"qualification","assessor":"synthetic-owner","purpose":input.purpose,"disposition":"accepted","reviewer":"synthetic-owner","exposure_boundary":"SYNTHETIC measurement only","capability_evidence":[assertion],"authorization_evidence":[assertion],"evidence":[evidence]}),
    );
    let mut facets = serde_json::Map::new();
    for name in [
        "provenance",
        "method_reliability",
        "consistency",
        "reproducibility",
        "context_transfer",
        "inferential_strength",
        "completeness",
        "temporal_relevance",
    ] {
        facets.insert(
            name.into(),
            json!({"state":"unknown","reason":"SYNTHETIC measurement only","evidence":[evidence]}),
        );
    }
    add(
        "assessment",
        json!({"kind":"assessment","subject":subject,"assessor":"synthetic-owner","as_of_context":"SYNTHETIC measurement only","scope":"size measurement only","limitations":["SYNTHETIC; no assessment was made"],"evidence":[evidence],"facets":facets,"predecessors":[],"corrections":[],"competes_with":[],"qualification":[qualification]}),
    );
    let request = add(
        "request",
        json!({"kind":"request","request_id":"synthetic.measurement","decision":input.decision_id,"decision_revision":decision_revision,"target":target,"selector":"SYNTHETIC measurement selector","purpose":input.purpose,"alternatives":["defer"],"conditions":["SYNTHETIC measurement only"],"operating_context":"SYNTHETIC measurement only","owner":"synthetic-owner","acceptance":["SYNTHETIC measurement only"]}),
    );
    let applicability = add(
        "applicability",
        json!({"kind":"applicability","subject":subject,"request":request,"decision":input.decision_id,"decision_revision":decision_revision,"relation":"not_established","bounds":"SYNTHETIC measurement only","owner":"synthetic-owner","authorization_evidence":[assertion]}),
    );
    std::fs::create_dir_all(repo.join("bundle")).unwrap();
    let mut inputs = vec![];
    for (name, bytes) in &records {
        let path = format!("{name}.json");
        std::fs::write(repo.join("bundle").join(&path), bytes).unwrap();
        let blob = String::from_utf8(git(repo, &["hash-object", "--stdin"], Some(bytes))).unwrap();
        inputs.push(json!({"path":path,"blob":blob.trim(),"sha256":revision(bytes)}));
    }
    let tree = git_line(repo, &["rev-parse", &format!("{source}:{register}")]);
    let mut manifest = serde_json::to_vec(&json!({"schema":VERSION,"source":{"commit":source,"register_path":register,"register_tree":tree},"inputs":inputs,"request":request,"applicability":[applicability]})).unwrap();
    manifest.push(b'\n');
    std::fs::write(repo.join("bundle/manifest.json"), &manifest).unwrap();
    std::fs::create_dir_all(repo.join("approvals")).unwrap();
    for role in ["qualification", "applicability"] {
        let event = ApprovalEvent {
            issuer: input.issuers.first().unwrap().clone(),
            role: role.into(),
            delegation: input.delegation.clone(),
            purpose: input.purpose.clone(),
            decision_id: input.decision_id.clone(),
            decision_revision: decision_revision.into(),
            request_sha256: request.clone(),
            bundle_sha256: revision(&manifest),
            records: records.iter().map(|(_, b)| revision(b)).collect(),
            scope_sha256: revision(&serde_json::to_vec(&input.scope).unwrap()),
        };
        std::fs::write(
            repo.join(format!("approvals/{role}.json")),
            serde_json::to_vec(&event).unwrap(),
        )
        .unwrap();
    }
}

fn copy_operator_material(repo: &Path, bundle_dir: &Path, approvals_dir: &Path) {
    std::fs::create_dir_all(repo.join("bundle")).unwrap();
    for entry in std::fs::read_dir(bundle_dir).unwrap() {
        let entry = entry.unwrap();
        assert!(entry.file_type().unwrap().is_file(), "bundle_dir is flat");
        std::fs::copy(entry.path(), repo.join("bundle").join(entry.file_name())).unwrap();
    }
    std::fs::create_dir_all(repo.join("approvals")).unwrap();
    for role in ["qualification", "applicability"] {
        let name = format!("{role}.json");
        std::fs::copy(
            approvals_dir.join(&name),
            repo.join("approvals").join(&name),
        )
        .unwrap();
    }
}

#[test]
#[ignore = "derives A4 material from real register objects; requires WE_A4_DERIVE_INPUT"]
fn derive_real_a4() {
    let input: DeriveInput = serde_json::from_slice(
        &std::fs::read(std::env::var("WE_A4_DERIVE_INPUT").unwrap()).unwrap(),
    )
    .unwrap();
    let repo = input.repository.clone();
    assert!(!repo.exists(), "dedicated repository must not exist");
    let closure = card_host::synthetic_source_copies(
        &input.source_repository,
        &input.source_commit,
        &input.register_path,
    )
    .unwrap();
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"], None);
    for o in &closure {
        let oid = git(
            &repo,
            &["hash-object", "-w", "--stdin", "-t", &o.kind],
            Some(&o.bytes),
        );
        assert_eq!(String::from_utf8(oid).unwrap().trim(), o.oid);
    }
    match &input.material {
        Material::Operator {
            bundle_dir,
            approvals_dir,
            ..
        } => copy_operator_material(&repo, bundle_dir, approvals_dir),
        Material::SyntheticMeasurement { finding } => write_synthetic_measurement(&input, finding),
    }
    let digest = |path: &str| revision(&std::fs::read(repo.join(path)).unwrap());
    if let Material::Operator {
        manifest_sha256,
        qualification_sha256,
        applicability_sha256,
        ..
    } = &input.material
    {
        assert_eq!(&digest("bundle/manifest.json"), manifest_sha256);
        assert_eq!(
            &digest("approvals/qualification.json"),
            qualification_sha256
        );
        assert_eq!(
            &digest("approvals/applicability.json"),
            applicability_sha256
        );
    }
    git(&repo, &["add", "bundle", "approvals"], None);
    git(
        &repo,
        &["commit", "-qm", "use bundle and approval events"],
        None,
    );
    let commit = git_line(&repo, &["rev-parse", "HEAD"]);
    let manifest = pin(&repo, &commit, "bundle/manifest.json");
    let receipts = ["qualification", "applicability"]
        .map(|role| {
            let p = pin(&repo, &commit, &format!("approvals/{role}.json"));
            let bytes = std::fs::read(repo.join(&p.path)).unwrap();
            (p, bytes)
        })
        .to_vec();
    let event: ApprovalEvent = serde_json::from_slice(&receipts[0].1).unwrap();
    let request = CardRequest {
        source: ReadRequest {
            repository: repo.clone(),
            commit: input.source_commit.clone(),
            register_path: input.register_path.clone(),
            bundle: manifest,
        },
        current: CurrentCardRequest {
            decision_id: input.decision_id.clone(),
            decision_revision: event.decision_revision,
            request_sha256: event.request_sha256,
        },
        codec: card_wire::CODEC.into(),
    };
    let profile = HostProfile {
        repository_scope: Some(repo.clone()),
        ..HostProfile::default()
    };
    let worker_input = json!({"request":request,"profile":profile,"producer":input.producer,"installation":{"issuers":input.issuers,"delegation":input.delegation,"purpose":input.purpose,"scope":input.scope,"receipts":receipts,"role_issuers":null}});
    let reconstruction =
        card_host::synthetic_reconstruction(serde_json::from_value(worker_input).unwrap()).unwrap();
    let basis = serde_json::to_vec(&reconstruction).unwrap();
    assert_eq!(
        card_host::reconstruct(&reconstruction, &revision(&basis)).unwrap(),
        reconstruction.card_bytes
    );
    let envelope = card_wire::decode(&reconstruction.card_bytes).unwrap();
    let request_bytes = envelope
        .card
        .as_ref()
        .unwrap()
        .records
        .iter()
        .find(|r| r.revision == reconstruction.request.current.request_sha256)
        .unwrap()
        .utf8
        .as_bytes();
    let request_record: Value = serde_json::from_slice(request_bytes).unwrap();
    let target_blob = request_record["record"]["target"]["blob"].as_str().unwrap();
    let target = &reconstruction
        .objects
        .iter()
        .find(|o| o.oid == target_blob && o.kind == "blob")
        .expect("target blob retained in reconstruction")
        .bytes;
    std::fs::create_dir_all(&input.out).unwrap();
    std::fs::write(input.out.join("request.json"), request_bytes).unwrap();
    std::fs::write(input.out.join("card.json"), &reconstruction.card_bytes).unwrap();
    std::fs::write(input.out.join("reconstruction.json"), &basis).unwrap();
    std::fs::write(input.out.join("target.txt"), target).unwrap();
    let mut kinds: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for o in &reconstruction.objects {
        let e = kinds.entry(o.kind.as_str()).or_default();
        e.0 += 1;
        e.1 += o.bytes.len();
    }
    let material = request_bytes.len() + reconstruction.card_bytes.len() + basis.len() + 2;
    println!(
        "{}",
        json!({"bundle_commit":commit,"request":request_bytes.len(),"card":reconstruction.card_bytes.len(),
            "reconstruction":basis.len(),"material_with_separators":material,
            "reads_32k":material.div_ceil(32768),"objects":reconstruction.objects.len(),
            "objects_by_kind":kinds,"reconstruction_sha256":revision(&basis)})
    );
}
