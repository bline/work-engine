use std::{
    fs,
    os::unix::{fs::PermissionsExt as _, net::UnixStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use lifecycle_codex::{AccessIsolationConfig, NativeAuthMode, NativeHarness, NativeHarnessError};
use sha2::{Digest as _, Sha256};

#[test]
fn production_access_lease_requires_selected_isolation_before_spawn() {
    let Ok(binary) = std::env::var("S10_NATIVE_BINARY") else {
        return;
    };
    let result = NativeHarness::spawn_with_auth_and_proxy(
        Path::new(&binary),
        Path::new("/"),
        Path::new("/"),
        Path::new("/"),
        "production-isolation-check".into(),
        NativeAuthMode::ExternalChatgptAccess,
        Some("http://127.0.0.1:3128"),
    );
    assert!(matches!(
        result,
        Err(NativeHarnessError::IsolationUnavailable)
    ));
}

#[cfg(feature = "synthetic-access-test")]
#[test]
fn synthetic_access_lease_requires_isolation_in_normal_feature_build() {
    let Ok(binary) = std::env::var("S10_NATIVE_BINARY") else {
        return;
    };
    let temp = tempfile::tempdir().unwrap();
    let work = temp.path().join("work");
    fs::create_dir(&work).unwrap();
    let result = NativeHarness::spawn_with_auth_and_proxy(
        Path::new(&binary),
        &work,
        temp.path(),
        temp.path(),
        "synthetic-missing-isolation".into(),
        NativeAuthMode::SyntheticChatgptAccessTest,
        Some("http://127.0.0.1:3128"),
    );
    assert!(matches!(
        result,
        Err(NativeHarnessError::IsolationUnavailable)
    ));
    assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
    assert!(!temp.path().join("tmp").exists());
}

#[test]
fn historical_routes_reject_access_isolation_profile() {
    let Ok(binary) = std::env::var("S10_NATIVE_BINARY") else {
        return;
    };
    let profile = AccessIsolationConfig {
        helper_executable: "/missing/helper".into(),
        helper_sha256: "0".repeat(64),
        proxy_socket: "/missing/proxy.sock".into(),
        proxy_policy_executable: "/missing/proxy-policy".into(),
        proxy_policy_sha256: "0".repeat(64),
        source_home: "/missing/source".into(),
        #[cfg(feature = "synthetic-access-test")]
        synthetic_http_socket: None,
        #[cfg(feature = "synthetic-access-test")]
        synthetic_http_port: None,
        #[cfg(feature = "synthetic-access-test")]
        synthetic_http_fixture_executable: None,
        #[cfg(feature = "synthetic-access-test")]
        synthetic_http_fixture_sha256: None,
    };
    for mode in [NativeAuthMode::None, NativeAuthMode::RuntimeApiKey] {
        let result = NativeHarness::spawn_with_access_isolation(
            Path::new(&binary),
            Path::new("/"),
            Path::new("/"),
            Path::new("/"),
            "production-isolation-check".into(),
            mode,
            None,
            Some(&profile),
        );
        assert!(matches!(
            result,
            Err(NativeHarnessError::IsolationUnavailable)
        ));
    }
}

fn selected_proxy_fixture(
    mode_args: &[&str],
    policy_env: &str,
) -> Option<(
    tempfile::TempDir,
    String,
    PathBuf,
    Child,
    AccessIsolationConfig,
)> {
    let (Ok(binary), Ok(helper), Ok(policy)) = (
        std::env::var("S10_NATIVE_BINARY"),
        std::env::var("S10_ISOLATION_HELPER"),
        std::env::var(policy_env),
    ) else {
        return None;
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("launch");
    let source = temp.path().join("source");
    let proxy_dir = temp.path().join("proxy");
    for directory in [
        &root,
        &source,
        &proxy_dir,
        &root.join("user"),
        &root.join("codex"),
        &root.join("work"),
    ] {
        fs::create_dir(directory).unwrap();
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let socket = proxy_dir.join("selected.sock");
    fs::write(root.join("work/.context-lifecycle-project"), b"synthetic\n").unwrap();
    fs::write(
        root.join("codex/config.toml"),
        b"model = \"gpt-6-sol\"\ncheck_for_update_on_startup = false\n",
    )
    .unwrap();
    let policy = PathBuf::from(policy);
    let proxy = Command::new("/usr/bin/python3")
        .arg(&policy)
        .args([
            "--socket",
            socket.to_str().unwrap(),
            "--journal",
            temp.path().join("journal.jsonl").to_str().unwrap(),
        ])
        .args(mode_args)
        .args(["--max-seconds", "90"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while UnixStream::connect(&socket).is_err() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(UnixStream::connect(&socket).is_ok());
    let helper = PathBuf::from(helper);
    let policy_sha256 = format!("{:x}", Sha256::digest(fs::read(&policy).unwrap()));
    let profile = AccessIsolationConfig {
        helper_sha256: format!("{:x}", Sha256::digest(fs::read(&helper).unwrap())),
        helper_executable: helper,
        proxy_socket: socket,
        proxy_policy_executable: policy,
        proxy_policy_sha256: policy_sha256,
        source_home: source,
        #[cfg(feature = "synthetic-access-test")]
        synthetic_http_socket: None,
        #[cfg(feature = "synthetic-access-test")]
        synthetic_http_port: None,
        #[cfg(feature = "synthetic-access-test")]
        synthetic_http_fixture_executable: None,
        #[cfg(feature = "synthetic-access-test")]
        synthetic_http_fixture_sha256: None,
    };
    Some((temp, binary, root, proxy, profile))
}

#[test]
fn production_route_rejects_duplicate_real_then_effective_synthetic_mode() {
    let Some((_temp, binary, root, mut proxy, profile)) = selected_proxy_fixture(
        &[
            "--mode",
            "real",
            "--mode",
            "synthetic",
            "--test-target",
            "127.0.0.1:9",
        ],
        "S10_PROXY_POLICY_SCRIPT",
    ) else {
        return;
    };
    let result = NativeHarness::spawn_with_access_isolation(
        Path::new(&binary),
        &root.join("work"),
        &root.join("user"),
        &root.join("codex"),
        "production-duplicate-mode-check".into(),
        NativeAuthMode::ExternalChatgptAccess,
        Some("http://127.0.0.1:3128"),
        Some(&profile),
    );
    assert!(matches!(
        result,
        Err(NativeHarnessError::IsolationUnavailable)
    ));
    assert!(!root.join("tmp").exists());
    proxy.kill().unwrap();
    proxy.wait().unwrap();
}

#[cfg(feature = "synthetic-access-test")]
#[tokio::test]
async fn distinct_test_route_admits_only_selected_synthetic_proxy() {
    let Some((_temp, binary, root, mut proxy, mut profile)) = selected_proxy_fixture(
        &["--mode", "synthetic", "--test-target", "127.0.0.1:9"],
        "S10_SYNTHETIC_PROXY_POLICY_SCRIPT",
    ) else {
        return;
    };
    let production = NativeHarness::spawn_with_access_isolation(
        Path::new(&binary),
        &root.join("work"),
        &root.join("user"),
        &root.join("codex"),
        "production-synthetic-rejection".into(),
        NativeAuthMode::ExternalChatgptAccess,
        Some("http://127.0.0.1:3128"),
        Some(&profile),
    );
    assert!(matches!(
        production,
        Err(NativeHarnessError::IsolationUnavailable)
    ));
    assert!(!root.join("tmp").exists());
    let (Ok(synthetic_helper), Ok(fixture_script)) = (
        std::env::var("S10_SYNTHETIC_ISOLATION_HELPER"),
        std::env::var("S10_HTTP_FIXTURE_SCRIPT"),
    ) else {
        proxy.kill().unwrap();
        proxy.wait().unwrap();
        return;
    };
    let synthetic_helper = PathBuf::from(synthetic_helper);
    let fixture_script = PathBuf::from(fixture_script);
    let http_parent = root.parent().unwrap().join("http");
    fs::create_dir(&http_parent).unwrap();
    fs::set_permissions(&http_parent, fs::Permissions::from_mode(0o700)).unwrap();
    let http_socket = http_parent.join("account.sock");
    let mut fixture = Command::new("/usr/bin/python3")
        .arg(&fixture_script)
        .args([
            "--socket",
            http_socket.to_str().unwrap(),
            "--journal",
            http_parent.join("journal.jsonl").to_str().unwrap(),
            "--max-seconds",
            "90",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while UnixStream::connect(&http_socket).is_err() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(UnixStream::connect(&http_socket).is_ok());
    profile.helper_sha256 = format!("{:x}", Sha256::digest(fs::read(&synthetic_helper).unwrap()));
    profile.helper_executable = synthetic_helper;
    profile.synthetic_http_socket = Some(http_socket);
    profile.synthetic_http_port = Some(38472);
    profile.synthetic_http_fixture_executable = Some(fixture_script);
    profile.synthetic_http_fixture_sha256 =
        Some("601d5ecaa942eba075c65e610dfe362c5235ccde7b1c93e8d931dd588ec918da".into());
    let mut test = NativeHarness::spawn_with_access_isolation(
        Path::new(&binary),
        &root.join("work"),
        &root.join("user"),
        &root.join("codex"),
        "test-only-synthetic-admission".into(),
        NativeAuthMode::SyntheticChatgptAccessTest,
        Some("http://127.0.0.1:3128"),
        Some(&profile),
    )
    .unwrap();
    assert!(test.pid().is_some());
    assert!(!test.entry_allowed());
    let exit = test.terminate_and_reap().await.unwrap();
    assert!(exit.pid.is_some());
    proxy.kill().unwrap();
    proxy.wait().unwrap();
    fixture.kill().unwrap();
    fixture.wait().unwrap();
}
