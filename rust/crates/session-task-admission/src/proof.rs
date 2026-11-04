//! Pure verifier for pinned ES256, EdDSA and RS256 owner-answer/v1 assertions. Key-policy provenance and
//! authoritative board access are separate protected deployment obligations.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::{
    ecdsa::{Signature, VerifyingKey, signature::Verifier},
    pkcs8::DecodePublicKey,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answer {
    v: u8,
    r#type: String,
    answer_id: String,
    board_seq: u64,
    request_digest: String,
    decision: String,
    comment: String,
    owner: String,
    created_at: String,
    expires_at: String,
    rp_id: String,
}
#[derive(Deserialize)]
struct Client {
    r#type: String,
    challenge: String,
    origin: String,
    #[serde(default, rename = "crossOrigin")]
    cross_origin: Option<bool>,
}

/// Values here must originate in a protected, operator-configured key policy,
/// never in an untrusted request. Unsupported key algorithms fail closed.
pub struct KeyPolicy<'a> {
    pub owner: &'a str,
    pub rp_id: &'a str,
    pub origin: &'a str,
    pub credential_id: &'a str,
    pub spki_der: &'a [u8],
    pub fingerprint: &'a str,
}
pub struct Assertion<'a> {
    pub credential_id: &'a str,
    pub authenticator_data: &'a str,
    pub client_data_json: &'a str,
    pub signature: &'a str,
}
/// A signature result is not an admission or evidence that a board reply is current.
pub struct VerifiedAnswer {
    pub(crate) answer_id: String,
    pub(crate) answer_digest: String,
    pub(crate) assertion_digest: String,
    pub(crate) board_seq: u64,
    pub(crate) request_digest: String,
    pub(crate) expires_at: OffsetDateTime,
    pub(crate) decision: String,
    pub(crate) comment: String,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ProofError;
fn dec(s: &str) -> Result<Vec<u8>, ProofError> {
    URL_SAFE_NO_PAD.decode(s).map_err(|_| ProofError)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn assertion_digest(assertion: &Assertion<'_>) -> String {
    let mut h = Sha256::new();
    for s in [
        assertion.credential_id,
        assertion.authenticator_data,
        assertion.client_data_json,
        assertion.signature,
    ] {
        h.update((s.len() as u64).to_be_bytes());
        h.update(s.as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// Existing execution-facing verifier keeps the approval-only policy.
pub fn verify(
    raw_request: &str,
    raw_answer: &str,
    assertion: &Assertion<'_>,
    policy: &KeyPolicy<'_>,
    now: OffsetDateTime,
) -> Result<VerifiedAnswer, ProofError> {
    let verified = verify_answer(raw_request, raw_answer, assertion, policy, now)?;
    if verified.decision != "approve" || !verified.comment.is_empty() {
        return Err(ProofError);
    }
    Ok(verified)
}

/// Signature validity is data, distinct from execution admission.
pub fn verify_answer(
    raw_request: &str,
    raw_answer: &str,
    assertion: &Assertion<'_>,
    policy: &KeyPolicy<'_>,
    now: OffsetDateTime,
) -> Result<VerifiedAnswer, ProofError> {
    if raw_request.len() > 1024 * 1024
        || raw_answer.len() > 65536
        || policy.spki_der.len() > 16384
        || assertion.authenticator_data.len() > 8192
        || assertion.client_data_json.len() > 32768
        || assertion.signature.len() > 16384
    {
        return Err(ProofError);
    }
    let a: Answer = serde_json::from_str(raw_answer).map_err(|_| ProofError)?;
    #[derive(Deserialize)]
    struct Request {
        sequence: u64,
    }
    let request: Request = serde_json::from_str(raw_request).map_err(|_| ProofError)?;
    let created = OffsetDateTime::parse(&a.created_at, &Rfc3339).map_err(|_| ProofError)?;
    let expires = OffsetDateTime::parse(&a.expires_at, &Rfc3339).map_err(|_| ProofError)?;
    if a.v != 1
        || a.r#type != "work-engine.owner-answer"
        || uuid::Uuid::parse_str(&a.answer_id).is_err_and(|_| true)
        || uuid::Uuid::parse_str(&a.answer_id)
            .map(|u| u.hyphenated().to_string())
            .ok()
            .as_deref()
            != Some(&a.answer_id)
        || a.board_seq == 0
        || a.board_seq != request.sequence
        || a.request_digest != digest(raw_request.as_bytes())
        || !["approve", "reject", "defer", "ask"].contains(&a.decision.as_str())
        || (a.decision == "ask" && a.comment.trim().is_empty())
        || a.comment.chars().count() > 4000
        || a.owner != policy.owner
        || a.rp_id != policy.rp_id
        || created > now + time::Duration::minutes(10)
        || expires <= now
        || expires <= created
        || expires - created > time::Duration::days(7)
        || assertion.credential_id != policy.credential_id
        || digest(policy.spki_der) != policy.fingerprint
    {
        return Err(ProofError);
    }
    let auth = dec(assertion.authenticator_data)?;
    let client_bytes = dec(assertion.client_data_json)?;
    let signature = dec(assertion.signature)?;
    let client: Client = serde_json::from_slice(&client_bytes).map_err(|_| ProofError)?;
    let client_value: serde_json::Value =
        serde_json::from_slice(&client_bytes).map_err(|_| ProofError)?;
    if client_value.get("crossOrigin").is_some_and(|v| v != false) {
        return Err(ProofError);
    }
    if client.r#type != "webauthn.get"
        || client.challenge != URL_SAFE_NO_PAD.encode(Sha256::digest(raw_answer.as_bytes()))
        || client.origin != policy.origin
        || client.cross_origin.unwrap_or(false)
        || auth.len() < 37
        || auth[..32] != Sha256::digest(policy.rp_id.as_bytes())[..]
        || auth[32] & 0x05 != 0x05
    {
        return Err(ProofError);
    }
    let mut signed = auth;
    signed.extend_from_slice(&Sha256::digest(&client_bytes));
    if let Ok(key) = VerifyingKey::from_public_key_der(policy.spki_der) {
        let signature = Signature::from_der(&signature).map_err(|_| ProofError)?;
        key.verify(&signed, &signature).map_err(|_| ProofError)?;
    } else {
        use p256::pkcs8::{der::Decode, spki::SubjectPublicKeyInfoRef};
        let spki = SubjectPublicKeyInfoRef::from_der(policy.spki_der).map_err(|_| ProofError)?;
        let key = spki.subject_public_key.as_bytes().ok_or(ProofError)?;
        let algorithm: &dyn ring::signature::VerificationAlgorithm =
            match spki.algorithm.oid.to_string().as_str() {
                "1.3.101.112" if spki.algorithm.parameters.is_none() => &ring::signature::ED25519,
                "1.2.840.113549.1.1.1"
                    if spki
                        .algorithm
                        .parameters
                        .is_none_or(|p| p.decode_as::<p256::pkcs8::der::asn1::Null>().is_ok()) =>
                {
                    &ring::signature::RSA_PKCS1_2048_8192_SHA256
                }
                _ => return Err(ProofError),
            };
        ring::signature::UnparsedPublicKey::new(algorithm, key)
            .verify(&signed, &signature)
            .map_err(|_| ProofError)?;
    }
    Ok(VerifiedAnswer {
        answer_id: a.answer_id,
        answer_digest: digest(raw_answer.as_bytes()),
        assertion_digest: assertion_digest(assertion),
        board_seq: a.board_seq,
        request_digest: a.request_digest,
        expires_at: expires,
        decision: a.decision,
        comment: a.comment,
    })
}

/// Check that the fetched authoritative board reply contains the very assertion
/// just verified. This does not establish that the caller fetched the current
/// authoritative board state, or that the task remains eligible.
pub fn reply_matches(
    raw_request: &str,
    raw_reply: &str,
    raw_answer: &str,
    assertion: &Assertion<'_>,
    verified: &VerifiedAnswer,
) -> Result<(), ProofError> {
    #[derive(Deserialize)]
    struct Request {
        sequence: u64,
        topic: String,
        session_id: String,
    }
    #[derive(Deserialize)]
    struct Signed {
        format: String,
        answer_json: String,
        credential_id: String,
        authenticator_data: String,
        client_data_json: String,
        signature: String,
    }
    #[derive(Deserialize)]
    struct Data {
        disposition: String,
        decision: String,
        comment: String,
        answer_id: String,
        signed_answer: Signed,
    }
    #[derive(Deserialize)]
    struct Reply {
        kind: String,
        in_reply_to: u64,
        topic: String,
        to: Vec<String>,
        message_id: String,
        data: Data,
    }
    let request: Request = serde_json::from_str(raw_request).map_err(|_| ProofError)?;
    let reply: Reply = serde_json::from_str(raw_reply).map_err(|_| ProofError)?;
    let signed = reply.data.signed_answer;
    if verified.decision != "approve"
        || !verified.comment.is_empty()
        || assertion_digest(assertion) != verified.assertion_digest
        || digest(raw_request.as_bytes()) != verified.request_digest
        || request.sequence != verified.board_seq
        || reply.in_reply_to != request.sequence
        || reply.kind != "reply"
        || reply.topic != request.topic
        || reply.to != [format!("session:{}", request.session_id)]
        || reply.message_id != verified.answer_id
        || reply.data.answer_id != verified.answer_id
        || reply.data.disposition != "answered"
        || reply.data.decision != "approve"
        || !reply.data.comment.is_empty()
        || signed.format != "work-engine.owner-answer/v1"
        || signed.answer_json != raw_answer
        || digest(raw_answer.as_bytes()) != verified.answer_digest
        || signed.credential_id != assertion.credential_id
        || signed.authenticator_data != assertion.authenticator_data
        || signed.client_data_json != assertion.client_data_json
        || signed.signature != assertion.signature
    {
        return Err(ProofError);
    }
    Ok(())
}
