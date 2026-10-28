use bounded_inference::{
    AuthError, AuthStore, Issuer, Readiness, RefreshFailure, RevocationOutcome, TokenResponse,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const JWKS: &str = include_str!("fixtures/synthetic-siwc-jwks.json");
const KEY: &[u8] = include_bytes!("fixtures/synthetic-siwc-rsa.pkcs1.der");
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
const NOW: u64 = 1_800_000_000;
fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "b2a-synthetic-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    p
}
fn issuer() -> Issuer {
    Issuer::new(
        "https://fake-issuer.invalid",
        "https://fake-issuer.invalid/authorize",
        "https://fake-issuer.invalid/token",
        "https://fake-issuer.invalid/revoke",
    )
    .unwrap()
}
fn store(p: &Path) -> AuthStore {
    AuthStore::open(p, "synthetic_host_1", issuer()).unwrap()
}
#[test]
fn opaque_host_uri_round_trips_and_keeps_client_ids_separate() {
    const HOST: &str = "urn:uuid:f5293ea4-b022-4721-8de5-d1bcdcbab966";
    let p = root();
    let s = AuthStore::open(&p, HOST, issuer()).unwrap();
    let start = s
        .begin_registration("Synthetic Tool", "http://127.0.0.1:45123/auth/callback", NOW)
        .unwrap();
    assert_eq!(param(start.authorize_url(), "ext_agent_host_id"), HOST);
    assert_eq!(param(start.authorize_url(), "client_id"), "dynamic_agent_client");
    assert!(matches!(
        AuthStore::open(&p, "urn:uuid:another-host", issuer()),
        Err(AuthError::UnsafeStore)
    ));
    drop(s);
    let reopened = AuthStore::open(&p, HOST, issuer()).unwrap();
    assert!(reopened.saved_registrations().unwrap().is_empty());
    let state = param(start.authorize_url(), "state");
    assert_eq!(
        reopened
            .accept_callback(
                &format!("http://127.0.0.1:45123/auth/callback?code=x&state={state}&client_id=urn:uuid:client"),
                NOW,
            )
            .unwrap_err(),
        AuthError::Client
    );
    fs::remove_dir_all(p).unwrap();
}

#[test]
fn host_uri_validation_rejects_hostile_input_and_malformed_persisted_state() {
    let p = root();
    for host in [
        "",
        "urn:uuid:",
        "did:key:",
        "urn:uuid:../escape",
        "urn:uuid:evil\nheader",
        "urn:uuid:evil\u{7f}",
        "urn:uuid:evil%0aheader",
        "https://unexpected.invalid/host",
    ] {
        assert!(matches!(
            AuthStore::open(&p, host, issuer()),
            Err(AuthError::InvalidInput)
        ), "unexpectedly accepted host {host:?}");
    }
    assert!(matches!(
        AuthStore::open(&p, &format!("urn:uuid:{}", "x".repeat(257)), issuer()),
        Err(AuthError::InvalidInput)
    ));
    assert!(!p.exists());

    for host in ["urn:ietf:params:oauth:jwk-thumbprint:sha-256:synthetic", "did:key:zSyntheticKey"] {
        let s = AuthStore::open(&p, host, issuer()).unwrap();
        drop(s);
        assert!(AuthStore::open(&p, host, issuer()).is_ok());
        fs::remove_dir_all(&p).unwrap();
    }

    let s = AuthStore::open(&p, "urn:uuid:synthetic", issuer()).unwrap();
    drop(s);
    let state_path = p.join("state.json");
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["host_id"] = json!("urn:uuid:evil\nheader");
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert!(matches!(
        AuthStore::open(&p, "urn:uuid:evil\nheader", issuer()),
        Err(AuthError::InvalidInput)
    ));
    assert!(matches!(
        AuthStore::open(&p, "urn:uuid:synthetic", issuer()),
        Err(AuthError::Store)
    ));
    fs::remove_dir_all(p).unwrap();
}
fn param(url: &str, key: &str) -> String {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == key)
        .unwrap()
        .1
        .to_string()
}
fn signed(client: &str, nonce: Option<&str>, sub: &str, edit: impl FnOnce(&mut Value)) -> String {
    let mut claims = json!({"iss":"https://fake-issuer.invalid","aud":client,"sub":sub,"exp":NOW+3600,"iat":NOW-1,"email":"same@example.invalid"});
    if let Some(n) = nonce {
        claims["nonce"] = json!(n);
    }
    edit(&mut claims);
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some("synthetic-key-1".into());
    encode(&h, &claims, &EncodingKey::from_rsa_der(KEY)).unwrap()
}
fn response(id: &str, access: &str, refresh: &str, scope: &str) -> TokenResponse {
    TokenResponse::from_json(serde_json::to_string(&json!({"access_token":access,"refresh_token":refresh,"id_token":id,"token_type":"Bearer","expires_in":3600,"scope":scope,"earliest_refresh_at":"opaque-uninterpreted"})).unwrap().as_bytes()).unwrap()
}
fn begin(s: &AuthStore, client: &str, sub: &str) -> bounded_inference::RegistrationRef {
    let start = s
        .begin_registration(
            "Synthetic Tool",
            "http://127.0.0.1:45123/auth/callback",
            NOW,
        )
        .unwrap();
    let state = param(start.authorize_url(), "state");
    let nonce = param(start.authorize_url(), "nonce");
    let grant=s.accept_callback(&format!("http://127.0.0.1:45123/auth/callback?code=synthetic-code&state={state}&client_id={client}"),NOW).unwrap();
    s.finish_exchange(
        &grant,
        response(
            &signed(client, Some(&nonce), sub, |_| {}),
            "synthetic-access",
            "synthetic-refresh",
            SCOPE,
        ),
        JWKS,
        NOW,
    )
    .unwrap()
}
#[test]
fn signed_registration_refresh_switch_and_revocation() {
    let p = root();
    let s = store(&p);
    let r1 = begin(&s, "client_one", "subject_one");
    assert!(matches!(s.readiness(NOW).unwrap(), Readiness::Ready(_)));
    let bearer = s.bearer(&r1, NOW).unwrap();
    assert_eq!(bearer.authorization_value(), "Bearer synthetic-access");
    assert!(!format!("{bearer:?}").contains("synthetic-access"));
    let r2 = s
        .refresh_with(&r1, NOW, JWKS, |g| {
            assert_eq!(g.client_id, "client_one");
            assert_eq!(g.refresh_token(), "synthetic-refresh");
            Ok(response(
                &signed("client_one", None, "subject_one", |_| {}),
                "access-two",
                "refresh-two",
                SCOPE,
            ))
        })
        .unwrap();
    let reopened = store(&p);
    assert_eq!(
        reopened.bearer(&r2, NOW).unwrap().authorization_value(),
        "Bearer access-two"
    );
    assert_eq!(s.bearer(&r1, NOW).unwrap_err(), AuthError::StaleRevision);
    assert_eq!(
        s.bearer(&r2, NOW).unwrap().authorization_value(),
        "Bearer access-two"
    );
    let start = s
        .begin_registration(
            "Synthetic Tool",
            "http://127.0.0.1:45123/auth/callback",
            NOW,
        )
        .unwrap();
    let state = param(start.authorize_url(), "state");
    let nonce = param(start.authorize_url(), "nonce");
    let grant=s.accept_callback(&format!("http://127.0.0.1:45123/auth/callback?code=second&state={state}&client_id=client_two"),NOW).unwrap();
    let other = s
        .finish_exchange(
            &grant,
            response(
                &signed("client_two", Some(&nonce), "subject_two", |_| {}),
                "other-access",
                "other-refresh",
                SCOPE,
            ),
            JWKS,
            NOW,
        )
        .unwrap();
    assert_eq!(s.bearer(&r2, NOW).unwrap_err(), AuthError::StaleRevision);
    assert_eq!(
        s.refresh_with(&r2, NOW, JWKS, |_| panic!("switch must block stale writer"))
            .unwrap_err(),
        AuthError::StaleRevision
    );
    s.switch(&r2).unwrap();
    assert_eq!(
        s.bearer(&r2, NOW).unwrap().authorization_value(),
        "Bearer access-two"
    );
    assert_eq!(
        s.sign_out_with(&r2, |grant| {
            assert_eq!(
                (grant.client_id.as_str(), grant.refresh_token()),
                ("client_one", "refresh-two")
            );
            assert_eq!(grant.endpoint, "https://fake-issuer.invalid/revoke");
            false
        })
        .unwrap(),
        RevocationOutcome::Unconfirmed
    );
    assert!(matches!(s.readiness(NOW).unwrap(), Readiness::SignedOut));
    s.switch(&other).unwrap();
    assert_eq!(
        s.bearer(&other, NOW).unwrap().authorization_value(),
        "Bearer other-access"
    );
    let disk = fs::read(p.join("state.json")).unwrap();
    let text = String::from_utf8(disk).unwrap();
    assert!(!text.contains("refresh-two"));
    assert!(!format!("{:?}", AuthError::Identity).contains("synthetic"));
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn callback_and_signature_failures_are_bounded() {
    let p = root();
    let s = store(&p);
    let start = s
        .begin_registration(
            "Synthetic Tool",
            "http://127.0.0.1:45123/auth/callback",
            NOW,
        )
        .unwrap();
    let state = param(start.authorize_url(), "state");
    let nonce = param(start.authorize_url(), "nonce");
    for url in [
        format!("http://127.0.0.1:45124/auth/callback?code=x&state={state}&client_id=client_one"),
        format!("http://127.0.0.1:45123/callback?code=x&state={state}&client_id=client_one"),
        "http://127.0.0.1:45123/auth/callback?code=x&state=bad&client_id=client_one".into(),
    ] {
        assert_eq!(
            s.accept_callback(&url, NOW).unwrap_err(),
            AuthError::Callback
        );
    }
    assert_eq!(
        s.accept_callback(
            &format!("http://127.0.0.1:45123/auth/callback?code=x&state={state}"),
            NOW
        )
        .unwrap_err(),
        AuthError::Client
    );
    let callback =
        format!("http://127.0.0.1:45123/auth/callback?code=x&state={state}&client_id=client_one");
    let grant = s.accept_callback(&callback, NOW).unwrap();
    assert_eq!(
        s.accept_callback(&callback, NOW).unwrap_err(),
        AuthError::StaleAttempt
    );
    for modify in [
        ("nonce", json!("wrong")),
        ("iss", json!("https://evil.invalid")),
        ("aud", json!("client_two")),
        ("aud", json!(["client_one", "client_two"])),
        ("sub", json!("")),
        ("exp", json!(NOW - 1)),
        ("nbf", json!("wrong-type")),
        ("nbf", json!(NOW + 100)),
    ] {
        let t = signed("client_one", Some(&nonce), "subject", |c| {
            c[modify.0] = modify.1
        });
        assert_eq!(
            s.finish_exchange(&grant, response(&t, "a", "r", SCOPE), JWKS, NOW)
                .unwrap_err(),
            AuthError::Identity,
            "{}",
            modify.0
        );
    }
    let valid = signed("client_one", Some(&nonce), "subject", |_| {});
    let (prefix, signature) = valid.rsplit_once('.').unwrap();
    let mut bad = format!(
        "{prefix}.{}",
        if signature.starts_with('A') { "B" } else { "A" }
    );
    bad.push_str(&signature[1..]);
    assert_eq!(
        s.finish_exchange(&grant, response(&bad, "a", "r", SCOPE), JWKS, NOW)
            .unwrap_err(),
        AuthError::Identity
    );
    let wrong_key = JWKS.replace("synthetic-key-1", "different-key");
    assert_eq!(
        s.finish_exchange(
            &grant,
            response(
                &signed("client_one", Some(&nonce), "subject", |_| {}),
                "a",
                "r",
                SCOPE
            ),
            &wrong_key,
            NOW
        )
        .unwrap_err(),
        AuthError::Identity
    );
    let mut h = Header::new(Algorithm::HS256);
    h.kid = Some("synthetic-key-1".into());
    let wrong_algorithm=encode(&h,&json!({"iss":"https://fake-issuer.invalid","aud":"client_one","sub":"subject","exp":NOW+3600,"nonce":nonce}),&EncodingKey::from_secret(b"synthetic-hmac")).unwrap();
    assert_eq!(
        s.finish_exchange(
            &grant,
            response(&wrong_algorithm, "a", "r", SCOPE),
            JWKS,
            NOW
        )
        .unwrap_err(),
        AuthError::Identity
    );
    assert_eq!(
        s.finish_exchange(
            &grant,
            response(
                &signed("client_one", Some(&nonce), "subject", |_| {}),
                "a",
                "r",
                "openid profile email offline_access resource.invoke"
            ),
            JWKS,
            NOW
        )
        .unwrap_err(),
        AuthError::Scope
    );
    assert_eq!(
        s.finish_exchange(
            &grant,
            response(
                &signed("client_one", Some(&nonce), "subject", |_| {}),
                "a",
                "r",
                SCOPE
            ),
            JWKS,
            NOW
        )
        .unwrap()
        .subject,
        "subject"
    );
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn returning_callback_may_omit_client_but_cannot_substitute_it() {
    let p = root();
    let s = store(&p);
    let r = begin(&s, "client_one", "subject");
    let start = s
        .begin_returning(&r, "http://127.0.0.1:54321/auth/callback", NOW)
        .unwrap();
    assert_eq!(param(start.authorize_url(), "client_id"), "client_one");
    assert!(start.authorize_url().contains("id_token_hint="));
    assert!(!format!("{start:?}").contains("id_token_hint"));
    let state = param(start.authorize_url(), "state");
    let nonce = param(start.authorize_url(), "nonce");
    assert_eq!(
        s.accept_callback(
            &format!(
                "http://127.0.0.1:54321/auth/callback?code=x&state={state}&client_id=client_two"
            ),
            NOW
        )
        .unwrap_err(),
        AuthError::Client
    );
    let grant = s
        .accept_callback(
            &format!("http://127.0.0.1:54321/auth/callback?code=x&state={state}"),
            NOW,
        )
        .unwrap();
    assert_eq!(grant.client_id(), "client_one");
    assert_eq!(
        s.finish_exchange(
            &grant,
            response(
                &signed("client_one", Some(&nonce), "other-sub", |_| {}),
                "a",
                "r",
                SCOPE
            ),
            JWKS,
            NOW
        )
        .unwrap_err(),
        AuthError::Identity
    );
    let r2 = s
        .finish_exchange(
            &grant,
            response(
                &signed("client_one", Some(&nonce), "subject", |_| {}),
                "a",
                "r",
                SCOPE,
            ),
            JWKS,
            NOW,
        )
        .unwrap();
    assert!(r2.credential_revision > r.credential_revision);
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn account_switch_invalidates_pending_authorization_publication() {
    let p = root();
    let s = store(&p);
    let r = begin(&s, "client_one", "subject");
    let start = s
        .begin_returning(&r, "http://127.0.0.1:54321/auth/callback", NOW)
        .unwrap();
    let state = param(start.authorize_url(), "state");
    let nonce = param(start.authorize_url(), "nonce");
    s.switch(&r).unwrap();
    let grant = s
        .accept_callback(
            &format!("http://127.0.0.1:54321/auth/callback?code=x&state={state}"),
            NOW,
        )
        .unwrap();
    assert_eq!(
        s.finish_exchange(
            &grant,
            response(
                &signed("client_one", Some(&nonce), "subject", |_| {}),
                "changed",
                "rotated",
                SCOPE
            ),
            JWKS,
            NOW
        )
        .unwrap_err(),
        AuthError::StaleAttempt
    );
    assert_eq!(
        s.bearer(&r, NOW).unwrap().authorization_value(),
        "Bearer synthetic-access"
    );
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn refresh_ambiguity_and_temporary_failure() {
    let p = root();
    let s = store(&p);
    let r = begin(&s, "client_one", "subject");
    assert_eq!(
        s.refresh_with(&r, NOW, JWKS, |_| Err(RefreshFailure::Temporary))
            .unwrap_err(),
        AuthError::Unavailable
    );
    assert!(s.bearer(&r, NOW).is_ok());
    assert_eq!(
        s.refresh_with(&r, NOW, JWKS, |_| Err(RefreshFailure::Ambiguous))
            .unwrap_err(),
        AuthError::AmbiguousRefresh
    );
    assert!(matches!(
        s.readiness(NOW).unwrap(),
        Readiness::RefreshAmbiguous(_)
    ));
    assert_eq!(
        s.refresh_with(&r, NOW, JWKS, |_| panic!(
            "must not reuse possibly rotated token"
        ))
        .unwrap_err(),
        AuthError::AmbiguousRefresh
    );
    assert_eq!(s.bearer(&r, NOW).unwrap_err(), AuthError::Unavailable);
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn owner_only_store_and_symlink_rejection() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let p = root();
    let s = store(&p);
    drop(s);
    assert_eq!(
        fs::metadata(&p).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(p.join("state.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(p.join(".lock")).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let link = p.with_extension("link");
    symlink(&p, &link).unwrap();
    assert!(matches!(
        AuthStore::open(&link, "synthetic_host_1", issuer()),
        Err(AuthError::UnsafeStore)
    ));
    fs::remove_file(link).unwrap();
    let state = p.join("state.json");
    fs::set_permissions(&state, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        AuthStore::open(&p, "synthetic_host_1", issuer()),
        Err(AuthError::UnsafeStore)
    ));
    fs::set_permissions(&state, fs::Permissions::from_mode(0o600)).unwrap();
    fs::remove_file(&state).unwrap();
    assert!(matches!(
        AuthStore::open(&p, "synthetic_host_1", issuer()),
        Err(AuthError::UnsafeStore)
    ));
    symlink(p.join(".lock"), &state).unwrap();
    assert!(matches!(
        AuthStore::open(&p, "synthetic_host_1", issuer()),
        Err(AuthError::UnsafeStore)
    ));
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn bounded_inputs_and_orphan_atomic_file_do_not_change_published_state() {
    let p = root();
    let s = store(&p);
    let r = begin(&s, "client_one", "subject");
    fs::write(p.join(".state-orphan"), b"crash before rename").unwrap();
    let reopened = store(&p);
    assert_eq!(
        reopened.bearer(&r, NOW).unwrap().authorization_value(),
        "Bearer synthetic-access"
    );
    assert_eq!(
        s.accept_callback(&"x".repeat(8193), NOW).unwrap_err(),
        AuthError::Bound
    );
    assert_eq!(
        TokenResponse::from_json(&vec![b'x'; 256 * 1024 + 1]).unwrap_err(),
        AuthError::Bound
    );
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn denied_and_expired_state() {
    let p = root();
    let s = store(&p);
    let start = s
        .begin_registration(
            "Synthetic Tool",
            "http://127.0.0.1:45123/auth/callback",
            NOW,
        )
        .unwrap();
    let state = param(start.authorize_url(), "state");
    assert_eq!(
        s.accept_callback(
            &format!("http://127.0.0.1:45123/auth/callback?error=access_denied&state={state}"),
            NOW
        )
        .unwrap_err(),
        AuthError::Denied
    );
    let start = s
        .begin_registration(
            "Synthetic Tool",
            "http://127.0.0.1:45123/auth/callback",
            NOW,
        )
        .unwrap();
    let state = param(start.authorize_url(), "state");
    assert_eq!(
        s.accept_callback(
            &format!(
                "http://127.0.0.1:45123/auth/callback?code=x&state={state}&client_id=client_one"
            ),
            NOW + 601
        )
        .unwrap_err(),
        AuthError::StaleAttempt
    );
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn invalid_grant_keeps_unverified_issued_client_only_for_explicit_retry() {
    let p = root();
    let s = store(&p);
    let start = s
        .begin_registration(
            "Synthetic Tool",
            "http://127.0.0.1:45123/auth/callback",
            NOW,
        )
        .unwrap();
    let state = param(start.authorize_url(), "state");
    let grant = s
        .accept_callback(
            &format!(
                "http://127.0.0.1:45123/auth/callback?code=x&state={state}&client_id=client_one"
            ),
            NOW,
        )
        .unwrap();
    s.invalid_grant(&grant).unwrap();
    assert_eq!(
        s.invalid_grant(&grant).unwrap_err(),
        AuthError::StaleAttempt
    );
    let retry = s
        .begin_issued_retry("http://127.0.0.1:45123/auth/callback", NOW)
        .unwrap();
    assert_eq!(param(retry.authorize_url(), "client_id"), "client_one");
    assert!(!retry.authorize_url().contains("agent_name_hint="));
    assert!(matches!(s.readiness(NOW).unwrap(), Readiness::SignedOut));
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn crash_during_refresh_or_revocation_blocks_old_bearer() {
    let exe = std::env::current_exe().unwrap();
    for mode in ["refresh", "revoke"] {
        let p = root();
        let s = store(&p);
        let r = begin(&s, "client_one", "subject");
        drop(s);
        assert!(Command::new(&exe)
            .arg("--exact")
            .arg("crash_child")
            .env("B2A_CRASH_ROOT", &p)
            .env("B2A_CRASH_MODE", mode)
            .env("B2A_CRASH_REV", r.credential_revision.to_string())
            .status()
            .unwrap()
            .success());
        let reopened = store(&p);
        assert_eq!(
            reopened.bearer(&r, NOW).unwrap_err(),
            AuthError::Unavailable
        );
        assert!(matches!(
            reopened.readiness(NOW).unwrap(),
            Readiness::RefreshAmbiguous(_) | Readiness::ReauthorizationRequired(_)
        ));
        fs::remove_dir_all(p).unwrap();
    }
}
#[test]
fn crash_child() {
    let Ok(root) = std::env::var("B2A_CRASH_ROOT") else {
        return;
    };
    let mode = std::env::var("B2A_CRASH_MODE").unwrap();
    let rev = std::env::var("B2A_CRASH_REV").unwrap().parse().unwrap();
    let s = store(Path::new(&root));
    let r = bounded_inference::RegistrationRef {
        client_id: "client_one".into(),
        subject: "subject".into(),
        credential_revision: rev,
    };
    if mode == "refresh" {
        let _ = s.refresh_with(&r, NOW, JWKS, |_| std::process::exit(0));
    } else {
        let _ = s.sign_out_with(&r, |_| std::process::exit(0));
    }
}
#[test]
fn competing_processes_serialize_one_rotating_refresh() {
    let p = root();
    let s = store(&p);
    let r = begin(&s, "client_one", "subject");
    drop(s);
    let exe = std::env::current_exe().unwrap();
    let marker = p.join("refresh-calls");
    let mut children = Vec::new();
    for _ in 0..2 {
        children.push(
            Command::new(&exe)
                .arg("--exact")
                .arg("refresh_child")
                .arg("--nocapture")
                .env("B2A_CHILD_ROOT", &p)
                .env("B2A_CHILD_MARKER", &marker)
                .env("B2A_CHILD_REV", r.credential_revision.to_string())
                .spawn()
                .unwrap(),
        );
    }
    for mut c in children {
        assert!(c.wait().unwrap().success());
    }
    assert_eq!(fs::read_to_string(marker).unwrap().lines().count(), 1);
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn refresh_child() {
    let Ok(root) = std::env::var("B2A_CHILD_ROOT") else {
        return;
    };
    let marker = std::env::var("B2A_CHILD_MARKER").unwrap();
    let rev = std::env::var("B2A_CHILD_REV").unwrap().parse().unwrap();
    let s = store(Path::new(&root));
    let r = bounded_inference::RegistrationRef {
        client_id: "client_one".into(),
        subject: "subject".into(),
        credential_revision: rev,
    };
    let result = s.refresh_with(&r, NOW, JWKS, |_| {
        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(marker)
            .unwrap();
        writeln!(f, "one exchange").unwrap();
        Ok(response(
            &signed("client_one", None, "subject", |_| {}),
            "new-access",
            "new-refresh",
            SCOPE,
        ))
    });
    assert!(result.is_ok() || result == Err(AuthError::StaleRevision));
}

#[test]
fn signed_out_registration_is_reachable_after_restart_without_private_state_read() {
    let p = root();
    let s = store(&p);
    let old = begin(&s, "client_one", "subject");
    assert_eq!(
        s.sign_out_with(&old, |_| true).unwrap(),
        RevocationOutcome::Confirmed
    );
    drop(s);
    let reopened = store(&p);
    let saved = reopened.saved_registrations().unwrap();
    assert_eq!(saved.len(), 1);
    assert!(!saved[0].signed_in);
    assert_eq!(saved[0].email.as_deref(), Some("same@example.invalid"));
    assert!(saved[0].registration.credential_revision > old.credential_revision);
    assert_eq!(
        reopened
            .begin_returning(&old, "http://127.0.0.1:54321/auth/callback", NOW)
            .unwrap_err(),
        AuthError::StaleRevision
    );
    let start = reopened
        .begin_returning(
            &saved[0].registration,
            "http://127.0.0.1:54321/auth/callback",
            NOW,
        )
        .unwrap();
    assert_eq!(param(start.authorize_url(), "client_id"), "client_one");
    let state = param(start.authorize_url(), "state");
    let nonce = param(start.authorize_url(), "nonce");
    let grant = reopened
        .accept_callback(
            &format!("http://127.0.0.1:54321/auth/callback?code=again&state={state}"),
            NOW,
        )
        .unwrap();
    let current = reopened
        .finish_exchange(
            &grant,
            response(
                &signed("client_one", Some(&nonce), "subject", |_| {}),
                "new-access",
                "new-refresh",
                SCOPE,
            ),
            JWKS,
            NOW,
        )
        .unwrap();
    assert_eq!(
        reopened
            .bearer(&current, NOW)
            .unwrap()
            .authorization_value(),
        "Bearer new-access"
    );
    fs::remove_dir_all(p).unwrap();
}

#[test]
fn issuer_and_endpoint_policy_cannot_retarget_persisted_secrets() {
    let p = root();
    let s = store(&p);
    let _ = begin(&s, "client_one", "subject");
    drop(s);
    let foreign = Issuer::new(
        "https://other-issuer.invalid",
        "https://other-issuer.invalid/authorize",
        "https://other-issuer.invalid/token",
        "https://other-issuer.invalid/revoke",
    )
    .unwrap();
    assert!(matches!(
        AuthStore::open(&p, "synthetic_host_1", foreign),
        Err(AuthError::UnsafeStore)
    ));
    let changed_endpoint = Issuer::new(
        "https://fake-issuer.invalid",
        "https://fake-issuer.invalid/authorize",
        "https://fake-issuer.invalid/token",
        "https://fake-issuer.invalid/other-revoke",
    )
    .unwrap();
    assert!(matches!(
        AuthStore::open(&p, "synthetic_host_1", changed_endpoint),
        Err(AuthError::UnsafeStore)
    ));
    assert_eq!(store(&p).saved_registrations().unwrap().len(), 1);
    fs::remove_dir_all(p).unwrap();
}

#[test]
fn failed_real_rename_cleans_rotated_secret_before_local_signout() {
    let p = root();
    let s = store(&p);
    let r = begin(&s, "client_one", "subject");
    let result = s.refresh_with(&r, NOW, JWKS, |_| {
        fs::rename(p.join("state.json"), p.join("saved-inflight.json")).unwrap();
        fs::create_dir(p.join("state.json")).unwrap();
        Ok(response(
            &signed("client_one", None, "subject", |_| {}),
            "new-orphan-access",
            "new-orphan-refresh",
            SCOPE,
        ))
    });
    assert_eq!(result.unwrap_err(), AuthError::Store);
    assert_eq!(
        fs::read_dir(&p)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(".state-"))
            .count(),
        0
    );
    fs::remove_dir(p.join("state.json")).unwrap();
    fs::rename(p.join("saved-inflight.json"), p.join("state.json")).unwrap();
    assert_eq!(
        s.sign_out_with(&r, |_| false).unwrap(),
        RevocationOutcome::Unconfirmed
    );
    assert!(matches!(s.readiness(NOW).unwrap(), Readiness::SignedOut));
    for e in fs::read_dir(&p).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_file() {
            assert!(!fs::read_to_string(e.path())
                .unwrap()
                .contains("new-orphan-refresh"));
        }
    }
    fs::remove_dir_all(p).unwrap();
}

#[test]
fn fifo_state_is_rejected_without_waiting_for_a_writer() {
    use std::time::{Duration, Instant};
    let p = root();
    let s = store(&p);
    drop(s);
    fs::remove_file(p.join("state.json")).unwrap();
    assert!(Command::new("mkfifo")
        .arg("-m")
        .arg("600")
        .arg(p.join("state.json"))
        .status()
        .unwrap()
        .success());
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("fifo_open_child")
        .env("B2A_FIFO_ROOT", &p)
        .spawn()
        .unwrap();
    let start = Instant::now();
    let result = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break Some(s);
        }
        if start.elapsed() > Duration::from_secs(2) {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(result.is_some_and(|s| s.success()));
    fs::remove_dir_all(p).unwrap();
}
#[test]
fn fifo_open_child() {
    let Ok(p) = std::env::var("B2A_FIFO_ROOT") else {
        return;
    };
    assert!(matches!(
        AuthStore::open(Path::new(&p), "synthetic_host_1", issuer()),
        Err(AuthError::UnsafeStore)
    ));
}

#[cfg(feature = "auth-test-faults")]
#[test]
fn publication_io_failures_preserve_old_or_new_state_and_clean_temps() {
    use bounded_inference::StoreFault;
    for stage in [
        StoreFault::Write,
        StoreFault::FileSync,
        StoreFault::Rename,
        StoreFault::DirectorySync,
    ] {
        let p = root();
        let s = store(&p);
        let old = begin(&s, "client_one", "subject");
        let result = s.refresh_with(&old, NOW, JWKS, |_| {
            s.set_store_fault(stage);
            Ok(response(
                &signed("client_one", None, "subject", |_| {}),
                "new-access",
                "new-refresh",
                SCOPE,
            ))
        });
        assert_eq!(result.unwrap_err(), AuthError::Store, "{stage:?}");
        let reopened = store(&p);
        assert_eq!(
            fs::read_dir(&p)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().starts_with(".state-"))
                .count(),
            0,
            "{stage:?}"
        );
        let current = reopened
            .saved_registrations()
            .unwrap()
            .remove(0)
            .registration;
        if stage == StoreFault::DirectorySync {
            assert!(current.credential_revision > old.credential_revision);
            assert_eq!(
                reopened
                    .bearer(&current, NOW)
                    .unwrap()
                    .authorization_value(),
                "Bearer new-access"
            );
        } else {
            assert_eq!(current.credential_revision, old.credential_revision);
            assert!(matches!(
                reopened.readiness(NOW).unwrap(),
                Readiness::RefreshAmbiguous(_)
            ));
            assert_eq!(
                reopened.bearer(&old, NOW).unwrap_err(),
                AuthError::Unavailable
            );
        }
        reopened.sign_out_with(&current, |_| false).unwrap();
        for e in fs::read_dir(&p).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_file() {
                let content = fs::read_to_string(e.path()).unwrap();
                assert!(!content.contains("new-refresh"));
            }
        }
        fs::remove_dir_all(p).unwrap();
    }
}

#[cfg(feature = "auth-test-faults")]
#[test]
fn process_death_at_publication_boundaries_reopens_without_secret_orphan() {
    let exe = std::env::current_exe().unwrap();
    for (name, after_publish) in [
        ("write", false),
        ("file-sync", false),
        ("rename", true),
        ("directory-sync", true),
    ] {
        let p = root();
        let s = store(&p);
        let old = begin(&s, "client_one", "subject");
        drop(s);
        let status = Command::new(&exe)
            .arg("--exact")
            .arg("publication_death_child")
            .env("B2A_PUBLISH_ROOT", &p)
            .env("B2A_PUBLISH_STAGE", name)
            .env("B2A_PUBLISH_REV", old.credential_revision.to_string())
            .status()
            .unwrap();
        assert!(status.success(), "{name}");
        let reopened = store(&p);
        let current = reopened
            .saved_registrations()
            .unwrap()
            .remove(0)
            .registration;
        assert_eq!(
            fs::read_dir(&p)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|e| e.file_name().to_string_lossy().starts_with(".state-"))
                .count(),
            0,
            "{name}"
        );
        if after_publish {
            assert_eq!(
                reopened
                    .bearer(&current, NOW)
                    .unwrap()
                    .authorization_value(),
                "Bearer new-access"
            );
        } else {
            assert!(matches!(
                reopened.readiness(NOW).unwrap(),
                Readiness::RefreshAmbiguous(_)
            ));
            assert_eq!(
                reopened.bearer(&old, NOW).unwrap_err(),
                AuthError::Unavailable
            );
        }
        reopened.sign_out_with(&current, |_| false).unwrap();
        for e in fs::read_dir(&p).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_file() {
                assert!(!fs::read_to_string(e.path())
                    .unwrap()
                    .contains("new-refresh"));
            }
        }
        fs::remove_dir_all(p).unwrap();
    }
}
#[cfg(feature = "auth-test-faults")]
#[test]
fn publication_death_child() {
    use bounded_inference::StoreFault;
    let Ok(root) = std::env::var("B2A_PUBLISH_ROOT") else {
        return;
    };
    let stage = std::env::var("B2A_PUBLISH_STAGE").unwrap();
    let rev = std::env::var("B2A_PUBLISH_REV").unwrap().parse().unwrap();
    let fault = match stage.as_str() {
        "write" => StoreFault::ExitAfterWrite,
        "file-sync" => StoreFault::ExitAfterFileSync,
        "rename" => StoreFault::ExitAfterRename,
        "directory-sync" => StoreFault::ExitAfterDirectorySync,
        _ => panic!("bad stage"),
    };
    let s = store(Path::new(&root));
    let r = bounded_inference::RegistrationRef {
        client_id: "client_one".into(),
        subject: "subject".into(),
        credential_revision: rev,
    };
    let _ = s.refresh_with(&r, NOW, JWKS, |_| {
        s.set_store_fault(fault);
        Ok(response(
            &signed("client_one", None, "subject", |_| {}),
            "new-access",
            "new-refresh",
            SCOPE,
        ))
    });
    panic!("fault did not exit");
}

#[test]
fn barrier_proves_refresh_serializes_switch_and_signout_processes() {
    use std::time::{Duration, Instant};
    let exe = std::env::current_exe().unwrap();
    for action in ["switch", "signout"] {
        let p = root();
        let s = store(&p);
        let old = begin(&s, "client_one", "subject");
        drop(s);
        let entered = p.join("refresh-entered");
        let release = p.join("refresh-release");
        let started = p.join("action-started");
        let mut refresh = Command::new(&exe)
            .arg("--exact")
            .arg("refresh_barrier_child")
            .env("B2A_RACE_ROOT", &p)
            .env("B2A_RACE_REV", old.credential_revision.to_string())
            .env("B2A_RACE_ENTERED", &entered)
            .env("B2A_RACE_RELEASE", &release)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !entered.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        if !entered.exists() {
            refresh.kill().unwrap();
            refresh.wait().unwrap();
            panic!("refresh barrier absent");
        }
        let mut contender = Command::new(&exe)
            .arg("--exact")
            .arg("race_action_child")
            .env("B2A_RACE_ROOT", &p)
            .env("B2A_RACE_REV", old.credential_revision.to_string())
            .env("B2A_RACE_ACTION", action)
            .env("B2A_RACE_STARTED", &started)
            .spawn()
            .unwrap();
        while !started.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        if !started.exists() {
            contender.kill().unwrap();
            contender.wait().unwrap();
            refresh.kill().unwrap();
            refresh.wait().unwrap();
            panic!("action barrier absent");
        }
        std::thread::sleep(Duration::from_millis(100));
        assert!(
            contender.try_wait().unwrap().is_none(),
            "{action} did not wait for refresh lock"
        );
        fs::write(&release, b"release").unwrap();
        assert!(refresh.wait().unwrap().success());
        assert!(contender.wait().unwrap().success());
        let reopened = store(&p);
        let current = reopened
            .saved_registrations()
            .unwrap()
            .remove(0)
            .registration;
        assert!(current.credential_revision > old.credential_revision);
        assert_eq!(
            reopened
                .bearer(&current, NOW)
                .unwrap()
                .authorization_value(),
            "Bearer post-refresh-access"
        );
        fs::remove_dir_all(p).unwrap();
    }
}
#[test]
fn refresh_barrier_child() {
    use std::time::{Duration, Instant};
    let Ok(root) = std::env::var("B2A_RACE_ROOT") else {
        return;
    };
    let entered = std::env::var("B2A_RACE_ENTERED").unwrap();
    let release = std::env::var("B2A_RACE_RELEASE").unwrap();
    let rev = std::env::var("B2A_RACE_REV").unwrap().parse().unwrap();
    let s = store(Path::new(&root));
    let r = bounded_inference::RegistrationRef {
        client_id: "client_one".into(),
        subject: "subject".into(),
        credential_revision: rev,
    };
    s.refresh_with(&r, NOW, JWKS, |_| {
        fs::write(&entered, b"entered").unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !Path::new(&release).exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(Path::new(&release).exists(), "release deadline");
        Ok(response(
            &signed("client_one", None, "subject", |_| {}),
            "post-refresh-access",
            "post-refresh-token",
            SCOPE,
        ))
    })
    .unwrap();
}
#[test]
fn race_action_child() {
    let Ok(root) = std::env::var("B2A_RACE_ROOT") else {
        return;
    };
    let rev = std::env::var("B2A_RACE_REV").unwrap().parse().unwrap();
    let action = std::env::var("B2A_RACE_ACTION").unwrap();
    fs::write(std::env::var("B2A_RACE_STARTED").unwrap(), b"started").unwrap();
    let s = store(Path::new(&root));
    let r = bounded_inference::RegistrationRef {
        client_id: "client_one".into(),
        subject: "subject".into(),
        credential_revision: rev,
    };
    let result = if action == "switch" {
        s.switch(&r)
    } else {
        s.sign_out_with(&r, |_| panic!("stale signout must not revoke"))
            .map(|_| ())
    };
    assert_eq!(result.unwrap_err(), AuthError::StaleRevision);
}
#[test]
fn terminal_refresh_blocks_use_and_all_secret_debug_views_are_redacted() {
    let p = root();
    let s = store(&p);
    let start = s
        .begin_registration(
            "Synthetic Tool",
            "http://127.0.0.1:45123/auth/callback",
            NOW,
        )
        .unwrap();
    let state = param(start.authorize_url(), "state");
    let nonce = param(start.authorize_url(), "nonce");
    let start_debug = format!("{start:?}");
    assert!(!start_debug.contains(&state));
    assert!(!start_debug.contains(&nonce));
    let grant=s.accept_callback(&format!("http://127.0.0.1:45123/auth/callback?code=secret-code&state={state}&client_id=client_one"),NOW).unwrap();
    let grant_debug = format!("{grant:?}");
    assert!(!grant_debug.contains("secret-code"));
    assert!(!grant_debug.contains(grant.code_verifier()));
    let token = response(
        &signed("client_one", Some(&nonce), "subject", |_| {}),
        "secret-access",
        "secret-refresh",
        SCOPE,
    );
    let token_debug = format!("{token:?}");
    for secret in ["secret-access", "secret-refresh"] {
        assert!(!token_debug.contains(secret));
    }
    let r = s.finish_exchange(&grant, token, JWKS, NOW).unwrap();
    assert_eq!(
        s.refresh_with(&r, NOW, JWKS, |refresh| {
            assert!(!format!("{refresh:?}").contains("secret-refresh"));
            Err(RefreshFailure::Terminal)
        })
        .unwrap_err(),
        AuthError::Unavailable
    );
    assert!(matches!(
        s.readiness(NOW).unwrap(),
        Readiness::ReauthorizationRequired(_)
    ));
    assert_eq!(s.bearer(&r, NOW).unwrap_err(), AuthError::Unavailable);
    let result = s
        .sign_out_with(&r, |revoke| {
            assert_eq!(revoke.refresh_token(), "secret-refresh");
            assert!(!format!("{revoke:?}").contains("secret-refresh"));
            false
        })
        .unwrap();
    assert_eq!(result, RevocationOutcome::Unconfirmed);
    for err in [
        AuthError::Store,
        AuthError::Identity,
        AuthError::Unavailable,
        AuthError::AmbiguousRefresh,
    ] {
        let view = format!("{err:?} {err}");
        for secret in [
            "secret-code",
            "secret-access",
            "secret-refresh",
            grant.code_verifier(),
        ] {
            assert!(!view.contains(secret));
        }
    }
    fs::remove_dir_all(p).unwrap();
}
