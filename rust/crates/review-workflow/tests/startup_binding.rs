#![cfg(unix)]
use review_workflow::startup::FilePin;
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
    };
    assert_eq!(pin.read(11).unwrap(), b"exact bytes");
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
