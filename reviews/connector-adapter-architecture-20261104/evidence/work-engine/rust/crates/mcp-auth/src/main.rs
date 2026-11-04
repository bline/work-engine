use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::Deserialize;
use std::{
    fs,
    io::{self, Read},
    time::{Duration, SystemTime},
};

#[derive(Default, PartialEq)]
enum Profile {
    #[default]
    Rfc9068,
    StytchConnectedApps,
}
impl<'de> Deserialize<'de> for Profile {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        match name.as_str() {
            "rfc9068-v1" => Ok(Self::Rfc9068),
            "stytch-connected-apps-v1" => Ok(Self::StytchConnectedApps),
            _ => Err(serde::de::Error::custom("unknown profile")),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    profile: Profile,
    expected_client_id: Option<String>,
    expected_project_id: Option<String>,
    resource: String,
    issuer: String,
    subject: String,
    write_scope: String,
    jwks_snapshot: String,
    jwks_max_age_seconds: u64,
    verifier: String,
}
#[derive(Deserialize)]
struct Claims {
    scope: Option<String>,
    sub: String,
    iss: String,
    exp: u64,
    iat: u64,
    client_id: String,
    jti: String,
    aud: serde_json::Value,
    #[serde(default, deserialize_with = "present_nbf")]
    nbf: Option<u64>,
}
fn present_nbf<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    u64::deserialize(d).map(Some)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    token: String,
    config: Config,
}

fn verify() -> Result<bool, ()> {
    let mut input = Vec::new();
    io::stdin()
        .take(32769)
        .read_to_end(&mut input)
        .map_err(|_| ())?;
    if input.len() > 32768 {
        return Err(());
    }
    let request: Request = serde_json::from_slice(&input).map_err(|_| ())?;
    let c = request.config;
    if c.resource.is_empty()
        || c.issuer.is_empty()
        || c.subject.is_empty()
        || c.write_scope.is_empty()
        || c.write_scope.split_whitespace().count() != 1
        || c.verifier.is_empty()
        || !std::path::Path::new(&c.jwks_snapshot).is_absolute()
        || !(1..=86400).contains(&c.jwks_max_age_seconds)
    {
        return Err(());
    }
    if c.profile == Profile::StytchConnectedApps
        && (c.expected_client_id.as_deref().is_none_or(str::is_empty)
            || c.expected_project_id.as_deref().is_none_or(str::is_empty))
    {
        return Err(());
    }
    let token = request.token.as_str();
    if token.is_empty() || token.len() > 16384 {
        return Err(());
    }
    let header = decode_header(token).map_err(|_| ())?;
    let expected_typ = match c.profile {
        Profile::Rfc9068 => "at+jwt",
        Profile::StytchConnectedApps => "JWT",
    };
    if header.alg != Algorithm::RS256 || header.typ.as_deref() != Some(expected_typ) {
        return Err(());
    }
    let kid = header.kid.ok_or(())?;
    if kid.is_empty() || kid.len() > 256 {
        return Err(());
    }
    let file = fs::File::open(&c.jwks_snapshot).map_err(|_| ())?;
    let meta = file.metadata().map_err(|_| ())?;
    if !meta.is_file()
        || meta.len() > 262144
        || SystemTime::now()
            .duration_since(meta.modified().map_err(|_| ())?)
            .map_err(|_| ())?
            > Duration::from_secs(c.jwks_max_age_seconds)
    {
        return Err(());
    }
    let mut bytes = Vec::new();
    file.take(262145).read_to_end(&mut bytes).map_err(|_| ())?;
    if bytes.len() > 262144 {
        return Err(());
    }
    let keys: JwkSet = serde_json::from_slice(&bytes).map_err(|_| ())?;
    if keys.keys.len() > 32 {
        return Err(());
    }
    let matching: Vec<_> = keys
        .keys
        .iter()
        .filter(|k| k.common.key_id.as_deref() == Some(&kid))
        .collect();
    if matching.len() != 1 {
        return Err(());
    }
    let key = matching[0];
    if key
        .common
        .key_algorithm
        .is_some_and(|a| a != jsonwebtoken::jwk::KeyAlgorithm::RS256)
        || key
            .common
            .public_key_use
            .as_ref()
            .is_some_and(|u| *u != jsonwebtoken::jwk::PublicKeyUse::Signature)
    {
        return Err(());
    }
    if key
        .common
        .key_operations
        .as_ref()
        .is_some_and(|ops| !ops.contains(&jsonwebtoken::jwk::KeyOperations::Verify))
    {
        return Err(());
    }
    let decoding_key = DecodingKey::from_jwk(key).map_err(|_| ())?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 0;
    validation.validate_nbf = true;
    validation.set_issuer(&[&c.issuer]);
    validation.set_audience(&[&c.resource]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    let claims = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|_| ())?
        .claims;
    if claims.sub != c.subject
        || claims.iss != c.issuer
        || claims.exp == 0
        || claims.iat == 0
        || claims.iat > jsonwebtoken::get_current_timestamp()
        || claims.client_id.is_empty()
        || claims.jti.is_empty()
        || claims.aud.is_null()
        || claims
            .nbf
            .is_some_and(|n| n > jsonwebtoken::get_current_timestamp())
    {
        return Err(());
    }
    if c.profile == Profile::StytchConnectedApps {
        let project = c.expected_project_id.as_deref().ok_or(())?;
        let audience_has_project = claims.aud.as_str() == Some(project)
            || claims
                .aud
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(project)));
        if claims.client_id != c.expected_client_id.as_deref().ok_or(())? || !audience_has_project {
            return Err(());
        }
    }
    Ok(claims
        .scope
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .any(|s| s == c.write_scope))
}
fn main() {
    match verify() {
        Ok(true) => println!("allow"),
        Ok(false) => println!("scope"),
        Err(()) => println!("invalid"),
    }
}
