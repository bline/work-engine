//! Outcome 4: subject materialization, on synthetic git repositories only.

mod common;

use common::{git, repo, repo_with};
use review_episode_core::codec::digest;
use std::collections::BTreeMap;
use work_engine_review_hosted::secrets::{self, RULE_IDS};
use work_engine_review_hosted::subject::{
    REVIEW_PATCH, REVIEW_SUBJECT, Snapshot, check_context_fit, check_members, check_read_budget,
    materialize, patch_bytes,
};
use work_engine_review_hosted::{Refusal, sha256_hex};
use work_engine_review_preparation::Subject;

#[test]
fn identity_ok_snapshot_is_changed_files_patch_and_subject() {
    let r = repo();
    let snapshot = materialize(&r.path(), &r.subject, &digest(&r.subject.to_js_value())).unwrap();
    let paths: Vec<&str> = snapshot.members.keys().map(String::as_str).collect();
    assert_eq!(paths, [REVIEW_PATCH, REVIEW_SUBJECT, "a.txt", "src/b.rs"]);
    assert_eq!(snapshot.members["a.txt"], b"alpha two\n");
    assert_eq!(snapshot.parent, r.parent);
    // P3-4: the patch member is exactly the bytes hashed for patchIdentity.
    assert_eq!(snapshot.manifest()[REVIEW_PATCH], r.subject.patch_identity);
    let subject: serde_json::Value =
        serde_json::from_slice(&snapshot.members[REVIEW_SUBJECT]).unwrap();
    assert_eq!(subject["commit"], r.subject.commit.as_str());
    assert_eq!(subject["tree"], r.subject.tree.as_str());
    assert_eq!(subject["patchIdentity"], r.subject.patch_identity.as_str());
    assert_eq!(subject["parent"], r.parent.as_str());
    let listed: Vec<&str> = subject["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["path"].as_str().unwrap())
        .collect();
    assert_eq!(listed, [REVIEW_PATCH, "a.txt", "src/b.rs"]);
    // Written members equal their manifest digests.
    let out = r.path().join("snap-out");
    snapshot.write(&out).unwrap();
    for (path, sha) in snapshot.manifest() {
        assert_eq!(
            sha256_hex(&std::fs::read(out.join(&path)).unwrap()),
            sha,
            "{path}"
        );
    }
}

#[test]
fn tree_mismatch_is_refused() {
    let r = repo();
    let subject = Subject {
        tree: r.parent_tree(),
        ..r.subject.clone()
    };
    assert!(matches!(
        materialize(&r.path(), &subject, &digest(&subject.to_js_value())),
        Err(Refusal::TreeMismatch { .. })
    ));
}

trait ParentTree {
    fn parent_tree(&self) -> String;
}
impl ParentTree for common::Repo {
    fn parent_tree(&self) -> String {
        git(
            &self.path(),
            &["rev-parse", &format!("{}^{{tree}}", self.parent)],
        )
    }
}

#[test]
fn patch_identity_mismatch_is_refused() {
    let r = repo();
    let subject = Subject {
        patch_identity: "0".repeat(64),
        ..r.subject.clone()
    };
    assert!(matches!(
        materialize(&r.path(), &subject, &digest(&subject.to_js_value())),
        Err(Refusal::PatchIdentityMismatch { .. })
    ));
}

#[test]
fn merge_and_root_commits_are_refused() {
    let r = repo();
    let p = r.path();
    // Root commit: the baseline has no parent.
    let root = Subject {
        commit: r.parent.clone(),
        tree: r.parent_tree(),
        patch_identity: "0".repeat(64),
    };
    assert!(matches!(
        materialize(&p, &root, &digest(&root.to_js_value())),
        Err(Refusal::ParentShape(_))
    ));
    // Merge commit.
    git(&p, &["checkout", "--quiet", "-b", "side", &r.parent]);
    std::fs::write(p.join("side.txt"), "side\n").unwrap();
    git(&p, &["add", "side.txt"]);
    git(&p, &["commit", "--quiet", "-m", "side"]);
    git(&p, &["merge", "--quiet", "--no-edit", &r.subject.commit]);
    let merge = Subject {
        commit: git(&p, &["rev-parse", "HEAD"]),
        tree: git(&p, &["rev-parse", "HEAD^{tree}"]),
        patch_identity: "0".repeat(64),
    };
    assert!(matches!(
        materialize(&p, &merge, &digest(&merge.to_js_value())),
        Err(Refusal::ParentShape(_))
    ));
}

#[test]
fn oversized_parent_listing_still_refuses_invalid_parent_shape() {
    let r = repo();
    let p = r.path();
    let mut args = vec!["commit-tree".to_owned(), r.subject.tree.clone()];
    // 100 distinct parents exceed the bounded rev-list metadata capture.
    for i in 0..100 {
        let parent = git(&p, &["commit-tree", &r.subject.tree, "-m", &i.to_string()]);
        args.extend(["-p".to_owned(), parent]);
    }
    args.extend(["-m".to_owned(), "many parents".to_owned()]);
    let args: Vec<_> = args.iter().map(String::as_str).collect();
    let subject = Subject {
        commit: git(&p, &args),
        tree: r.subject.tree,
        patch_identity: "0".repeat(64),
    };
    assert!(matches!(
        materialize(&p, &subject, &digest(&subject.to_js_value())),
        Err(Refusal::ParentShape(commit)) if commit == subject.commit
    ));
}

#[test]
fn binary_and_non_utf8_members_are_refused() {
    for bytes in [&b"a\0b\n"[..], &[0xff, 0xfe, b'\n'][..]] {
        let r = repo_with(&[("bin.dat", bytes)]);
        let refused = materialize(&r.path(), &r.subject, &digest(&r.subject.to_js_value()));
        assert!(
            matches!(&refused, Err(Refusal::Member { path, .. }) if path == "bin.dat"),
            "{refused:?}"
        );
    }
}

#[test]
fn oversize_snapshot_is_refused() {
    let big = "line of text\n".repeat(90_000); // ~1.17 MB, over 1 MiB with the patch
    let r = repo_with(&[("big.txt", big.as_bytes())]);
    assert!(matches!(
        materialize(&r.path(), &r.subject, &digest(&r.subject.to_js_value())),
        Err(Refusal::SnapshotBound(why)) if why.starts_with("git diff-tree output exceeds")
    ));
    // Member count bound.
    let many: BTreeMap<String, Vec<u8>> = (0..257)
        .map(|i| (format!("f{i}.txt"), b"x\n".to_vec()))
        .collect();
    assert!(matches!(
        check_members(&Snapshot {
            parent: String::new(),
            members: many
        }),
        Err(Refusal::SnapshotBound(_))
    ));
    // Path length bound.
    let long: BTreeMap<String, Vec<u8>> = [("p".repeat(257), b"x\n".to_vec())].into();
    assert!(matches!(
        check_members(&Snapshot {
            parent: String::new(),
            members: long
        }),
        Err(Refusal::Member { .. })
    ));
}

#[test]
fn materialization_reserves_two_slots_at_the_snapshot_member_limit() {
    // repo_with also modifies a.txt. Its deleted c.txt has no member slot.
    let files: Vec<_> = (0..254).map(|i| format!("f{i:03}.txt")).collect();
    let files: Vec<_> = files.iter().map(|p| (p.as_str(), &b"x\n"[..])).collect();
    let at_limit = repo_with(&files[..253]);
    let scope = digest(&at_limit.subject.to_js_value());
    let snapshot = materialize(&at_limit.path(), &at_limit.subject, &scope).unwrap();
    assert_eq!(snapshot.members.len(), 256);
    assert!(snapshot.members.contains_key(REVIEW_SUBJECT));
    assert!(snapshot.members.contains_key(REVIEW_PATCH));

    let over_limit = repo_with(&files);
    let scope = digest(&over_limit.subject.to_js_value());
    assert!(matches!(
        materialize(&over_limit.path(), &over_limit.subject, &scope),
        Err(Refusal::SnapshotBound(why)) if why.starts_with("more than 256 members")
    ));
}

#[test]
fn materialization_bounds_cumulative_blob_capture_even_with_a_small_patch() {
    let content = b"plain text\n".repeat(50_000);
    let r = repo_with(&[("first.txt", &content), ("second.txt", &content)]);
    let p = r.path();
    let baseline = r.subject.commit;
    let materialize_head = || {
        let tree = git(&p, &["rev-parse", "HEAD^{tree}"]);
        let patch = patch_bytes(&p, &baseline, &tree).unwrap();
        assert!(patch.len() < 500, "only executable bits changed");
        let subject = Subject {
            commit: git(&p, &["rev-parse", "HEAD"]),
            tree,
            patch_identity: sha256_hex(&patch),
        };
        materialize(&p, &subject, &digest(&subject.to_js_value()))
    };

    git(&p, &["update-index", "--chmod=+x", "first.txt"]);
    git(&p, &["commit", "--quiet", "-m", "first mode"]);
    assert_eq!(materialize_head().unwrap().members["first.txt"], content);

    git(&p, &["reset", "--soft", &baseline]);
    git(&p, &["update-index", "--chmod=+x", "second.txt"]);
    git(&p, &["commit", "--quiet", "-m", "both modes"]);
    assert!(matches!(
        materialize_head(),
        Err(Refusal::SnapshotBound(why)) if why.starts_with("git cat-file output exceeds")
    ));
}

#[test]
fn request_context_bound_is_snapshot_times_one_and_a_quarter_plus_task() {
    assert!(check_context_fit(320_000, 0).is_ok());
    assert!(matches!(
        check_context_fit(320_001, 0),
        Err(Refusal::ContextBound { .. })
    ));
    assert!(check_context_fit(240_000, 100_000).is_ok());
    assert!(matches!(
        check_context_fit(240_000, 100_001),
        Err(Refusal::ContextBound { .. })
    ));
}

/// One synthetic hit per rule: (rule, file content). Built at run time so this source file holds
/// no literal that a scanner of this repository would flag.
fn samples() -> Vec<(&'static str, String)> {
    let dash = "-----";
    vec![
        (
            secrets::PEM_PRIVATE_KEY,
            format!("{dash}BEGIN RSA PRIVATE KEY{dash}\nMIIB\n"),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            format!(
                "let k = \"{}{}\";\n",
                "gh", "p_0123456789abcdefghijABCDEFGHIJ0123"
            ),
        ),
        (
            secrets::SECRET_ASSIGNMENT,
            format!("db_{} = \"{}\"\n", "password", "Tr0ub4dor-3x9q"),
        ),
        (
            secrets::PRIVATE_PATH,
            format!("see {}/{}/agent/auth.json\n", "~", ".pi"),
        ),
        (
            secrets::EMAIL_ADDRESS,
            format!("owner: {}@{}\n", "someone", "example.org"),
        ),
    ]
}

#[test]
fn each_secret_rule_refuses_with_rule_and_path_and_no_content() {
    let samples = samples();
    assert_eq!(
        samples.iter().map(|s| s.0).collect::<Vec<_>>(),
        RULE_IDS[..5]
    );
    for (rule, content) in samples {
        let r = repo_with(&[("notes/leak.txt", content.as_bytes())]);
        let refused = materialize(&r.path(), &r.subject, &digest(&r.subject.to_js_value()));
        let Err(Refusal::Secret(hits)) = &refused else {
            panic!("{rule}: expected a secret refusal, got {refused:?}");
        };
        assert!(
            hits.iter()
                .any(|h| h.rule == rule && h.path == "notes/leak.txt"),
            "{rule}: {hits:?}"
        );
        assert!(
            hits.iter()
                .any(|h| h.rule == rule && h.path == REVIEW_PATCH),
            "{rule}: {hits:?}"
        );
        let shown = refused.unwrap_err().to_string();
        let secret_line = content.lines().next().unwrap();
        assert!(
            !shown.contains(secret_line),
            "{rule}: refusal echoes content"
        );
    }
}

/// Remediation 1 (rule set v2): one positive per added alternative, via `scan`. Literals are
/// assembled at run time (see `samples`).
#[test]
fn v2_alternatives_each_hit() {
    let j = |parts: &[&str]| parts.concat();
    let cases: Vec<(&str, String)> = vec![
        // (b) token prefixes
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["eyJ", "hbGciOiJIUzI1NiJ9", ".", "eyJ", "zdWIiOiIxIn0"]),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["sk", "_live_", "4eC39HqLyjWDarjtT1zdp7dc"]),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["rk", "_live_", "4eC39HqLyjWDarjtT1zdp7dc"]),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["AS", "IA", "ABCDEFGHIJKLMNOP"]),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["hf", "_", "abcdefghijklmnopqrstuvwxyzABCDEF12"]),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["xa", "pp-", "1-A0123456789-abcdef"]),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["xo", "xe.", "xoxp-1-abcdefghijkl"]),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["AGE-SECRET", "-KEY-1", "QQQQQQQQQQQQQQQQQQQQQQQQQQ"]),
        ),
        (
            secrets::CREDENTIAL_TOKEN_PREFIX,
            j(&["PuTTY-User", "-Key-File-", "3: ssh-ed25519"]),
        ),
        // (a) assignment forms
        (
            secrets::SECRET_ASSIGNMENT,
            j(&["const API_", "KEY: &str = \"", "q8Zr2mXw7Lp4Tn", "\";"]),
        ),
        (
            secrets::SECRET_ASSIGNMENT,
            j(&["DB_PASS", "WORD=", "q8Zr2mXw7Lp4Tn9k"]),
        ),
        (
            secrets::SECRET_ASSIGNMENT,
            j(&["export GH_TO", "KEN=", "q8Zr2mXw7Lp4Tn9k"]),
        ),
        (
            secrets::SECRET_ASSIGNMENT,
            j(&["  pass", "word: ", "q8Zr2mXw7Lp4Tn9k"]),
        ),
        (
            secrets::SECRET_ASSIGNMENT,
            j(&["secret = \"", "correct horse battery staple", "\""]),
        ),
        (
            secrets::SECRET_ASSIGNMENT,
            j(&[".header(\"x-api-", "key\", \"", "q8Zr2mXw7Lp4Tn", "\")"]),
        ),
        (
            secrets::SECRET_ASSIGNMENT,
            j(&["Authorization: Bea", "rer ", "q8Zr2mXw7Lp4Tn9kVb3c"]),
        ),
        // (d) private paths
        (
            secrets::PRIVATE_PATH,
            j(&["/var/lib", "/work-engine/state.db"]),
        ),
        (secrets::PRIVATE_PATH, j(&["~/", ".codex-api/auth.json"])),
        (secrets::PRIVATE_PATH, j(&["~/", ".claude.json"])),
        (secrets::PRIVATE_PATH, j(&["it lives in ~/", ".pi."])),
        // (e)
        (secrets::BINARY_PATCH, "GIT binary patch".to_owned()),
    ];
    for (rule, line) in &cases {
        let hits = secrets::scan("x.txt", line);
        assert!(
            hits.iter().any(|h| h.rule == *rule),
            "{rule}: {line} gave {hits:?}"
        );
    }
    // (c) member paths
    for path in [
        ".env",
        ".env.local",
        "deploy/server.pem",
        "keys/tls.key",
        "a/cert.p12",
        "id_rsa",
        "home/id_ed25519.pub",
        ".netrc",
        "pkg/.npmrc",
        "agent/auth.json",
        "aws/credentials",
        "CREDENTIALS.txt",
    ] {
        let hits = secrets::scan(path, "plain\n");
        assert_eq!(
            hits,
            [secrets::SecretHit {
                rule: secrets::SENSITIVE_MEMBER_PATH.into(),
                path: path.into(),
                line: 0
            }],
            "{path}"
        );
    }
    assert!(secrets::scan("src/keys.rs", "plain\n").is_empty());
}

#[test]
fn sensitive_member_and_deleted_binary_are_refused_by_materialize() {
    let r = repo_with(&[("config/.env", b"MODE=dev\n")]);
    let Err(Refusal::Secret(hits)) =
        materialize(&r.path(), &r.subject, &digest(&r.subject.to_js_value()))
    else {
        panic!("expected a member-path refusal");
    };
    assert!(
        hits.iter()
            .any(|h| h.rule == secrets::SENSITIVE_MEMBER_PATH && h.path == "config/.env")
    );
    let r = common::repo_deleting(&[("old.bin", b"\x00\x01binary\x00")], &[]);
    let Err(Refusal::Secret(hits)) =
        materialize(&r.path(), &r.subject, &digest(&r.subject.to_js_value()))
    else {
        panic!("expected a binary-patch refusal");
    };
    assert!(
        hits.iter()
            .any(|h| h.rule == secrets::BINARY_PATCH && h.path == REVIEW_PATCH),
        "{hits:?}"
    );
}

#[test]
fn unsafe_member_path_components_are_refused() {
    for path in ["../x", "a//b", "a/./b", ".git/config", "a/.git/b"] {
        let members: BTreeMap<String, Vec<u8>> = [(path.to_owned(), b"x\n".to_vec())].into();
        assert!(
            matches!(
                check_members(&Snapshot {
                    parent: String::new(),
                    members
                }),
                Err(Refusal::Member { .. })
            ),
            "{path}"
        );
    }
}

#[test]
fn read_budget_counts_snapshot_list_and_patch_pages() {
    let snap = |patch: usize| Snapshot {
        parent: String::new(),
        members: [
            (REVIEW_PATCH.to_owned(), vec![b'x'; patch]),
            ("a.txt".to_owned(), vec![b'y'; 30_000]),
        ]
        .into(),
    };
    // 24 576 × 3 bytes = 3 pages + snapshot_list = 4 reads.
    assert_eq!(check_read_budget(&snap(24_576 * 3), 4), Ok(3 + 2));
    assert!(matches!(
        check_read_budget(&snap(24_576 * 3 + 1), 4),
        Err(Refusal::ReadBudget {
            needed: 5,
            max_reads: 4
        })
    ));
}

#[test]
fn rule_negatives_stay_quiet() {
    for line in [
        "token: String,",
        "let api_key: Option<String> = None;",
        "let token = parse_token(&input)?;",
        "password: Option<String>,",
        "api_key = \"OPENAI_API_KEY\"",
        "password = \"changeme-later\"",
        "secret = \"${SECRET_VALUE}\"",
        "task-0123456789abcdefghijklmnop",
        "image node@sha256:4b150b48",
        "path ~/.pinned/file and /home/user/.pilot",
        "import from \"@earendil-works/pi-coding-agent\"",
    ] {
        assert!(secrets::scan("x", line).is_empty(), "{line}");
    }
}
