use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as b64};
use p256::{
    ecdsa::{Signature, SigningKey, signature::Signer},
    pkcs8::EncodePublicKey,
};
use session_task_admission::proof::{self, Assertion, KeyPolicy};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, macros::datetime};

fn verify_case(
    answer_override: Option<serde_json::Value>,
    corrupt: bool,
    now: OffsetDateTime,
) -> bool {
    let request = r#"{"sequence":1,"topic":"fixture","session_id":"session"}"#;
    let answer = answer_override.unwrap_or_else(|| serde_json::json!({
        "v":1,"type":"work-engine.owner-answer","answer_id":"00000000-0000-4000-8000-000000009003",
        "board_seq":1,"request_digest":format!("{:x}",Sha256::digest(request.as_bytes())),"decision":"approve","comment":"",
        "owner":"owner","created_at":"2026-11-02T00:00:00Z","expires_at":"2026-11-03T00:00:00Z","rp_id":"auth.example"
    }));
    let raw = serde_json::to_string(&answer).unwrap();
    let key = SigningKey::from_slice(&[7; 32]).unwrap();
    let spki = key.verifying_key().to_public_key_der().unwrap();
    let fp = format!("{:x}", Sha256::digest(spki.as_bytes()));
    let client = serde_json::to_vec(&serde_json::json!({"type":"webauthn.get","challenge":b64.encode(Sha256::digest(raw.as_bytes())),"origin":"https://auth.example","crossOrigin":false})).unwrap();
    let mut auth = Sha256::digest(b"auth.example").to_vec();
    auth.extend_from_slice(&[5, 0, 0, 0, 0]);
    let mut message = auth.clone();
    message.extend_from_slice(&Sha256::digest(&client));
    let sig: Signature = key.sign(&message);
    if corrupt {
        auth[32] = 1;
    }
    let encoded_auth = b64.encode(auth);
    let encoded_client = b64.encode(client);
    let encoded_sig = b64.encode(sig.to_der().as_bytes());
    let policy = KeyPolicy {
        owner: "owner",
        rp_id: "auth.example",
        origin: "https://auth.example",
        credential_id: "key",
        spki_der: spki.as_bytes(),
        fingerprint: &fp,
    };
    let assertion = Assertion {
        credential_id: "key",
        authenticator_data: &encoded_auth,
        client_data_json: &encoded_client,
        signature: &encoded_sig,
    };
    match proof::verify(request, &raw, &assertion, &policy, now) {
        Err(_) => false,
        Ok(verified) => {
            let wrong_pin = KeyPolicy {
                fingerprint: "0",
                ..policy
            };
            assert!(proof::verify(request, &raw, &assertion, &wrong_pin, now).is_err());
            let wrong_key = Assertion {
                credential_id: "other",
                ..assertion
            };
            assert!(proof::verify(request, &raw, &wrong_key, &policy, now).is_err());
            let wrong_signature = Assertion {
                signature: "AA",
                ..assertion
            };
            assert!(proof::verify(request, &raw, &wrong_signature, &policy, now).is_err());
            assert!(proof::verify("{\"sequence\":1}", &raw, &assertion, &policy, now).is_err());
            let mut reply = serde_json::json!({"kind":"reply","in_reply_to":1,"topic":"fixture","to":["session:session"],"message_id":"00000000-0000-4000-8000-000000009003","data":{"disposition":"answered","decision":"approve","comment":"","answer_id":"00000000-0000-4000-8000-000000009003","signed_answer":{"format":"work-engine.owner-answer/v1","answer_json":raw,"credential_id":"key","authenticator_data":encoded_auth,"client_data_json":encoded_client,"signature":encoded_sig}}});
            assert!(
                proof::reply_matches(request, &reply.to_string(), &raw, &assertion, &verified)
                    .is_ok()
            );
            reply["data"]["signed_answer"]["signature"] = serde_json::json!("another");
            assert!(
                proof::reply_matches(request, &reply.to_string(), &raw, &assertion, &verified)
                    .is_err()
            );
            let other = Assertion {
                credential_id: "key",
                authenticator_data: &encoded_auth,
                client_data_json: &encoded_client,
                signature: "another",
            };
            assert!(
                proof::reply_matches(request, &reply.to_string(), &raw, &other, &verified).is_err()
            );
            true
        }
    }
}
#[test]
fn authentic_assertion_and_negative_authority_cases() {
    let now = datetime!(2026-11-02 01:00 UTC);
    assert!(verify_case(None, false, now));
    assert!(!verify_case(None, true, now));
    assert!(!verify_case(None, false, datetime!(2026-11-03 00:00 UTC)));
    let bad = serde_json::json!({"v":1,"type":"work-engine.owner-answer","answer_id":"00000000-0000-4000-8000-000000009003","board_seq":1,"request_digest":format!("{:x}", Sha256::digest(br#"{"sequence":1,"topic":"fixture","session_id":"session"}"#)),"decision":"approve","comment":" ","owner":"owner","created_at":"2026-11-02T00:00:00Z","expires_at":"2026-11-03T00:00:00Z","rp_id":"auth.example"});
    assert!(!verify_case(Some(bad), false, now));
}

#[test]
fn shared_owner_answer_thirty_case_laptop_parity() {
    let raw = include_str!("owner-answer-v1.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(raw.as_bytes())),
        "e1ef48422f326f1b691319bdf1b7125711af79ca506cc9d0dbebfef6133a3ad5"
    );
    let doc: serde_json::Value = serde_json::from_str(raw).unwrap();
    let vectors = doc["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 30);
    let mut mismatches = Vec::new();
    for v in vectors {
        let credential = &v["credential"];
        let id = credential["id"].as_str().unwrap();
        let accepted = (|| {
            // These fixture inputs represent the existing current-request and
            // protected laptop pin-policy owners. Registered keys alone do not
            // establish laptop trust, and a closed request is never current.
            if v["request_open"] != true {
                return false;
            }
            let key = &doc["pinned_keys"][id];
            let Some(spki) = key["spki_b64url"].as_str().and_then(|s| b64.decode(s).ok()) else {
                return false;
            };
            let Some(fingerprint) = key["fingerprint"].as_str() else {
                return false;
            };
            let policy = KeyPolicy {
                owner: doc["owner"].as_str().unwrap(),
                rp_id: doc["rp_id"].as_str().unwrap(),
                origin: doc["origin"].as_str().unwrap(),
                credential_id: id,
                spki_der: &spki,
                fingerprint,
            };
            let response = &credential["response"];
            let assertion = Assertion {
                credential_id: id,
                authenticator_data: response["authenticatorData"].as_str().unwrap(),
                client_data_json: response["clientDataJSON"].as_str().unwrap(),
                signature: response["signature"].as_str().unwrap(),
            };
            proof::verify_answer(
                doc["request"]["request_json"].as_str().unwrap(),
                v["answer_json"].as_str().unwrap(),
                &assertion,
                &policy,
                OffsetDateTime::parse(
                    v["now"].as_str().unwrap(),
                    &time::format_description::well_known::Rfc3339,
                )
                .unwrap(),
            )
            .is_ok()
        })();
        if accepted != (v["expect"]["laptop"] == "accept") {
            mismatches.push(v["name"].clone())
        }
    }
    assert!(mismatches.is_empty(), "parity mismatch: {mismatches:?}");
}
