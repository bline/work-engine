//! Explicit test-owner qualification. These selections never confer installation authority.
#![cfg(feature = "test-faults")]
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use workspace_publication::{
    AuthorityMode, Workspace, WorkspaceConfig, codec,
    custody::{CustodySelection, FileIdentity, ManagedCustody},
};
struct Fixture {
    temp: tempfile::TempDir,
    policy: CustodySelection,
    path: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = PathBuf::from(
            std::env::var("WORK_ENGINE_W1_TEST_ROOT").expect("persistent test root required"),
        );
        fs::create_dir_all(&base).unwrap();
        let temp = tempfile::Builder::new()
            .prefix("c")
            .tempdir_in(base)
            .unwrap();
        let root = temp.path().join("w");
        let repo = temp.path().join("r");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(
            Command::new("/usr/bin/git")
                .args(["init", "--bare", "-q"])
                .arg(&repo)
                .status()
                .unwrap()
                .success()
        );
        fs::write(repo.join(".work-engine-publication.lock"), b"lock sentinel").unwrap();
        fs::set_permissions(
            repo.join(".work-engine-publication.lock"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::set_permissions(&repo, fs::Permissions::from_mode(0o700)).unwrap();
        let policy = CustodySelection {
            schema_version: 1,
            epoch: "fixture-epoch-1".into(),
            phase: "initialize".into(),
            runtime_uid: unsafe { libc::geteuid() },
            runtime_sha256: codec::raw_sha256(&fs::read(std::env::current_exe().unwrap()).unwrap()),
            root_identity: FileIdentity::observe(&root).unwrap(),
            repository_identity: FileIdentity::observe(&repo).unwrap(),
            lock_identity: FileIdentity::observe(&repo.join(".work-engine-publication.lock"))
                .unwrap(),
            marker_identity: None,
            ledger_identity: None,
            root,
            repository: repo,
            canonical_refs: vec!["refs/heads/main".into()],
            validation_program: vec!["/usr/bin/true".into()],
            validation_program_sha256: codec::raw_sha256(&fs::read("/usr/bin/true").unwrap()),
            validation_policy_digest: "b".repeat(64),
            timeout_seconds: 3,
        };
        let path = temp.path().join("p");
        let mut f = Self { temp, policy, path };
        f.save();
        let workspace = Workspace::initialize_managed(f.handle()).unwrap();
        drop(workspace);
        f.policy.phase = "active".into();
        f.policy.marker_identity =
            Some(FileIdentity::observe(&f.policy.root.join(".workspace-publication-v1")).unwrap());
        f.policy.ledger_identity = Some(
            FileIdentity::observe(&f.policy.root.join("workspace-publication.sqlite")).unwrap(),
        );
        f.save();
        f
    }
    fn save(&self) {
        fs::write(&self.path, serde_json::to_vec(&self.policy).unwrap()).unwrap();
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn handle(&self) -> ManagedCustody {
        ManagedCustody::load_fixture_selection(&self.path).unwrap()
    }
    fn source(&self) -> PathBuf {
        let p = self.policy.root.join("inputs/job");
        fs::create_dir_all(&p).unwrap();
        p
    }
}
#[test]
fn policy_data_and_test_build_cannot_grant_production_startup() {
    let f = Fixture::new();
    let c = WorkspaceConfig {
        root: f.temp.path().join("not-created"),
        repository: f.policy.repository.clone(),
        canonical_refs: f.policy.canonical_refs.clone(),
        validation_program: f.policy.validation_program.clone(),
        validation_policy_digest: f.policy.validation_policy_digest.clone(),
        authority_mode: AuthorityMode::Production,
    };
    assert!(Workspace::initialize(c.clone()).is_err());
    assert!(!c.root.exists());
    assert!(Workspace::open(c).is_err());
    assert!(ManagedCustody::load_installed().is_err());
    assert_eq!(f.handle().standing(), "fixture_managed_custody");
}
#[test]
fn actual_rust_launcher_denies_host_authority_and_descendants() {
    let f = Fixture::new();
    let source = f.source();
    fs::write(source.join("source.txt"), b"selected bytes").unwrap();
    let socket = f.temp.path().join("s");
    let _listener = UnixListener::bind(&socket).unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.policy.repository.join(".work-engine-publication.lock"))
        .unwrap();
    let dir = File::open(&f.policy.root).unwrap();
    let fds = [lock.as_raw_fd(), dir.as_raw_fd(), _listener.as_raw_fd()];
    for fd in fds {
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0);
    }
    use std::os::unix::fs::MetadataExt;
    let fd_identities: Vec<_> = fds
        .iter()
        .map(|fd| {
            let metadata = fs::metadata(format!("/proc/self/fd/{fd}")).unwrap();
            serde_json::json!([fd, metadata.dev(), metadata.ino()])
        })
        .collect();
    let selection = serde_json::json!({"paths":[f.policy.repository.join("refs/heads/escape"),f.policy.repository.join("worktrees/escape"),f.policy.repository.join(".work-engine-publication.lock"),f.policy.root.join("workspace-publication.sqlite"),f.path],"socket":socket,"fds":fd_identities,"hostPid":std::process::id()});
    let script = format!(
        r#"import os,socket,sys,subprocess,ctypes,errno,json
c=json.loads({selection:?})
def denied(fn):
 try:fn()
 except OSError:return
 raise AssertionError('host authority escaped')
for p in c['paths']:denied(lambda p=p:os.open(p,os.O_WRONLY|os.O_CREAT,0o600))
for fd,device,inode in c['fds']:
 try:s=os.fstat(fd)
 except OSError:continue
 assert (s.st_dev,s.st_ino)!=(device,inode),'protected host descriptor survived'
 print('descriptor slot reused:',fd,os.readlink('/proc/self/fd/'+str(fd)))
denied(lambda:socket.socket(socket.AF_UNIX).connect(c['socket']))
denied(lambda:os.open('/proc/'+str(c['hostPid'])+'/root',os.O_RDONLY))
denied(lambda:os.kill(c['hostPid'],0))
denied(lambda:os.open('/candidate/source.txt',os.O_WRONLY))
l=ctypes.CDLL(None,use_errno=True)
assert l.unshare(0x10000000)==-1 and ctypes.get_errno() in [errno.EPERM,errno.ENOSPC]
assert l.mount(None,b'/candidate',None,32,None)==-1
assert open('/candidate/source.txt').read()=='selected bytes'
open('/scratch/result','w').write('allowed')
if len(sys.argv)==1:subprocess.run([sys.executable,'-I','/candidate/canary.py','descendant'],check=True)
print('confined')
"#,
        selection = selection.to_string()
    );
    fs::write(source.join("canary.py"), script).unwrap();
    let before = fs::read(f.policy.repository.join(".work-engine-publication.lock")).unwrap();
    let output = f
        .handle()
        .run_untrusted(
            &source,
            &[
                "/usr/bin/python3".into(),
                "-I".into(),
                "/candidate/canary.py".into(),
            ],
        )
        .unwrap();
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .matches("confined")
            .count(),
        2
    );
    assert_eq!(
        before,
        fs::read(f.policy.repository.join(".work-engine-publication.lock")).unwrap()
    );
    assert!(!f.policy.repository.join("refs/heads/escape").exists());
    assert!(!f.policy.repository.join("worktrees/escape").exists());
}
#[test]
fn currentness_rejects_replacement_revocation_and_forged_identities() {
    let mut f = Fixture::new();
    let old = f.handle();
    f.policy.epoch = "fixture-epoch-2".into();
    f.save();
    assert!(
        old.run_untrusted(&f.source(), &["/usr/bin/true".into()])
            .is_err()
    );
    assert!(Workspace::open_managed(f.handle()).is_ok());
    f.policy.ledger_identity.as_mut().unwrap().inode += 1;
    f.save();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
    f.policy.ledger_identity.as_mut().unwrap().inode -= 1;
    f.save();
    let old = f.handle();
    fs::remove_file(&f.path).unwrap();
    assert!(
        old.run_untrusted(&f.source(), &["/usr/bin/true".into()])
            .is_err()
    );
}
#[test]
fn replacement_files_and_lock_are_not_recaptured_on_reopen() {
    for name in ["workspace-publication.sqlite", ".workspace-publication-v1"] {
        let f = Fixture::new();
        let p = f.policy.root.join(name);
        let bytes = fs::read(&p).unwrap();
        fs::rename(&p, p.with_extension("old")).unwrap();
        fs::write(&p, bytes).unwrap();
        assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
    }
    let f = Fixture::new();
    let p = f.policy.repository.join(".work-engine-publication.lock");
    fs::rename(&p, p.with_extension("old")).unwrap();
    fs::write(&p, b"lock sentinel").unwrap();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
}
#[test]
fn wrong_runtime_validator_source_and_unactivated_selection_refuse() {
    let mut f = Fixture::new();
    f.policy.runtime_sha256 = "0".repeat(64);
    f.save();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
    f.policy.runtime_sha256 =
        codec::raw_sha256(&fs::read(std::env::current_exe().unwrap()).unwrap());
    f.policy.validation_program_sha256 = "0".repeat(64);
    f.save();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
    f.policy.validation_program_sha256 = codec::raw_sha256(&fs::read("/usr/bin/true").unwrap());
    f.save();
    assert!(
        f.handle()
            .run_untrusted(&f.policy.root, &["/usr/bin/true".into()])
            .is_err()
    );
    f.policy.phase = "initialize".into();
    f.policy.marker_identity = None;
    f.policy.ledger_identity = None;
    f.save();
    assert!(Workspace::open_managed(f.handle()).is_err());
}
#[test]
fn timeout_collects_confined_descendants() {
    let mut f = Fixture::new();
    f.policy.timeout_seconds = 1;
    f.save();
    let start = Instant::now();
    let result = f.handle().run_untrusted(
        &f.source(),
        &[
            "/usr/bin/python3".into(),
            "-c".into(),
            "import subprocess,time;subprocess.Popen(['/usr/bin/sleep','60']);time.sleep(60)"
                .into(),
        ],
    );
    assert!(result.is_err());
    assert!(start.elapsed().as_secs() < 8);
}

#[test]
fn source_socket_and_hard_link_are_rejected_before_exec() {
    let f = Fixture::new();
    let source = f.source();
    let socket = source.join("s");
    // Bind through a directory descriptor to stay under sockaddr_un's path bound.
    let dir = File::open(&source).unwrap();
    let short = format!("/proc/self/fd/{}/s", dir.as_raw_fd());
    let listener = UnixListener::bind(short).unwrap();
    assert!(
        f.handle()
            .run_untrusted(&source, &["/usr/bin/true".into()])
            .is_err()
    );
    drop(listener);
    fs::remove_file(socket).unwrap();
    fs::hard_link(
        f.policy.root.join("workspace-publication.sqlite"),
        source.join("ledger-link"),
    )
    .unwrap();
    assert!(
        f.handle()
            .run_untrusted(&source, &["/usr/bin/true".into()])
            .is_err()
    );
}
#[test]
fn replacement_selection_same_bytes_and_symlink_cannot_reuse_old_handle() {
    let f = Fixture::new();
    let old = f.handle();
    let bytes = fs::read(&f.path).unwrap();
    let original = f.path.with_extension("old");
    fs::rename(&f.path, &original).unwrap();
    fs::write(&f.path, &bytes).unwrap();
    fs::set_permissions(&f.path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        old.run_untrusted(&f.source(), &["/usr/bin/true".into()])
            .is_err()
    );
    assert!(Workspace::open_managed(f.handle()).is_ok());
    fs::remove_file(&f.path).unwrap();
    std::os::unix::fs::symlink(&original, &f.path).unwrap();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
}
#[test]
fn caller_selected_helper_fields_are_not_part_of_authority_contract() {
    let f = Fixture::new();
    let mut value = serde_json::to_value(&f.policy).unwrap();
    value["bwrapPath"] = "/candidate/replacement".into();
    value["bwrapSha256"] = "0".repeat(64).into();
    fs::write(&f.path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
}

#[test]
fn canonical_symlink_hard_link_and_alternate_topology_refuse() {
    let f = Fixture::new();
    let external = f.temp.path().join("external");
    fs::write(&external, b"outside").unwrap();
    let redirect = f.policy.repository.join("refs/escape");
    std::os::unix::fs::symlink(&external, &redirect).unwrap();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
    fs::remove_file(&redirect).unwrap();
    fs::hard_link(&external, &redirect).unwrap();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
    fs::remove_file(&redirect).unwrap();
    fs::write(
        f.policy.repository.join("objects/info/alternates"),
        b"/outside/objects\n",
    )
    .unwrap();
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
}

#[test]
fn managed_custody_refuses_loose_and_packed_replacement_objects() {
    let f = Fixture::new();
    fn blob(repo: &std::path::Path, text: &[u8]) -> String {
        use std::io::Write;
        use std::process::Stdio;
        let mut child = Command::new("/usr/bin/git")
            .arg("-C")
            .arg(repo)
            .args(["hash-object", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(text).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().trim().into()
    }
    let original = blob(&f.policy.repository, b"unchanged baseline blob\n");
    let replacement = blob(&f.policy.repository, b"substituted blob\n");
    assert!(
        Command::new("/usr/bin/git")
            .arg("-C")
            .arg(&f.policy.repository)
            .args([
                "update-ref",
                &format!("refs/replace/{original}"),
                &replacement
            ])
            .status()
            .unwrap()
            .success()
    );
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
    assert!(
        Command::new("/usr/bin/git")
            .arg("-C")
            .arg(&f.policy.repository)
            .args(["pack-refs", "--all", "--prune"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        !f.policy
            .repository
            .join(format!("refs/replace/{original}"))
            .exists()
    );
    assert!(
        fs::read_to_string(f.policy.repository.join("packed-refs"))
            .unwrap()
            .contains("refs/replace/")
    );
    assert!(ManagedCustody::load_fixture_selection(&f.path).is_err());
}
