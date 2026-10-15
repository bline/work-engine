//! Test-only binding shared by the two installed native compatibility targets.
use sha2::{Digest, Sha256};
use std::{ffi::OsString, fs, os::unix::fs::PermissionsExt, path::PathBuf, sync::OnceLock};

pub struct NativeBinding {
    pub binary: PathBuf,
    pub temporary: PathBuf,
    pub target: PathBuf,
}

impl NativeBinding {
    fn from_values(
        mut value: impl FnMut(&str) -> Option<OsString>,
        supported_sha256: &str,
    ) -> Result<Self, String> {
        let expected = value("S8_NATIVE_BINARY_SHA256")
            .and_then(|v| v.into_string().ok())
            .ok_or("missing native digest")?;
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("invalid native digest".into());
        }
        if expected != supported_sha256 {
            return Err("native binding does not match the supported adapter profile".into());
        }
        let mut path = |key: &str| -> Result<PathBuf, String> {
            let p = PathBuf::from(value(key).ok_or_else(|| format!("missing {key}"))?);
            if !p.is_absolute() {
                return Err(format!("{key} must be absolute"));
            }
            p.canonicalize().map_err(|_| format!("{key} unavailable"))
        };
        let binary = path("S8_NATIVE_BINARY")?;
        let root = path("S8_NATIVE_RUN_ROOT")?;
        let temporary = path("TMPDIR")?;
        let target = path("CARGO_TARGET_DIR")?;
        let meta = fs::metadata(&binary).map_err(|_| "native metadata unavailable")?;
        if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
            return Err("native executable unavailable".into());
        }
        if format!(
            "{:x}",
            Sha256::digest(fs::read(&binary).map_err(|_| "native bytes unavailable")?)
        ) != expected
        {
            return Err("native executable digest mismatch".into());
        }
        if !root.is_dir()
            || fs::metadata(&root)
                .map_err(|_| "run metadata unavailable")?
                .permissions()
                .mode()
                & 0o777
                != 0o700
            || !temporary.is_dir()
            || !target.is_dir()
            || temporary == root
            || target == root
            || temporary == target
            || !temporary.starts_with(&root)
            || !target.starts_with(&root)
        {
            return Err(
                "temporary/target directories must be distinct children of the private run root"
                    .into(),
            );
        }
        Ok(Self {
            binary,
            temporary,
            target,
        })
    }
}

pub fn checked() -> &'static NativeBinding {
    static BINDING: OnceLock<NativeBinding> = OnceLock::new();
    BINDING.get_or_init(|| {
        let binding = NativeBinding::from_values(
            |key| std::env::var_os(key),
            lifecycle_codex::NATIVE_BINARY_SHA256,
        )
        .expect("exact native test binding required before effects");
        let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
        assert!(
            executable.starts_with(&binding.target),
            "test executable is outside the bound target"
        );
        binding
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn fixture_digest() -> String {
        format!("{:x}", Sha256::digest(b"not executed"))
    }

    #[test]
    fn refuses_unsupported_profile_before_resolving_paths() {
        let error = NativeBinding::from_values(
            |key| match key {
                "S8_NATIVE_BINARY_SHA256" => Some(OsString::from("0".repeat(64))),
                _ => panic!("unsupported binding reached path resolution"),
            },
            lifecycle_codex::NATIVE_BINARY_SHA256,
        )
        .err()
        .expect("unsupported profile must fail");
        assert_eq!(
            error,
            "native binding does not match the supported adapter profile"
        );
    }

    fn fixture() -> (tempfile::TempDir, BTreeMap<String, OsString>) {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let binary = root.path().join("native");
        fs::write(&binary, b"not executed").unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let temporary = root.path().join("tmp");
        let target = root.path().join("target");
        fs::create_dir(&temporary).unwrap();
        fs::create_dir(&target).unwrap();
        let values = BTreeMap::from([
            ("S8_NATIVE_BINARY".into(), binary.into_os_string()),
            (
                "S8_NATIVE_RUN_ROOT".into(),
                root.path().as_os_str().to_owned(),
            ),
            ("TMPDIR".into(), temporary.into_os_string()),
            ("CARGO_TARGET_DIR".into(), target.into_os_string()),
            (
                "S8_NATIVE_BINARY_SHA256".into(),
                OsString::from(format!("{:x}", Sha256::digest(b"not executed"))),
            ),
        ]);
        (root, values)
    }

    #[test]
    fn validates_without_launching_and_refuses_missing_or_relative_binding() {
        let (_root, mut values) = fixture();
        assert!(
            NativeBinding::from_values(|key| values.get(key).cloned(), &fixture_digest()).is_ok()
        );
        for key in [
            "S8_NATIVE_BINARY",
            "S8_NATIVE_RUN_ROOT",
            "TMPDIR",
            "CARGO_TARGET_DIR",
            "S8_NATIVE_BINARY_SHA256",
        ] {
            let saved = values.remove(key).unwrap();
            assert!(
                NativeBinding::from_values(|key| values.get(key).cloned(), &fixture_digest())
                    .is_err()
            );
            values.insert(key.into(), saved);
        }
        values.insert("S8_NATIVE_BINARY".into(), OsString::from("relative-native"));
        assert!(
            NativeBinding::from_values(|key| values.get(key).cloned(), &fixture_digest()).is_err()
        );
    }

    #[test]
    fn refuses_wrong_digest_and_run_directory_escape_without_launching() {
        let (_root, mut values) = fixture();
        let correct = values
            .insert(
                "S8_NATIVE_BINARY_SHA256".into(),
                OsString::from("0".repeat(64)),
            )
            .unwrap();
        assert!(
            NativeBinding::from_values(|key| values.get(key).cloned(), &fixture_digest()).is_err()
        );
        values.insert("S8_NATIVE_BINARY_SHA256".into(), correct);
        let outside = tempfile::tempdir().unwrap();
        values.insert("TMPDIR".into(), outside.path().as_os_str().to_owned());
        assert!(
            NativeBinding::from_values(|key| values.get(key).cloned(), &fixture_digest()).is_err()
        );
        values.insert("TMPDIR".into(), values["S8_NATIVE_RUN_ROOT"].clone());
        assert!(
            NativeBinding::from_values(|key| values.get(key).cloned(), &fixture_digest()).is_err()
        );
    }
}
