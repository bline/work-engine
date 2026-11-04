//! Narrow native-subscription access projection. No function here opens a native
//! credential source. The parent must separately admit that future operation.
use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::os::unix::fs::OpenOptionsExt;

/// Operator-bound source references. Public configuration is inventoried;
/// native credentials are never an Artifact, tree inventory, or export.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaneInputs {
    pub pi_config: Tree,
    pub native_auth_source: PathBuf,
}
impl LaneInputs {
    pub fn validate_references(&self) -> Result<()> {
        self.pi_config.verify()?;
        canonical_directory(&self.pi_config.root)?;
        let source = &self.native_auth_source;
        if !source.is_absolute()
            || source.to_string_lossy().contains([',', ':', '\n', '\r'])
            || fs::canonicalize(source)? != *source
            || source.starts_with(&self.pi_config.root)
            || !fs::symlink_metadata(source)?.is_file()
            || fs::metadata(source)?.permissions().mode() & 0o077 != 0
        {
            return refuse("native auth source/public config references conflict");
        }
        Ok(())
    }
}
#[derive(Deserialize)]
struct NativeCache {
    auth_mode: String,
    #[serde(rename = "OPENAI_API_KEY")]
    api_key: Option<String>,
    tokens: NativeTokens,
}
#[derive(Deserialize)]
struct NativeTokens {
    access_token: String,
    account_id: String,
}
#[derive(Deserialize)]
struct Claims {
    exp: u64,
    #[serde(rename = "https://api.openai.com/auth")]
    auth: Account,
}
#[derive(Deserialize)]
struct Account {
    chatgpt_account_id: String,
}

/// Deliberately implements neither Debug nor Serialize. Only the explicit
/// private staging operation can encode the three permitted values.
pub struct AccessGrant {
    access: String,
    expires_ms: u64,
    account_id: String,
}
impl AccessGrant {
    /// Write access-only data into an explicitly private, non-exported staging
    /// directory. Never returns a credential digest or includes values in errors.
    pub fn stage(&self, directory: &Path) -> Result<PathBuf> {
        canonical_directory(directory)?;
        if fs::metadata(directory)?.permissions().mode() & 0o077 != 0 {
            return refuse("access staging directory must be private");
        }
        let path = directory.join("access.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        serde_json::to_writer(
            &mut file,
            &json!({"access":self.access,"expires_ms":self.expires_ms,"account_id":self.account_id}),
        )?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        File::open(directory)?.sync_all()?;
        Ok(path)
    }
}
/// Exact current native cache shape, established by Root's metadata observation.
/// JWT decoding checks structure/expiry/account consistency, not signature or
/// backend/account eligibility. It does not refresh, use or copy id/refresh tokens.
pub fn project_native_access(
    raw: &[u8],
    now_seconds: u64,
    deadline_seconds: u64,
) -> Result<AccessGrant> {
    if raw.is_empty()
        || raw.len() > 65536
        || deadline_seconds < now_seconds
        || deadline_seconds.saturating_sub(now_seconds) > 10800
    {
        return refuse("native access input/deadline bound");
    }
    let cache: NativeCache = serde_json::from_slice(raw)
        .map_err(|_| Error::Refused("native cache shape invalid".into()))?;
    if cache.auth_mode != "chatgpt"
        || cache.api_key.as_deref().is_some_and(|s| !s.is_empty())
        || cache.tokens.access_token.len() > 32768
        || cache.tokens.account_id.is_empty()
        || cache.tokens.account_id.len() > 512
    {
        return refuse("subscription-only native credential required");
    }
    let parts: Vec<_> = cache.tokens.access_token.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|s| s.is_empty()) {
        return refuse("native access JWT structure invalid");
    }
    let header = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| Error::Refused("native JWT header invalid".into()))?;
    let header: Value = serde_json::from_slice(&header)
        .map_err(|_| Error::Refused("native JWT header invalid".into()))?;
    if header["alg"]
        .as_str()
        .is_none_or(|alg| alg.is_empty() || alg.eq_ignore_ascii_case("none"))
        || URL_SAFE_NO_PAD.decode(parts[2]).is_err()
    {
        return refuse("native JWT header/signature structure invalid");
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| Error::Refused("native JWT encoding invalid".into()))?;
    let claims: Claims = serde_json::from_slice(&decoded)
        .map_err(|_| Error::Refused("native JWT claims invalid".into()))?;
    // Pi's own OAuth minimum is five minutes. Cover the whole bounded job plus
    // that minimum so its resolver never needs a refresh during this admission.
    let minimum = deadline_seconds
        .checked_add(300)
        .ok_or_else(|| Error::Refused("native deadline overflow".into()))?;
    if claims.exp <= minimum || claims.auth.chatgpt_account_id != cache.tokens.account_id {
        return refuse("native access expiry/account conflict");
    }
    let expires_ms = claims
        .exp
        .checked_mul(1000)
        .ok_or_else(|| Error::Refused("native expiry overflow".into()))?;
    Ok(AccessGrant {
        access: cache.tokens.access_token,
        expires_ms,
        account_id: cache.tokens.account_id,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cache(exp: u64, account: &str, key: Value) -> Vec<u8> {
        let payload = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(
                &json!({"exp":exp,"https://api.openai.com/auth":{"chatgpt_account_id":account}}),
            )
            .unwrap(),
        );
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let signature = URL_SAFE_NO_PAD.encode([0_u8; 32]);
        serde_json::to_vec(&json!({"auth_mode":"chatgpt","OPENAI_API_KEY":key,"tokens":{"access_token":format!("{header}.{payload}.{signature}"),"account_id":"fake-account","id_token":"ID_TOKEN_MUST_NOT_STAGE","refresh_token":"REFRESH_MUST_NOT_STAGE"},"last_refresh":"fixture"})).unwrap()
    }
    #[test]
    fn projects_only_access_expiry_and_account_into_private_new_file() {
        let grant =
            project_native_access(&cache(2000, "fake-account", Value::Null), 1000, 1100).unwrap();
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let staged = grant.stage(temp.path()).unwrap();
        let bytes = fs::read(staged).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert_eq!(value["expires_ms"], 2000000);
        assert!(!String::from_utf8_lossy(&bytes).contains("MUST_NOT_STAGE"));
        assert!(grant.stage(temp.path()).is_err());
    }
    #[test]
    fn refuses_api_key_expiry_account_mismatch_and_malformed_without_secret_errors() {
        for raw in [
            cache(2000, "fake-account", json!("API_SECRET_MUST_NOT_LOG")),
            cache(1300, "fake-account", Value::Null),
            cache(2000, "different-account", Value::Null),
            b"{broken_SECRET_MUST_NOT_LOG".to_vec(),
        ] {
            let error = match project_native_access(&raw, 1000, 1100) {
                Ok(_) => panic!("invalid cache accepted"),
                Err(e) => e,
            };
            assert!(!error.to_string().contains("MUST_NOT_LOG"));
        }
    }
    #[test]
    fn explicit_lane_references_keep_public_configuration_and_secret_source_disjoint() {
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("config");
        fs::create_dir(&config).unwrap();
        let manifest = temp.path().join("config-manifest.json");
        let bytes = serde_json::to_vec(&inventory(&config).unwrap()).unwrap();
        fs::write(&manifest, &bytes).unwrap();
        let public = Tree {
            root: config.clone(),
            manifest: Artifact {
                path: manifest,
                sha256: digest(&bytes),
            },
        };
        let auth = temp.path().join("operator-native-cache.json");
        fs::write(&auth, b"unread fake content").unwrap();
        fs::set_permissions(&auth, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(
            LaneInputs {
                pi_config: public.clone(),
                native_auth_source: auth
            }
            .validate_references()
            .is_ok()
        );
        fs::write(config.join("secret.json"), b"not public").unwrap();
        assert!(
            LaneInputs {
                pi_config: public,
                native_auth_source: config.join("secret.json")
            }
            .validate_references()
            .is_err()
        );
    }
}
