#![cfg(unix)]
use std::process::Command;
#[cfg(all(feature = "test-support", feature = "test-peer"))]
mod support;

#[test]
fn completion_startup_cannot_override_installed_authority() {
    let host = env!("CARGO_BIN_EXE_work-engine-review-host");
    for operation in ["completion-bootstrap", "completion-serve"] {
        let output = Command::new(host)
            .args([
                operation,
                "--startup",
                "/missing",
                "--completion-selection",
                "/caller/authority.json",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unsupported host startup option")
        );
    }
}

#[cfg(all(feature = "test-support", feature = "test-peer"))]
mod cold_fixture {
    use super::*;

    #[test]
    fn cold_completion_reopens_exact_owners_without_launch_or_result_reissue() {
        let fixture = support::Fixture::new(0, "result");
        let initialized = fixture.initialize();
        let revision = initialized["campaign"]["revision"].as_str().unwrap();
        let mut review = fixture.start(&[]);
        review.ready();
        review.domain("execute_initial", revision);
        let result = review.receive();
        assert_eq!(
            result["result"]["outcome"]["status"], "committed",
            "{result}"
        );
        let receipt = &result["result"]["outcome"]["receipt"];
        review.finish();
        for _ in 0..2 {
            let output = Command::new(&fixture.host)
                .args(["completion-fixture-readback", "--startup"])
                .arg(&fixture.startup)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["standing"], "fixture_owner_readback_only");
            assert_eq!(&value["receipt"], receipt);
            assert_eq!(fixture.entries(), 1);
        }
        let denied = Command::new(&fixture.host)
            .args(["completion-serve", "--startup"])
            .arg(&fixture.startup)
            .output()
            .unwrap();
        assert!(!denied.status.success());
        assert!(
            String::from_utf8_lossy(&denied.stderr)
                .contains("test build cannot serve production completion")
        );
    }
}

#[cfg(all(feature = "test-support", feature = "test-peer"))]
#[test]
fn actual_reviewer_constructor_denies_operator_socket_to_descendants() {
    use review_execution_evidence::claude::{CandidateBoundary, PinnedProgram, TrustedRp0Profile};
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        os::unix::{
            fs::PermissionsExt,
            net::{UnixListener, UnixStream},
        },
        path::Path,
    };
    let digest = |p: &Path| format!("{:x}", Sha256::digest(fs::read(p).unwrap()));
    let base =
        std::env::var_os("REVIEW_SOCKET_TEST_ROOT").expect("explicit persistent fixture root");
    fs::create_dir_all(&base).unwrap();
    let temp = tempfile::tempdir_in(base).unwrap();
    let operator = temp.path().join("operator");
    fs::create_dir(&operator).unwrap();
    fs::set_permissions(&operator, fs::Permissions::from_mode(0o700)).unwrap();
    let socket = operator.join("completion.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    // Establish actual host reachability before testing the namespace denial.
    let host = UnixStream::connect(&socket).unwrap();
    drop(listener.accept().unwrap());
    drop(host);
    let candidate = temp.path().join("candidate");
    fs::create_dir(&candidate).unwrap();
    fs::write(candidate.join("lib.rs"), "pub fn socket_canary() {}\n").unwrap();
    let bindir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let bwrap = Path::new("/usr/bin/bwrap");
    let cbm = Path::new("/home/bline/.local/bin/codebase-memory-mcp");
    let launcher = temp.path().join("claude_cbm_restricted");
    let client = temp.path().join("claude_cbm_client");
    for (name, destination) in [
        ("claude_cbm_restricted", &launcher),
        ("claude_cbm_client", &client),
    ] {
        fs::copy(bindir.join(name), destination).unwrap();
        fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let profile = TrustedRp0Profile::load(
        PinnedProgram::load(bwrap, &digest(bwrap)).unwrap(),
        PinnedProgram::load(cbm, &digest(cbm)).unwrap(),
        PinnedProgram::load(&launcher, &digest(&launcher)).unwrap(),
        PinnedProgram::load(&client, &digest(&client)).unwrap(),
    )
    .unwrap();
    let boundary = CandidateBoundary::prepare(
        profile,
        &candidate,
        temp.path().join("boundary"),
        "socket_canary",
    )
    .unwrap();
    let (boundary, _) = boundary.publish_manifest().unwrap();
    let probe = temp.path().join("probe.py");
    let socket_json = serde_json::to_string(&socket).unwrap();
    let script = r#"import socket, subprocess, sys, json
path = SOCKET_PATH
code = "import socket; s=socket.socket(socket.AF_UNIX); s.connect(" + repr(path) + ")"
for descendant in [False, True]:
 if descendant:
  result=subprocess.run([sys.executable,'-I','-c',code],capture_output=True)
  assert result.returncode != 0 and b'FileNotFoundError' in result.stderr, result
 else:
  try: socket.socket(socket.AF_UNIX).connect(path)
  except FileNotFoundError: pass
  else: raise AssertionError('operator socket exposed')
 print('operator-socket-denied', file=sys.stderr)
p=subprocess.Popen(['/cbm/client'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
def rpc(value):
 p.stdin.write(json.dumps(value)+'\n'); p.stdin.flush()
 return json.loads(p.stdout.readline())
assert 'result' in rpc({'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'socket-canary','version':'1'}}})
p.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');p.stdin.flush()
assert 'result' in rpc({'jsonrpc':'2.0','id':2,'method':'tools/call','params':{'name':'index_status','arguments':{'project':'socket_canary','format':'json'}}})
p.stdin.close();p.wait(timeout=5)
print(json.dumps({'operator_socket_denied':True,'descendant_denied':True,'standing':'fixture_reviewer_boundary_only','provider_calls':0}))
"#.replace("SOCKET_PATH", &socket_json);
    fs::write(&probe, script).unwrap();
    let output = boundary.run_hostile_probe(&probe).unwrap();
    assert_eq!(
        output.exit_code,
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr)
            .matches("operator-socket-denied")
            .count(),
        2
    );
    let observed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(observed["operator_socket_denied"], true);
    assert_eq!(observed["descendant_denied"], true);
    println!("{observed}");
    // Listener is still reachable on the host; denial came from confinement.
    let _host = UnixStream::connect(&socket).unwrap();
    drop(listener.accept().unwrap());
}
