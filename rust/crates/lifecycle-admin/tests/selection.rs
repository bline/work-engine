use std::fs;

use lifecycle_admin::selection::{
    FileLaunchSelectionPort, LaunchSelectionGuard, LaunchSelectionPort, SelectionError,
    SelectionRecord,
};
use tempfile::tempdir;

#[test]
fn guarded_selection_cas_is_exact_and_restart_readable() {
    let temporary = tempdir().unwrap();
    let source = temporary.path().join("source");
    let destination = temporary.path().join("destination");
    let selector = temporary.path().join("selector");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&destination).unwrap();
    fs::create_dir(&selector).unwrap();
    let before = SelectionRecord {
        generation: 0,
        store_id: "store:source".into(),
        root: source,
        handoff_id: None,
        capsule_digest: None,
    };
    let mut port = FileLaunchSelectionPort::initialize(&selector, before.clone()).unwrap();
    let after = SelectionRecord {
        generation: 1,
        store_id: "store:destination".into(),
        root: destination,
        handoff_id: Some("handoff-1".into()),
        capsule_digest: Some("capsule-sha".into()),
    };
    {
        let mut guard = port.lock().unwrap();
        assert_eq!(guard.read().unwrap(), before);
        let committed = guard.compare_and_swap(&before, after.clone()).unwrap();
        assert_eq!(committed.after, after);
        let evidence = guard.evidence(&committed).unwrap();
        assert_eq!(evidence.prior_selection_revision, 0);
        assert_eq!(evidence.current_selection_revision, 1);
        assert_eq!(evidence.handoff_id, "handoff-1");
        assert_eq!(evidence.source_store_id, "store:source");
        assert_eq!(evidence.destination_store_id, "store:destination");
    }
    let mut reopened = FileLaunchSelectionPort::open(&selector).unwrap();
    let mut guard = reopened.lock().unwrap();
    assert_eq!(guard.read().unwrap(), after);
    assert_eq!(
        guard
            .compare_and_swap(&before, after.clone())
            .unwrap()
            .after,
        after
    );
    let changed = SelectionRecord {
        capsule_digest: Some("different".into()),
        ..after
    };
    assert!(matches!(
        guard.compare_and_swap(&before, changed),
        Err(SelectionError::Conflict)
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_root_and_hardlinked_record_are_rejected() {
    use std::os::unix::fs::symlink;

    let temporary = tempdir().unwrap();
    let source = temporary.path().join("source");
    let alias = temporary.path().join("alias");
    let selector = temporary.path().join("selector");
    fs::create_dir(&source).unwrap();
    symlink(&source, &alias).unwrap();
    fs::create_dir(&selector).unwrap();
    let invalid = SelectionRecord {
        generation: 0,
        store_id: "store:source".into(),
        root: alias,
        handoff_id: None,
        capsule_digest: None,
    };
    assert!(matches!(
        FileLaunchSelectionPort::initialize(&selector, invalid),
        Err(SelectionError::Invalid)
    ));
    let valid = SelectionRecord {
        generation: 0,
        store_id: "store:source".into(),
        root: source,
        handoff_id: None,
        capsule_digest: None,
    };
    let mut port = FileLaunchSelectionPort::initialize(&selector, valid).unwrap();
    fs::hard_link(selector.join("selection.json"), selector.join("alias.json")).unwrap();
    let guard = port.lock().unwrap();
    assert!(matches!(guard.read(), Err(SelectionError::Invalid)));
}

#[test]
fn replacing_a_held_lock_inode_invalidates_the_guard() {
    let temporary = tempdir().unwrap();
    let source = temporary.path().join("source");
    let selector = temporary.path().join("selector");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&selector).unwrap();
    let initial = SelectionRecord {
        generation: 0,
        store_id: "store:source".into(),
        root: source,
        handoff_id: None,
        capsule_digest: None,
    };
    let mut port = FileLaunchSelectionPort::initialize(&selector, initial).unwrap();
    let guard = port.lock().unwrap();
    fs::remove_file(selector.join("selection.lock")).unwrap();
    fs::write(selector.join("selection.lock"), b"replacement").unwrap();
    assert!(matches!(guard.read(), Err(SelectionError::Invalid)));
}
