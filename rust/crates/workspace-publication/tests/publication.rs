//! Workspace-only fixture authority. Joined tests must use the campaign's
//! actual socket decisions; these cases qualify Git and crash mechanics.
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(feature = "test-faults")]
use std::time::{Duration, Instant};
use workspace_publication::{
    AttributedManifest, AuthorityMode, AuthorityReadback, BlobSide, CheckedCreate, Eligibility,
    GitOid, ManifestEntry, OperationState, PreparedPublication, PublicationCandidate,
    PublicationRequest, RecordRef, Workspace, WorkspaceConfig, codec,
};

fn run(repo: &Path, args: &[&str], input: Option<&[u8]>) -> String {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@localhost")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@localhost")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "git {:?}: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn id(hex: String) -> GitOid {
    GitOid {
        algorithm: "sha1".into(),
        hex,
    }
}
fn rf(kind: &str, id: &str) -> RecordRef {
    RecordRef {
        kind: kind.into(),
        schema_version: 1,
        root_id: "fixture-campaign".into(),
        record_id: id.into(),
        sha256: "a".repeat(64),
    }
}
fn temp_root() -> tempfile::TempDir {
    let root = PathBuf::from(
        std::env::var("WORK_ENGINE_W1_TEST_ROOT").expect("persistent W1 test root required"),
    );
    fs::create_dir_all(&root).unwrap();
    tempfile::Builder::new()
        .prefix("publication-")
        .tempdir_in(root)
        .unwrap()
}
#[derive(Clone, Serialize, Deserialize)]
struct FixtureContext {
    repo: PathBuf,
    root: PathBuf,
    eligibility_ref: RecordRef,
    eligibility: Eligibility,
    candidate: PublicationCandidate,
    request: PublicationRequest,
    decision_ref: RecordRef,
    consumption_ref: RecordRef,
}
impl FixtureContext {
    fn config(&self) -> WorkspaceConfig {
        WorkspaceConfig {
            root: self.root.clone(),
            repository: self.repo.clone(),
            canonical_refs: vec!["refs/heads/main".into()],
            validation_program: vec!["/usr/bin/true".into()],
            validation_policy_digest: "b".repeat(64),
            authority_mode: AuthorityMode::FixtureOperator,
        }
    }
}
impl AuthorityReadback for FixtureContext {
    fn check_eligibility(&self, r: &RecordRef) -> workspace_publication::Result<Eligibility> {
        if r != &self.eligibility_ref {
            return Err(workspace_publication::Error::Conflict(
                "fixture eligibility".into(),
            ));
        }
        Ok(self.eligibility.clone())
    }
    fn read_candidate(&self, digest: &str) -> workspace_publication::Result<PublicationCandidate> {
        if digest != self.eligibility.candidate_digest {
            return Err(workspace_publication::Error::Conflict(
                "fixture candidate".into(),
            ));
        }
        Ok(self.candidate.clone())
    }
    fn check_create_consumption(
        &self,
        r: &PublicationRequest,
    ) -> workspace_publication::Result<CheckedCreate> {
        if r != &self.request {
            return Err(workspace_publication::Error::Conflict(
                "fixture consumption".into(),
            ));
        }
        Ok(CheckedCreate {
            decision_ref: self.decision_ref.clone(),
            consumption_ref: self.consumption_ref.clone(),
        })
    }
}
fn setup() -> (tempfile::TempDir, FixtureContext, Workspace, String) {
    let temp = temp_root();
    let repo = temp.path().join("repo.git");
    fs::create_dir(&repo).unwrap();
    run(&repo, &["init", "--bare", "-b", "main"], None);
    let old_blob = id(run(
        &repo,
        &["hash-object", "-w", "--stdin"],
        Some(b"old\n"),
    ));
    let new_blob = id(run(
        &repo,
        &["hash-object", "-w", "--stdin"],
        Some(b"new\n"),
    ));
    let old_tree = id(run(
        &repo,
        &["mktree"],
        Some(format!("100644 blob {}\ta.txt\n", old_blob.hex).as_bytes()),
    ));
    let new_tree = id(run(
        &repo,
        &["mktree"],
        Some(format!("100644 blob {}\ta.txt\n", new_blob.hex).as_bytes()),
    ));
    let base = id(run(&repo, &["commit-tree", &old_tree.hex], Some(b"base\n")));
    run(&repo, &["update-ref", "refs/heads/main", &base.hex], None);
    let candidate_commit = id(run(
        &repo,
        &["commit-tree", &new_tree.hex, "-p", &base.hex],
        Some(b"candidate\n"),
    ));
    let attribution = rf("task-attribution", "evidence");
    let manifest = AttributedManifest {
        entries: vec![ManifestEntry {
            path: "a.txt".into(),
            before: Some(BlobSide {
                mode: 0o100644,
                blob: old_blob,
                raw_sha256: codec::raw_sha256(b"old\n"),
            }),
            after: Some(BlobSide {
                mode: 0o100644,
                blob: new_blob,
                raw_sha256: codec::raw_sha256(b"new\n"),
            }),
            task_attribution: "fixture-task".into(),
            attribution_evidence: attribution,
        }],
    };
    let candidate = PublicationCandidate {
        campaign: rf("campaign", "campaign"),
        candidate: rf("candidate", "candidate"),
        plan_revision: "v1".into(),
        scope_revision: "scope-v1".into(),
        checkpoint_commit: candidate_commit,
        checkpoint_tree: new_tree,
        checkpoint_parent: base.clone(),
        review_outcomes: vec![rf("review", "review")],
        gate_receipts: vec![rf("gate", "gate")],
        manifest,
    };
    let eligibility_ref = rf("eligibility", "accepted");
    let eligibility = Eligibility {
        candidate_digest: codec::digest("publication-candidate", &candidate).unwrap(),
        campaign_ref: candidate.campaign.clone(),
        acceptance_ref: rf("operator-decision", "accept"),
        scope: "publication-only".into(),
        active: true,
        revision: 1,
    };
    let root = temp.path().join("workspace");
    let mut ctx = FixtureContext {
        repo: repo.clone(),
        root,
        eligibility_ref: eligibility_ref.clone(),
        eligibility,
        candidate,
        request: PublicationRequest {
            offer_ref: rf("offer", "offer"),
            eligibility_ref,
            prepared_ref: rf("prepared", "pending"),
            operation_id: "pending".into(),
        },
        decision_ref: rf("decision", "create"),
        consumption_ref: rf("consumption", "consume"),
    };
    let mut workspace = Workspace::initialize(ctx.config()).unwrap();
    let (p_ref, p) = workspace
        .prepare(
            &ctx,
            &ctx.eligibility_ref,
            "refs/heads/main",
            "publish fixture",
            "prepare-1",
        )
        .unwrap();
    ctx.request.prepared_ref = p_ref;
    ctx.request.operation_id = p.operation_id;
    (temp, ctx, workspace, base.hex)
}
#[test]
fn publishes_exact_prepared_commit_and_reopens_without_replay() {
    let (_temp, ctx, mut workspace, base) = setup();
    let receipt = workspace.publish(&ctx, &ctx.request).unwrap();
    assert_eq!(receipt.state, OperationState::PublishedObserved);
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        receipt.commit.hex
    );
    assert_eq!(receipt.parent.hex, base);
    drop(workspace);
    let mut reopened = Workspace::open(ctx.config()).unwrap();
    let again = reopened.reconcile(&ctx, &ctx.request).unwrap();
    assert_eq!(again, receipt);
    assert!(reopened.publish(&ctx, &ctx.request).is_err());
    assert_eq!(
        reopened.inspect(&ctx.request.operation_id).unwrap().state,
        OperationState::PublishedObserved
    );
}
#[test]
fn stale_parent_is_closed_before_attempt() {
    let (_temp, ctx, mut workspace, _) = setup();
    let other = run(
        &ctx.repo,
        &[
            "commit-tree",
            &ctx.candidate.checkpoint_tree.hex,
            "-p",
            &ctx.candidate.checkpoint_parent.hex,
        ],
        Some(b"other\n"),
    );
    run(&ctx.repo, &["update-ref", "refs/heads/main", &other], None);
    let receipt = workspace.publish(&ctx, &ctx.request).unwrap();
    assert_eq!(receipt.state, OperationState::ClosedNoEffect);
    assert!(receipt.closure_ref.is_some());
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        other
    );
}

#[test]
fn reopened_binding_digests_and_wrong_request_refs_preserve_commit() {
    let (_temp, ctx, workspace, base) = setup();
    drop(workspace);
    let mut workspace = Workspace::open(ctx.config()).unwrap();
    let p: PreparedPublication = workspace.read_record(&ctx.request.prepared_ref).unwrap();
    let expected=codec::digest("preparation-binding",&serde_json::json!({"profile":workspace_publication::PROFILE,"version":1,"rootId":workspace.root_id(),"operationId":p.operation_id,
        "eligibility":p.eligibility_ref,"repositoryId":workspace.repository_id(),"canonicalRef":p.canonical_ref,"expectedParent":p.expected_parent,
        "manifestDigest":p.manifest_digest,"integratedTree":p.integrated_tree,"validationPolicyDigest":p.validation_policy_digest,
        "treeValidation":p.validation_ref,"message":p.proposed_message})).unwrap();
    assert_eq!(p.preparation_digest, expected);
    assert_eq!(
        p.message,
        format!(
            "{}\n\nWork-Engine-Operation: {}\nWork-Engine-Preparation: {}\n",
            p.proposed_message, p.operation_id, expected
        )
    );
    let prepared_oid = p.prepared_commit.hex.clone();
    let commit = Command::new("git")
        .arg("-C")
        .arg(&ctx.repo)
        .args(["cat-file", "commit", &prepared_oid])
        .output()
        .unwrap();
    assert!(commit.status.success());
    let separator = commit.stdout.windows(2).position(|w| w == b"\n\n").unwrap();
    let headers = std::str::from_utf8(&commit.stdout[..separator]).unwrap();
    assert!(
        headers
            .lines()
            .any(|line| line == format!("tree {}", p.integrated_tree.hex))
    );
    assert!(
        headers
            .lines()
            .any(|line| line == format!("parent {}", p.expected_parent.hex))
    );
    assert_eq!(&commit.stdout[separator + 2..], p.message.as_bytes());
    for field in ["offer", "eligibility", "prepared"] {
        let mut bad = ctx.request.clone();
        match field {
            "offer" => bad.offer_ref = rf("completion-offer", "wrong"),
            "eligibility" => bad.eligibility_ref = rf("eligibility", "wrong"),
            _ => bad.prepared_ref = rf("prepared-publication", "wrong"),
        }
        assert!(workspace.publish(&ctx, &bad).is_err(), "{field}");
        assert_eq!(
            run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
            base
        );
    }
    let receipt = workspace.publish(&ctx, &ctx.request).unwrap();
    assert_eq!(receipt.state, OperationState::PublishedObserved);
    assert_eq!(receipt.commit.hex, prepared_oid);
    let repository_id = workspace.repository_id().to_string();
    drop(workspace);
    let reopened = Workspace::open(ctx.config()).unwrap();
    let op = reopened.inspect(&ctx.request.operation_id).unwrap();
    let admission: serde_json::Value = reopened
        .read_record(op.admission_ref.as_ref().unwrap())
        .unwrap();
    let generation = op.generation.unwrap();
    let recomputed=codec::digest("admission-binding",&serde_json::json!({"operationId":p.operation_id,"preparationDigest":p.preparation_digest,"prepared":ctx.request.prepared_ref,
        "offer":ctx.request.offer_ref,"createDecision":ctx.decision_ref,"campaignConsumption":ctx.consumption_ref,
        "resources":[p.canonical_ref,repository_id,p.artifact_identity],"generation":generation})).unwrap();
    assert_eq!(admission["digest"], recomputed);
    assert_eq!(
        run(&ctx.repo, &["rev-parse", &prepared_oid], None),
        prepared_oid
    );
}

#[test]
fn successful_publication_preserves_attached_user_state_and_checkpoint_ref() {
    let (temp, ctx, mut workspace, _) = setup();
    let user_ref = "refs/heads/user-work";
    run(&ctx.repo, &["branch", "user-work", "refs/heads/main"], None);
    let attached = temp.path().join("user-attached");
    run(
        &ctx.repo,
        &["worktree", "add", attached.to_str().unwrap(), user_ref],
        None,
    );
    let index = PathBuf::from(run(
        &attached,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
        None,
    ));
    let unrelated = attached.join("unrelated.txt");
    fs::write(&unrelated, b"unrelated exact bytes\n").unwrap();
    let before_index = fs::read(&index).unwrap();
    let before_file = fs::read(attached.join("a.txt")).unwrap();
    let before_unrelated = fs::read(&unrelated).unwrap();
    let before_user = run(&ctx.repo, &["rev-parse", user_ref], None);
    let checkpoint_ref = "refs/checkpoints/accepted";
    run(
        &ctx.repo,
        &[
            "update-ref",
            checkpoint_ref,
            &ctx.candidate.checkpoint_commit.hex,
        ],
        None,
    );
    let before_checkpoint = run(&ctx.repo, &["rev-parse", checkpoint_ref], None);
    assert_eq!(
        workspace.publish(&ctx, &ctx.request).unwrap().state,
        OperationState::PublishedObserved
    );
    assert_eq!(run(&ctx.repo, &["rev-parse", user_ref], None), before_user);
    assert_eq!(
        run(&ctx.repo, &["rev-parse", checkpoint_ref], None),
        before_checkpoint
    );
    assert_eq!(fs::read(index).unwrap(), before_index);
    assert_eq!(fs::read(attached.join("a.txt")).unwrap(), before_file);
    assert_eq!(fs::read(unrelated).unwrap(), before_unrelated);
}

#[test]
fn wrong_checkpoint_missing_review_and_changed_policy_are_refused() {
    let (_temp, ctx, workspace, base) = setup();
    let mut wrong = ctx.candidate.clone();
    wrong.checkpoint_commit = id(base.clone());
    assert!(
        workspace
            .inspect_candidate(&wrong)
            .unwrap_err()
            .to_string()
            .contains("checkpoint object binding")
    );
    let mut missing = ctx.candidate.clone();
    missing.review_outcomes.clear();
    assert!(
        workspace
            .inspect_candidate(&missing)
            .unwrap_err()
            .to_string()
            .contains("review or gate evidence missing")
    );
    let mut policy = ctx.config();
    policy.validation_policy_digest = "c".repeat(64);
    assert!(
        Workspace::open(policy)
            .err()
            .unwrap()
            .to_string()
            .contains("installed workspace config changed")
    );
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        base
    );
}

#[test]
fn child_entry() {
    let Ok(path) = std::env::var("W1_CHILD_CONTEXT") else {
        return;
    };
    let ctx: FixtureContext = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let mut workspace = Workspace::open(ctx.config()).unwrap();
    if std::env::var("W1_CHILD_ACTION").ok().as_deref() == Some("checkout") {
        let _ = workspace.managed_checkout_fixture(
            "refs/heads/main",
            &ctx.root.join("managed-checkouts").join("checkout-child"),
        );
    } else if std::env::var("W1_CHILD_ACTION").ok().as_deref() == Some("poisoned-prepare") {
        workspace
            .prepare(
                &ctx,
                &ctx.eligibility_ref,
                "refs/heads/main",
                "poisoned environment preparation",
                "prepare-poisoned",
            )
            .unwrap();
    } else if std::env::var("W1_CHILD_ACTION").ok().as_deref() == Some("poisoned-publish") {
        let receipt = workspace.publish(&ctx, &ctx.request).unwrap();
        assert_eq!(receipt.state, OperationState::PublishedObserved);
    } else {
        let _ = workspace.publish(&ctx, &ctx.request);
    }
}
#[test]
fn repository_owner_rejects_second_ledger() {
    let (_temp, ctx, workspace, _) = setup();
    drop(workspace);
    let mut alternate = ctx.config();
    alternate.root = ctx.root.with_file_name("other-workspace");
    assert!(Workspace::initialize(alternate).is_err());
    assert!(Workspace::open(ctx.config()).is_ok());
}

#[test]
fn poisoned_git_selection_preserves_attached_index_and_branch() {
    let (temp, ctx, workspace, base) = setup();
    drop(workspace);
    let attached = temp.path().join("attached");
    run(
        &ctx.repo,
        &[
            "worktree",
            "add",
            attached.to_str().unwrap(),
            "refs/heads/main",
        ],
        None,
    );
    let index = PathBuf::from(run(
        &attached,
        &["rev-parse", "--path-format=absolute", "--git-path", "index"],
        None,
    ));
    let before = fs::read(&index).unwrap();
    let file = fs::read(attached.join("a.txt")).unwrap();
    let context = temp.path().join("poison-context.json");
    fs::write(&context, serde_json::to_vec(&ctx).unwrap()).unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_entry", "--nocapture"])
        .env("W1_CHILD_CONTEXT", &context)
        .env("W1_CHILD_ACTION", "poisoned-prepare")
        .env("GIT_INDEX_FILE", &index)
        .env("GIT_DIR", &ctx.repo)
        .env("GIT_WORK_TREE", &attached)
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(fs::read(index).unwrap(), before);
    assert_eq!(fs::read(attached.join("a.txt")).unwrap(), file);
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        base
    );
}

#[test]
fn poisoned_git_namespace_cannot_redirect_canonical_publish() {
    let (temp, ctx, workspace, base) = setup();
    drop(workspace);
    let namespace = "refs/namespaces/w1-poison/refs/heads/main";
    run(&ctx.repo, &["update-ref", namespace, &base], None);
    let context = temp.path().join("namespace-context.json");
    fs::write(&context, serde_json::to_vec(&ctx).unwrap()).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_entry", "--nocapture"])
        .env("W1_CHILD_CONTEXT", &context)
        .env("W1_CHILD_ACTION", "poisoned-publish")
        .env("GIT_NAMESPACE", "w1-poison")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let canonical = run(&ctx.repo, &["rev-parse", "refs/heads/main"], None);
    assert_ne!(canonical, base);
    assert_eq!(run(&ctx.repo, &["rev-parse", namespace], None), base);
    assert_eq!(
        Workspace::open(ctx.config())
            .unwrap()
            .inspect(&ctx.request.operation_id)
            .unwrap()
            .state,
        OperationState::PublishedObserved
    );
}
#[cfg(feature = "test-faults")]
fn wait_ready(path: &Path) {
    let start = Instant::now();
    while !path.exists() {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "child did not reach cut"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[cfg(feature = "test-faults")]
fn child(ctx: &FixtureContext, dir: &Path, cut: &str) -> std::process::Child {
    let context = dir.join("context.json");
    fs::write(&context, serde_json::to_vec(ctx).unwrap()).unwrap();
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_entry", "--nocapture"])
        .env("W1_CHILD_CONTEXT", context)
        .env("WORKSPACE_PUBLICATION_FAULT_CUT", cut)
        .env("WORKSPACE_PUBLICATION_FAULT_DIR", dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}
#[cfg(feature = "test-faults")]
fn child_action(ctx: &FixtureContext, dir: &Path, action: &str) -> std::process::Child {
    let context = dir.join(format!("context-{action}.json"));
    fs::write(&context, serde_json::to_vec(ctx).unwrap()).unwrap();
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_entry", "--nocapture"])
        .env("W1_CHILD_CONTEXT", context)
        .env("W1_CHILD_ACTION", action)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}
#[cfg(feature = "test-faults")]
#[test]
fn delayed_publisher_cannot_cross_durable_closure() {
    let (temp, ctx, mut workspace, base) = setup();
    let mut publisher = child(&ctx, temp.path(), "before_admission");
    wait_ready(&temp.path().join("before_admission.ready"));
    let closed = workspace.close_no_effect(&ctx, &ctx.request).unwrap();
    assert_eq!(closed.state, OperationState::ClosedNoEffect);
    fs::write(temp.path().join("before_admission.release"), b"go").unwrap();
    assert!(publisher.wait().unwrap().success());
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        base
    );
    assert_eq!(
        Workspace::open(ctx.config())
            .unwrap()
            .inspect(&ctx.request.operation_id)
            .unwrap()
            .state,
        OperationState::ClosedNoEffect
    );
}
#[cfg(feature = "test-faults")]
#[test]
fn crash_after_attempt_start_is_unresolved_and_never_retried() {
    let (temp, ctx, workspace, base) = setup();
    drop(workspace);
    let mut publisher = child(&ctx, temp.path(), "after_attempt_started");
    wait_ready(&temp.path().join("after_attempt_started.ready"));
    publisher.kill().unwrap();
    publisher.wait().unwrap();
    let mut reopened = Workspace::open(ctx.config()).unwrap();
    let receipt = reopened.reconcile(&ctx, &ctx.request).unwrap();
    assert_eq!(receipt.state, OperationState::Unresolved);
    assert_eq!(receipt.observed_target.unwrap().hex, base);
    assert!(reopened.publish(&ctx, &ctx.request).is_err());
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        base
    );
}
#[cfg(feature = "test-faults")]
#[test]
fn second_root_cannot_publish_past_uncertain_first_root() {
    let (temp, ctx, workspace, base) = setup();
    drop(workspace);
    let mut first = child(&ctx, temp.path(), "after_attempt_started");
    wait_ready(&temp.path().join("after_attempt_started.ready"));
    first.kill().unwrap();
    first.wait().unwrap();
    let mut alternate = ctx.config();
    alternate.root = temp.path().join("alternate-ledger");
    assert!(Workspace::initialize(alternate).is_err());
    let mut reopened = Workspace::open(ctx.config()).unwrap();
    assert_eq!(
        reopened.reconcile(&ctx, &ctx.request).unwrap().state,
        OperationState::Unresolved
    );
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        base
    );
}

#[cfg(feature = "test-faults")]
#[test]
fn checkout_child_retains_guard_after_parent_kill() {
    let (temp, ctx, workspace, _) = setup();
    drop(workspace);
    let ready = temp.path().join("git-child.ready");
    let release = temp.path().join("git-child.release");
    let wrapper = temp.path().join("guarded-git.py");
    fs::write(&wrapper,format!("#!/usr/bin/python3\nimport os,sys,time\nopen({:?},'wb').write(b'ready')\nwhile not os.path.exists({:?}): time.sleep(0.02)\nsink=os.open('/dev/null',os.O_WRONLY)\nos.dup2(sink,1)\nos.dup2(sink,2)\nos.execv('/usr/bin/git',['git']+sys.argv[1:])\n",ready.to_string_lossy(),release.to_string_lossy())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let context = temp.path().join("checkout-parent.json");
    fs::write(&context, serde_json::to_vec(&ctx).unwrap()).unwrap();
    let mut parent = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_entry", "--nocapture"])
        .env("W1_CHILD_CONTEXT", &context)
        .env("W1_CHILD_ACTION", "checkout")
        .env("WORKSPACE_PUBLICATION_GUARDED_GIT_FIXTURE", &wrapper)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_ready(&ready);
    parent.kill().unwrap();
    parent.wait().unwrap();
    let mut publisher = child_action(&ctx, temp.path(), "publisher");
    std::thread::sleep(Duration::from_millis(250));
    assert!(
        publisher.try_wait().unwrap().is_none(),
        "publisher crossed checkout child's guard"
    );
    fs::write(&release, b"go").unwrap();
    assert!(publisher.wait().unwrap().success());
    assert!(ctx.root.join("managed-checkouts/checkout-child").exists());
}
#[cfg(feature = "test-faults")]
#[test]
fn crash_after_admission_can_only_close_under_guard() {
    let (temp, ctx, workspace, base) = setup();
    drop(workspace);
    let mut publisher = child(&ctx, temp.path(), "after_admission");
    wait_ready(&temp.path().join("after_admission.ready"));
    publisher.kill().unwrap();
    publisher.wait().unwrap();
    let mut reopened = Workspace::open(ctx.config()).unwrap();
    assert_eq!(
        reopened.inspect(&ctx.request.operation_id).unwrap().state,
        OperationState::Admitted
    );
    let closed = reopened.close_no_effect(&ctx, &ctx.request).unwrap();
    assert_eq!(closed.state, OperationState::ClosedNoEffect);
    assert!(closed.closure_ref.is_some());
    assert!(reopened.publish(&ctx, &ctx.request).is_ok());
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        base
    );
}
#[cfg(feature = "test-faults")]
#[test]
fn live_guard_excludes_second_publisher_and_managed_checkout() {
    let (temp, ctx, workspace, _) = setup();
    let mut first = child(&ctx, temp.path(), "after_attempt_started");
    wait_ready(&temp.path().join("after_attempt_started.ready"));
    let generation = workspace
        .inspect(&ctx.request.operation_id)
        .unwrap()
        .generation;
    let mut second = child_action(&ctx, temp.path(), "publisher");
    let mut checkout = child_action(&ctx, temp.path(), "checkout");
    std::thread::sleep(Duration::from_millis(1200));
    assert_eq!(
        workspace
            .inspect(&ctx.request.operation_id)
            .unwrap()
            .generation,
        generation
    );
    assert!(
        second.try_wait().unwrap().is_none(),
        "second publisher crossed guard"
    );
    assert!(
        checkout.try_wait().unwrap().is_none(),
        "checkout crossed guard"
    );
    fs::write(temp.path().join("after_attempt_started.release"), b"go").unwrap();
    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());
    assert!(checkout.wait().unwrap().success());
    let reopened = Workspace::open(ctx.config()).unwrap();
    assert_eq!(
        reopened.inspect(&ctx.request.operation_id).unwrap().state,
        OperationState::PublishedObserved
    );
    assert!(
        ctx.root
            .join("managed-checkouts")
            .join("checkout-child")
            .exists()
    );
    let canonical = run(&ctx.repo, &["rev-parse", "refs/heads/main"], None);
    assert!(
        Workspace::open(ctx.config())
            .unwrap()
            .publish(&ctx, &ctx.request)
            .is_err()
    );
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        canonical
    );
}

#[cfg(feature = "test-faults")]
#[test]
fn unmanaged_checkout_and_external_aba_remain_custody_limits() {
    for actor in ["checkout", "aba"] {
        let (temp, ctx, workspace, base) = setup();
        drop(workspace);
        let mut publisher = child(&ctx, temp.path(), "after_attempt_started");
        wait_ready(&temp.path().join("after_attempt_started.ready"));
        if actor == "checkout" {
            let outside = temp.path().join("unmanaged-checkout");
            run(
                &ctx.repo,
                &[
                    "worktree",
                    "add",
                    outside.to_str().unwrap(),
                    "refs/heads/main",
                ],
                None,
            );
            assert!(outside.exists());
        } else {
            run(
                &ctx.repo,
                &[
                    "update-ref",
                    "refs/heads/main",
                    &ctx.candidate.checkpoint_commit.hex,
                ],
                None,
            );
            run(&ctx.repo, &["update-ref", "refs/heads/main", &base], None);
        }
        fs::write(temp.path().join("after_attempt_started.release"), b"go").unwrap();
        assert!(publisher.wait().unwrap().success());
        let mut reopened = Workspace::open(ctx.config()).unwrap();
        let result = reopened.reconcile(&ctx, &ctx.request).unwrap();
        assert_eq!(result.state, OperationState::PublishedObserved);
        assert_eq!(
            run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
            result.commit.hex
        );
        // This fixture intentionally crossed the enrolled guard. The result
        // cannot be used as evidence of custody or absence of ABA history.
    }
}
#[cfg(feature = "test-faults")]
#[test]
fn crash_after_physical_cas_reconciles_without_second_attempt() {
    let (temp, ctx, workspace, _) = setup();
    drop(workspace);
    let mut publisher = child(&ctx, temp.path(), "after_ref_cas");
    wait_ready(&temp.path().join("after_ref_cas.ready"));
    publisher.kill().unwrap();
    publisher.wait().unwrap();
    let mut reopened = Workspace::open(ctx.config()).unwrap();
    let receipt = reopened.reconcile(&ctx, &ctx.request).unwrap();
    assert_eq!(receipt.state, OperationState::PublishedObserved);
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        receipt.commit.hex
    );
    assert!(reopened.publish(&ctx, &ctx.request).is_err());
}
#[cfg(feature = "test-faults")]
#[test]
fn missing_or_tampered_admission_never_confirms_effect() {
    for (cut, damage, expected) in [
        (
            "after_ref_cas",
            "delete",
            OperationState::PublishedUnconfirmed,
        ),
        (
            "after_attempt_started",
            "tamper",
            OperationState::Unresolved,
        ),
    ] {
        let (temp, ctx, workspace, base) = setup();
        drop(workspace);
        let mut publisher = child(&ctx, temp.path(), cut);
        wait_ready(&temp.path().join(format!("{cut}.ready")));
        publisher.kill().unwrap();
        publisher.wait().unwrap();
        let conn =
            rusqlite::Connection::open(ctx.root.join("workspace-publication.sqlite")).unwrap();
        let record_id = format!("admission-{}", ctx.request.operation_id);
        if damage == "delete" {
            conn.execute("DELETE FROM records WHERE record_id=?1", [record_id])
                .unwrap();
        } else {
            conn.execute(
                "UPDATE records SET payload=X'7b7d' WHERE record_id=?1",
                [record_id],
            )
            .unwrap();
        }
        drop(conn);
        let mut reopened = Workspace::open(ctx.config()).unwrap();
        let receipt = reopened.reconcile(&ctx, &ctx.request).unwrap();
        assert_eq!(receipt.state, expected);
        assert!(reopened.publish(&ctx, &ctx.request).is_err());
        let current = run(&ctx.repo, &["rev-parse", "refs/heads/main"], None);
        if expected == OperationState::Unresolved {
            assert_eq!(current, base);
        } else {
            assert_eq!(current, receipt.commit.hex);
        }
    }
}
#[cfg(feature = "test-faults")]
#[test]
fn crash_before_result_finalization_reconciles_observed_commit() {
    let (temp, ctx, workspace, _) = setup();
    drop(workspace);
    let mut publisher = child(&ctx, temp.path(), "before_result");
    wait_ready(&temp.path().join("before_result.ready"));
    publisher.kill().unwrap();
    publisher.wait().unwrap();
    let mut reopened = Workspace::open(ctx.config()).unwrap();
    let receipt = reopened.reconcile(&ctx, &ctx.request).unwrap();
    assert_eq!(receipt.state, OperationState::PublishedObserved);
    assert!(reopened.publish(&ctx, &ctx.request).is_err());
    assert_eq!(
        run(&ctx.repo, &["rev-parse", "refs/heads/main"], None),
        receipt.commit.hex
    );
}
#[cfg(feature = "test-faults")]
#[test]
fn in_process_expected_old_cas_rejects_unmanaged_parent_change() {
    let (temp, ctx, workspace, _) = setup();
    drop(workspace);
    let mut publisher = child(&ctx, temp.path(), "after_attempt_started");
    wait_ready(&temp.path().join("after_attempt_started.ready"));
    let other = run(
        &ctx.repo,
        &[
            "commit-tree",
            &ctx.candidate.checkpoint_tree.hex,
            "-p",
            &ctx.candidate.checkpoint_parent.hex,
        ],
        Some(b"outside\n"),
    );
    run(&ctx.repo, &["update-ref", "refs/heads/main", &other], None);
    fs::write(temp.path().join("after_attempt_started.release"), b"go").unwrap();
    assert!(publisher.wait().unwrap().success());
    let mut reopened = Workspace::open(ctx.config()).unwrap();
    let receipt = reopened.reconcile(&ctx, &ctx.request).unwrap();
    assert_eq!(receipt.state, OperationState::Unresolved);
    assert_eq!(receipt.observed_target.unwrap().hex, other);
}
