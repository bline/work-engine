#![cfg(unix)]
use review_workflow::startup::{FilePin, TrustedStartup};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
#[test]
fn captured_file_pins_check_actual_bytes_mode_path_and_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("input");
    fs::write(&path, b"exact bytes").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    let pin = FilePin {
        path: path.clone(),
        sha256: claim_evidence::sha256_bytes(b"exact bytes"),
        mode: 0o400,
        owner_uid: None,
    };
    assert_eq!(pin.read(11).unwrap(), b"exact bytes");
    assert!(
        FilePin {
            owner_uid: Some(u32::MAX),
            ..pin.clone()
        }
        .read(11)
        .is_err(),
        "qualified owner binding must reject a different file owner"
    );
    assert!(pin.read(10).is_err());
    let linked = temp.path().join("link");
    symlink(&path, &linked).unwrap();
    assert!(
        FilePin {
            path: linked,
            ..pin.clone()
        }
        .read(11)
        .is_err()
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(pin.read(11).is_err());
    fs::write(&path, b"other bytes").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    assert!(pin.read(11).is_err());
}

#[test]
fn production_qualified_startup_refuses_user_owned_descriptor_before_parsing_authority() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("qualified-startup.json");
    fs::write(&path, br#"{"profile":"review-host-qualified-initial-v1"}"#).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
    let error = TrustedStartup::load(&path).err().unwrap();
    assert!(
        error.to_string().contains("deployment-owned"),
        "unexpected refusal: {error}"
    );
}
