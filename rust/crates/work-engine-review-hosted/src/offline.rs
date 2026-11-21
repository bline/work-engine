//! Private, test-only expected worlds. Nothing from a job record selects a world.

use std::borrow::Cow;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use review_episode_core::implementation_review_v1::SubjectParts;
use serde_json::{Value, json};
use work_engine_review_preparation::Subject;

use crate::revision::{FROZEN_REQUIREMENTS, Profile};
use crate::task::Limits;
use crate::{Refusal, inputs, sha256_hex};

mod check_tests;

const PARENT: &str = "d43b06f7aefc11a3df3812059e583df1125a495a";
const COMMIT: &str = "1b88dfd345a01fa073187baf1d3d3290de74db79";
const TREE: &str = "153f894198f23f43e94e680c54360c230e08b1a3";
const PATCH_IDENTITY: &str = "36c4967817b8440964669d75f2efaef1ba6977ea3b123539531c317c67183038";
const EXACT_SCOPE: &str = "abc39243b9a9f09c20307a53c837d4769ad88471193965a4ac6a64bd29c7106f";
const TWIN: &str = "b0aad594b8d18ca71a49f45f02ed452bce6850ef";
const PATCH: &[u8] = b"diff --git a/src/task.mjs b/src/task.mjs\nindex 24eef23..1322169 100644\n--- a/src/task.mjs\n+++ b/src/task.mjs\n@@ -1 +1 @@\n-// baseline\n+pub fn task() {}\n";
const SRC: &str = "pub fn task() {}\n";
const JOB: &str = "o7-synthetic-artifact-job";
const LIMITS: Limits = Limits {
    max_requests: 8,
    max_reads: 16,
};

pub(crate) struct ExpectedWorld {
    pub(crate) profile: Profile,
    pub(crate) id: &'static str,
    pub(crate) run_id: &'static str,
    pub(crate) requirements_sha256: String,
    requirements: Cow<'static, [u8]>,
}

impl ExpectedWorld {
    fn pinned(profile: Profile) -> Self {
        let (id, run_id, requirements_sha256, requirements) = match profile {
            Profile::OpenaiCodex => (
                "hosted-offline-exact-v1-responses",
                "hosted-offline-exact-v1-responses-fixture",
                "e072042c16727d734cce794dd95a3be8de6dd9e44d677a36c5e3e6b21ad13475",
                &include_bytes!("../fixtures/hosted-offline-exact-v1/responses/requirements.json")
                    [..],
            ),
            Profile::ClaudeSdk => (
                "hosted-offline-exact-v1-messages",
                "hosted-offline-exact-v1-messages-fixture",
                "c5843b94a71e7b8ba59df1447d462e71f9602d5856fde1af37779d206abc9e3f",
                &include_bytes!("../fixtures/hosted-offline-exact-v1/messages/requirements.json")[..],
            ),
        };
        Self {
            profile,
            id,
            run_id,
            requirements_sha256: requirements_sha256.to_owned(),
            requirements: Cow::Borrowed(requirements),
        }
    }

    pub(crate) fn requirements(&self) -> &[u8] {
        &self.requirements
    }

    pub(crate) fn verify_bytes(&self) -> Result<(), Refusal> {
        let actual = sha256_hex(self.requirements());
        if actual != self.requirements_sha256 {
            return Err(Refusal::Requirements(format!(
                "{} expected requirements sha256 {}, got {actual}",
                self.id, self.requirements_sha256
            )));
        }
        Ok(())
    }

    /// A coherent *negative* pin, not an unknown record or a hash mismatch masking scope.
    /// Only the scope and the test-control identity differ, never derived from a job.
    fn wrong_scope(profile: Profile, wrong: WrongScope) -> Self {
        let mut world = Self::pinned(profile);
        let (id, scope) = match wrong {
            WrongScope::Patch => ("negative-control-patch-as-scope", PATCH_IDENTITY.to_owned()),
            WrongScope::Other => ("negative-control-unrelated-scope", "5".repeat(64)),
        };
        let requirements = std::str::from_utf8(world.requirements())
            .unwrap()
            .replace(EXACT_SCOPE, &scope)
            .into_bytes();
        world.id = id;
        world.requirements_sha256 = sha256_hex(&requirements);
        world.requirements = Cow::Owned(requirements);
        world
    }
}

#[derive(Clone, Copy)]
enum WrongScope {
    Patch,
    Other,
}

fn pinned_subject() -> Subject {
    Subject {
        commit: COMMIT.to_owned(),
        tree: TREE.to_owned(),
        patch_identity: PATCH_IDENTITY.to_owned(),
    }
}

fn git(repo: &Path, args: &[&str], input: &[u8]) -> String {
    // Fixture authoring does not inherit Git config, attributes, repo redirects or diff
    // environment. The real materializer still runs its own object reads over this repo.
    let mut child = Command::new("git")
        .env_clear()
        .env(
            "PATH",
            std::env::var_os("PATH").expect("PATH for fixture Git"),
        )
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", out.status);
    String::from_utf8(out.stdout).unwrap().trim_end().to_owned()
}

fn git_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    git(
        path,
        &[
            "init",
            "--quiet",
            "--bare",
            "--object-format=sha1",
            "--template=",
        ],
        b"",
    );
    // Explicit local diff settings for the materializer's real Git object reads.
    for (key, value) in [
        ("core.abbrev", "7"),
        ("core.attributesFile", "/dev/null"),
        ("diff.algorithm", "myers"),
        ("diff.context", "3"),
        ("diff.interHunkContext", "0"),
        ("diff.mnemonicPrefix", "false"),
        ("diff.noprefix", "false"),
        ("diff.srcPrefix", "a/"),
        ("diff.dstPrefix", "b/"),
        ("diff.relative", "false"),
        ("color.diff", "never"),
    ] {
        git(path, &["config", "--local", key, value], b"");
    }
    // Write actual objects, not staging, working files, branches or commits via porcelain.
    let tree = |text: &[u8]| {
        let blob = write_object(path, "blob", text);
        let mut leaf = b"100644 task.mjs\0".to_vec();
        leaf.extend(object_id_bytes(&blob));
        let mut root = b"40000 src\0".to_vec();
        root.extend(object_id_bytes(&write_object(path, "tree", &leaf)));
        write_object(path, "tree", &root)
    };
    let commit = |tree: &str, parent: &str, message: &str| {
        let who = "Fixture <fixture@invalid> 1700000000 +0000";
        write_object(
            path,
            "commit",
            format!("tree {tree}\n{parent}author {who}\ncommitter {who}\n\n{message}\n").as_bytes(),
        )
    };
    let before = tree(b"// baseline\n");
    let after = tree(SRC.as_bytes());
    assert_eq!(after, TREE);
    assert_eq!(commit(&before, "", "baseline"), PARENT);
    assert_eq!(
        commit(&after, &format!("parent {PARENT}\n"), "candidate"),
        COMMIT
    );
    dir
}

fn write_object(path: &Path, kind: &str, bytes: &[u8]) -> String {
    git(path, &["hash-object", "-t", kind, "-w", "--stdin"], bytes)
}

fn object_id_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn scope(subject: &Subject) -> String {
    SubjectParts {
        commit: &subject.commit,
        tree: &subject.tree,
        patch_identity: &subject.patch_identity,
    }
    .digest()
}

#[test]
fn expected_worlds_change_only_the_pinned_run_scope_and_offline_propositions() {
    // Independent raw byte pins from records/org-hosted-review-20261117/h1/
    // remediation-6/final-hashes.txt:3 (R1), :5 (R2).
    for (profile, parent_sha256, revision, run, wire) in [
        (
            Profile::OpenaiCodex,
            "a2d08797241df85404f627e1603a9c980921236b2debf6ba34290819bb5803b3",
            "hosted-offline-exact-v1-responses",
            "hosted-offline-exact-v1-responses-fixture",
            "openai-codex / Responses",
        ),
        (
            Profile::ClaudeSdk,
            "2a2fac7b1e01bffc7e9f0a7ad0c7c29551ac3ef441966490b86789bdba5d1b43",
            "hosted-offline-exact-v1-messages",
            "hosted-offline-exact-v1-messages-fixture",
            "claude-sdk / Messages",
        ),
    ] {
        let world = ExpectedWorld::pinned(profile);
        world.verify_bytes().unwrap();
        assert_eq!(world.id, revision);
        assert_eq!(world.run_id, run);
        assert_eq!(Profile::parse(world.id), None);
        let parent_revision = profile.revision();
        let parent_bytes = parent_revision.requirements;
        assert_eq!(
            sha256_hex(parent_bytes),
            parent_sha256,
            "historical parent {} raw requirements bytes",
            parent_revision.id,
        );
        let mut parent: Value = serde_json::from_slice(parent_bytes).unwrap();
        parent["problemScope"]["runId"] = json!(run);
        parent["obligations"][0]["scope"]["digest"] = json!(EXACT_SCOPE);
        for (i, boundary) in ["builder projection", "campaign terminalization"]
            .into_iter()
            .enumerate()
        {
            parent["acceptanceConditions"][i]["proposition"] = json!(format!(
                "Offline exact-scope fixture ({revision}): compare synthetic review artifacts using the {wire} wire shape at the {boundary} boundary. No live route or provider is observed; this does not establish the condition or make the result consumable."
            ));
        }
        assert_eq!(
            serde_json::from_slice::<Value>(world.requirements()).unwrap(),
            parent
        );
    }
}

#[test]
fn writer_refuses_coherent_wrong_scope_before_any_staging() {
    let dir = git_fixture();
    let subject = pinned_subject();
    let out = dir.path().join("out");
    for (wrong, expected) in [
        (WrongScope::Patch, PATCH_IDENTITY.to_owned()),
        (WrongScope::Other, "5".repeat(64)),
    ] {
        assert_eq!(
            inputs::write(
                &ExpectedWorld::wrong_scope(Profile::OpenaiCodex, wrong),
                dir.path(),
                &subject,
                JOB,
                LIMITS,
                &out
            ),
            Err(Refusal::ScopeDigestMismatch {
                expected,
                actual: EXACT_SCOPE.to_owned()
            })
        );
        assert_no_staging(&out);
    }
}

#[test]
fn writer_refuses_same_tree_diff_metadata_twin_on_scope_only() {
    let dir = git_fixture();
    let body = format!(
        "tree {TREE}\nparent {PARENT}\nauthor Fixture <fixture@invalid> 1700000000 +0000\ncommitter Fixture <fixture@invalid> 1700000000 +0000\n\nmetadata twin\n"
    );
    assert_eq!(write_object(dir.path(), "commit", body.as_bytes()), TWIN);
    let twin = Subject {
        commit: TWIN.to_owned(),
        ..pinned_subject()
    };
    let actual = scope(&twin);
    assert_ne!(actual, EXACT_SCOPE);
    assert!(crate::subject::materialize(dir.path(), &twin, &actual).is_ok());
    let out = dir.path().join("out");
    assert_eq!(
        inputs::write(
            &ExpectedWorld::pinned(Profile::OpenaiCodex),
            dir.path(),
            &twin,
            JOB,
            LIMITS,
            &out
        ),
        Err(Refusal::ScopeDigestMismatch {
            expected: EXACT_SCOPE.to_owned(),
            actual
        })
    );
    assert_no_staging(&out);
}

#[test]
fn frozen_counterfactuals_emit_no_bundle_even_when_preparation_can_freeze() {
    let dir = git_fixture();
    let subject = pinned_subject();
    let out = dir.path().join("out");
    for profile in Profile::ALL {
        let mut expected = ExpectedWorld::pinned(profile);
        // Substitute the frozen requirements but retain the expected revision pin.
        expected.requirements = Cow::Borrowed(FROZEN_REQUIREMENTS);
        let refusal = inputs::write(&expected, dir.path(), &subject, JOB, LIMITS, &out);
        match profile {
            Profile::OpenaiCodex => assert!(
                matches!(refusal, Err(Refusal::KnownIncompatibility(ids)) if ids.len() == 2)
            ),
            Profile::ClaudeSdk => assert!(
                matches!(refusal, Err(Refusal::Requirements(error)) if error.contains("expected requirements sha256"))
            ),
        }
        assert_no_staging(&out);
    }
}

#[test]
fn writer_refuses_historical_requirements_even_under_the_expected_new_id() {
    let dir = git_fixture();
    let out = dir.path().join("out");
    for profile in Profile::ALL {
        let mut expected = ExpectedWorld::pinned(profile);
        expected.requirements = Cow::Borrowed(profile.revision().requirements);
        let refusal = inputs::write(&expected, dir.path(), &pinned_subject(), JOB, LIMITS, &out);
        assert!(
            matches!(refusal, Err(Refusal::Requirements(error)) if error.contains("expected requirements sha256"))
        );
        assert_no_staging(&out);
    }
}

fn assert_no_staging(out: &Path) {
    assert!(!out.exists());
    assert!(
        !out.with_file_name(format!(
            ".{}.staging-{}",
            out.file_name().unwrap().to_string_lossy(),
            std::process::id()
        ))
        .exists()
    );
}

#[test]
fn bundle_writes_atomically_and_leaves_nothing_on_io_failure() {
    let repo = git_fixture();
    let mut bundle = inputs::build(
        &ExpectedWorld::pinned(Profile::OpenaiCodex),
        repo.path(),
        &pinned_subject(),
        JOB,
        LIMITS,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("ok");
    inputs::write_bundle(&bundle, &out).unwrap();
    let record = std::fs::read(out.join("job-inputs.json")).unwrap();
    assert!(matches!(
        inputs::write_bundle(&bundle, &out),
        Err(Refusal::Io(_))
    ));
    assert_eq!(std::fs::read(out.join("job-inputs.json")).unwrap(), record);
    for path in [
        "snapshot/src/task.mjs",
        "task.txt",
        "contract.json",
        "requirements.json",
        "frozen-world.json",
        "job-inputs.json",
    ] {
        assert!(out.join(path).is_file(), "{path}");
    }
    // Partially written snapshot, then a member whose parent is a file.
    bundle.snapshot.members.extend([
        ("a".to_owned(), b"a\n".to_vec()),
        ("a/b".to_owned(), b"b\n".to_vec()),
    ]);
    let bad = dir.path().join("bad");
    assert!(matches!(
        inputs::write_bundle(&bundle, &bad),
        Err(Refusal::Io(_))
    ));
    assert_no_staging(&bad);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
