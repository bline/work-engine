//! Local key-policy loading kernel. Calling this is not proof of storage
//! isolation: the deployment must protect the file and all its parent paths.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{
    Deserialize, Deserializer,
    de::{Error, MapAccess, Visitor},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};

#[derive(Deserialize)]
struct Key {
    spki_b64url: String,
    fingerprint: String,
    cose_alg: i64,
}
#[derive(Deserialize)]
struct File {
    #[serde(deserialize_with = "unique_keys")]
    keys: BTreeMap<String, Key>,
}
fn unique_keys<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, Key>, D::Error> {
    struct Unique;
    impl<'de> Visitor<'de> for Unique {
        type Value = BTreeMap<String, Key>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a map of distinct credential IDs")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut m: M) -> Result<Self::Value, M::Error> {
            let mut keys = BTreeMap::new();
            while let Some((id, key)) = m.next_entry()? {
                if keys.insert(id, key).is_some() {
                    return Err(M::Error::custom("duplicate key"));
                }
            }
            Ok(keys)
        }
    }
    d.deserialize_map(Unique)
}
#[derive(Debug)]
pub(crate) struct PolicyError;
#[allow(dead_code)]
pub(crate) struct PinnedKey {
    pub(crate) spki_der: Vec<u8>,
    pub(crate) fingerprint: String,
}
/// Supported COSE labels are bound to the encoded key algorithm and parameters.
fn spki_algorithm(bytes: &[u8]) -> Option<i64> {
    use p256::pkcs8::{DecodePublicKey, der::Decode, spki::SubjectPublicKeyInfoRef};
    let spki = SubjectPublicKeyInfoRef::from_der(bytes).ok()?;
    spki.subject_public_key.as_bytes()?;
    match spki.algorithm.oid.to_string().as_str() {
        "1.2.840.10045.2.1" if p256::ecdsa::VerifyingKey::from_public_key_der(bytes).is_ok() => {
            Some(-7)
        }
        "1.3.101.112"
            if spki.algorithm.parameters.is_none()
                && spki.subject_public_key.as_bytes()?.len() == 32 =>
        {
            Some(-8)
        }
        "1.2.840.113549.1.1.1"
            if spki
                .algorithm
                .parameters
                .is_none_or(|p| p.decode_as::<p256::pkcs8::der::asn1::Null>().is_ok()) =>
        {
            Some(-257)
        }
        _ => None,
    }
}
/// Path is selected by a protected operator configuration, never from an MCP
/// request. Reopen at the action boundary to detect removal/revocation.
#[cfg(unix)]
#[allow(dead_code)]
pub(crate) fn load(path: &Path, credential_id: &str) -> Result<PinnedKey, PolicyError> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    if !path.is_absolute() || credential_id.is_empty() {
        return Err(PolicyError);
    }
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| PolicyError)?;
    let meta = f.metadata().map_err(|_| PolicyError)?;
    if !meta.is_file()
        || meta.len() > 1024 * 1024
        || meta.mode() & 0o077 != 0
        || meta.uid() != unsafe { libc::geteuid() }
    {
        return Err(PolicyError);
    }
    let mut bytes = Vec::new();
    f.by_ref()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PolicyError)?;
    let after = f.metadata().map_err(|_| PolicyError)?;
    if bytes.len() as u64 != meta.len()
        || after.len() != meta.len()
        || after.mtime() != meta.mtime()
        || after.mtime_nsec() != meta.mtime_nsec()
    {
        return Err(PolicyError);
    }
    let file: File = serde_json::from_slice(&bytes).map_err(|_| PolicyError)?;
    let key = file.keys.get(credential_id).ok_or(PolicyError)?;
    let spki = URL_SAFE_NO_PAD
        .decode(&key.spki_b64url)
        .map_err(|_| PolicyError)?;
    if spki_algorithm(&spki) != Some(key.cose_alg)
        || format!("{:x}", Sha256::digest(&spki)) != key.fingerprint
    {
        return Err(PolicyError);
    }
    Ok(PinnedKey {
        spki_der: spki,
        fingerprint: key.fingerprint.clone(),
    })
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn declared_algorithms_load_and_verify_real_signed_vectors() {
        let doc: serde_json::Value =
            serde_json::from_str(include_str!("../tests/owner-answer-v1.json")).unwrap();
        let path = std::env::temp_dir().join(format!("algorithm-policy-{}", std::process::id()));
        let mut seen = std::collections::BTreeSet::new();
        for v in doc["vectors"].as_array().unwrap() {
            if v["expect"]["laptop"] != "accept" || v["decision"] == "reject" {
                continue;
            }
            let id = v["credential"]["id"].as_str().unwrap();
            let Some(key) = doc["pinned_keys"].get(id) else {
                continue;
            };
            let alg = doc["registered_keys"]
                .as_array()
                .unwrap()
                .iter()
                .find(|k| k["credential_id"] == id)
                .unwrap()["cose_alg"]
                .as_i64()
                .unwrap();
            if !seen.insert(alg) {
                continue;
            }
            for label in [-7, -8, -257] {
                let mut entry = key.clone();
                entry["cose_alg"] = label.into();
                std::fs::write(&path, serde_json::json!({"keys":{id:entry}}).to_string()).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
                let loaded = load(&path, id);
                if label != alg {
                    assert!(loaded.is_err());
                    continue;
                }
                let loaded = loaded.unwrap();
                let policy = crate::proof::KeyPolicy {
                    owner: doc["owner"].as_str().unwrap(),
                    rp_id: doc["rp_id"].as_str().unwrap(),
                    origin: doc["origin"].as_str().unwrap(),
                    credential_id: id,
                    spki_der: &loaded.spki_der,
                    fingerprint: &loaded.fingerprint,
                };
                let r = &v["credential"]["response"];
                let assertion = crate::proof::Assertion {
                    credential_id: id,
                    authenticator_data: r["authenticatorData"].as_str().unwrap(),
                    client_data_json: r["clientDataJSON"].as_str().unwrap(),
                    signature: r["signature"].as_str().unwrap(),
                };
                let now = time::OffsetDateTime::parse(
                    v["now"].as_str().unwrap(),
                    &time::format_description::well_known::Rfc3339,
                )
                .unwrap();
                // Execution verifier still requires an approving answer.
                let answer: serde_json::Value =
                    serde_json::from_str(v["answer_json"].as_str().unwrap()).unwrap();
                let result = crate::proof::verify(
                    doc["request"]["request_json"].as_str().unwrap(),
                    v["answer_json"].as_str().unwrap(),
                    &assertion,
                    &policy,
                    now,
                );
                assert_eq!(result.is_ok(), answer["decision"] == "approve");
            }
        }
        assert_eq!(seen, [-257, -8, -7].into_iter().collect());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn pin_is_checked_and_key_removal_holds() {
        let path = std::env::temp_dir().join(format!("task-key-policy-{}", std::process::id()));
        use p256::pkcs8::EncodePublicKey;
        let key = p256::ecdsa::SigningKey::from_slice(&[7; 32]).unwrap();
        let der = key.verifying_key().to_public_key_der().unwrap();
        let bytes = der.as_bytes();
        let pin = format!("{:x}", Sha256::digest(bytes));
        std::fs::write(&path,format!("{{\"keys\":{{\"id\":{{\"spki_b64url\":\"{}\",\"fingerprint\":\"{}\",\"cose_alg\":-7}}}}}}",URL_SAFE_NO_PAD.encode(bytes),pin)).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(load(&path, "id").is_ok());
        assert!(load(&path, "absent").is_err());
        let entry = format!(
            "{{\"spki_b64url\":\"{}\",\"fingerprint\":\"{}\",\"cose_alg\":-7}}",
            URL_SAFE_NO_PAD.encode(bytes),
            pin
        );
        std::fs::write(
            &path,
            format!("{{\"keys\":{{\"id\":{},\"id\":{}}}}}", entry, entry),
        )
        .unwrap();
        assert!(load(&path, "id").is_err());
        std::fs::remove_file(path).unwrap();
    }
}
