//! A separate test process owns the working-directory change in this regression.
#[allow(dead_code)]
mod support;

use std::fs;
use std::path::{Path, PathBuf};

use review_episode::admission::FixtureAdmission;
use review_episode::operation::Operation;
use review_episode::protocol::{DirectRequest, RequestSelection};
use review_episode::{Application, OperationResult};
use review_episode_core::codec::{JsString, JsValue, canonical_json, digest};
use review_episode_core::identity::{Authority, Revision};
use review_episode_store::{EpisodeStore, StoreOptions, init_native_root, init_offline_root};

struct RestoreDirectory(PathBuf);
impl Drop for RestoreDirectory {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).expect("restore test working directory");
    }
}

fn run_case(base: &Path, label: &str, fault: bool) {
    std::env::set_current_dir(base).unwrap();
    let relative = PathBuf::from(format!("episode-{label}"));
    init_offline_root(&relative).unwrap();
    let absolute = relative.canonicalize().unwrap();
    let authority = Authority::parse(support::authority()).unwrap();
    let identity = authority.identity().clone();
    let begin = DirectRequest::new(
        RequestSelection::Offline {
            request_id: format!("begin-{label}"),
            grant_id: format!("grant-{label}"),
            selection_revision: Revision("a".repeat(64)),
        },
        Operation::Begin {
            authority,
            transition_id: JsString::new(label),
            unresolved_questions: vec![],
        },
    )
    .unwrap();
    let registry = base.join(format!("registry-{label}.json"));
    let grants = JsValue::Array(vec![JsValue::object([
        ("grantId", JsValue::text(&format!("grant-{label}"))),
        (
            "requestDigest",
            JsValue::text(&digest(begin.request().envelope())),
        ),
        ("selectionRevision", JsValue::text(&"a".repeat(64))),
        ("observedRevision", JsValue::Null),
    ])]);
    fs::write(
        &registry,
        canonical_json(&JsValue::object([
            ("root", JsValue::text(absolute.to_str().unwrap())),
            ("grants", grants),
        ])),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&registry, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let store = EpisodeStore::open(&relative, StoreOptions::default()).unwrap();
    assert_eq!(store.root(), absolute);
    let port = FixtureAdmission::load(&registry, &relative).unwrap();
    let mut owner = Application::new(store, port, 32 * 1024 * 1024);
    #[cfg(feature = "test-faults")]
    if fault {
        unsafe { std::env::set_var("REVIEW_EPISODE_FAULT_CUT", "after_commit_unknown") };
    }
    let outcome = owner.execute_direct(&begin);
    #[cfg(feature = "test-faults")]
    if fault {
        unsafe { std::env::remove_var("REVIEW_EPISODE_FAULT_CUT") };
    }
    #[cfg(not(feature = "test-faults"))]
    assert!(!fault);
    let context = if fault {
        outcome
            .unwrap_err()
            .possible_commit()
            .expect("uncertain write has exact context")
            .clone()
    } else {
        match outcome.unwrap() {
            OperationResult::Write { context, .. } => context,
            _ => panic!("expected write"),
        }
    };
    assert_eq!(context.root, absolute);
    assert_eq!(context.operation, "begin");
    assert_eq!(context.identity, identity);
    assert_eq!(context.transition_id, JsString::new(label));
    drop(owner);
    let different = base.join(format!("different-{label}"));
    fs::create_dir(&different).unwrap();
    std::env::set_current_dir(&different).unwrap();
    let mut reopened = EpisodeStore::open(&context.root, StoreOptions::default()).unwrap();
    let snapshot = reopened.read(&identity.key().0).unwrap();
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.current.unwrap().identity(), &identity);
}

#[test]
fn relative_store_root_stays_anchored_for_success_and_uncertain_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let _restore = RestoreDirectory(std::env::current_dir().unwrap());
    run_case(temp.path(), "success", false);
    #[cfg(feature = "test-faults")]
    run_case(temp.path(), "uncertain", true);
    std::env::set_current_dir(temp.path()).unwrap();
    let native_relative = PathBuf::from("episode-native");
    let expected = init_native_root(&native_relative).unwrap();
    let (store, selected) =
        EpisodeStore::open_native(&native_relative, StoreOptions::default()).unwrap();
    assert_eq!(store.root(), native_relative.canonicalize().unwrap());
    assert_eq!(selected, expected);
}
