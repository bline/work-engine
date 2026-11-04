"""Verify a Work Engine owner answer (format v1) signed with an approval passkey.

Reference verifier for docs/owner-answer-v1.md. Standard library plus
`cryptography`. Raises AnswerRejected with a short code; returns the parsed
answer only when every check passes.
"""
from __future__ import annotations

import base64
import datetime as dt
import hashlib
import json
import re

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec, ed25519, padding, rsa

RP_ID = "auth.devwits.ai"
ORIGIN = "https://auth.devwits.ai"
ANSWER_TYPE = "work-engine.owner-answer"
FIELDS = ("v", "type", "answer_id", "board_seq", "request_digest", "decision", "comment", "owner",
          "created_at", "expires_at", "rp_id")
DECISIONS = ("approve", "reject", "defer", "ask")
DISPOSITIONS = {"approve": "answered", "reject": "declined", "defer": "deferred", "ask": "partial"}
MAX_LIFETIME = dt.timedelta(days=7)
CLOCK_SKEW = dt.timedelta(minutes=10)
FLAG_USER_PRESENT = 0x01
FLAG_USER_VERIFIED = 0x04


class AnswerRejected(Exception):
    def __init__(self, code: str, detail: str = ""):
        super().__init__(f"{code}: {detail}" if detail else code)
        self.code, self.detail = code, detail


def b64url_decode(text: str) -> bytes:
    if not isinstance(text, str) or not re.fullmatch(r"[A-Za-z0-9_-]*", text):
        raise AnswerRejected("encoding")
    return base64.urlsafe_b64decode(text + "=" * (-len(text) % 4))


def b64url_encode(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()


def request_json(row: dict) -> str:
    """The exact bytes an answer's request_digest covers: the full board row."""
    return json.dumps(row, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def request_digest(row: dict) -> str:
    return hashlib.sha256(request_json(row).encode("utf-8")).hexdigest()


def key_fingerprint(spki_der: bytes) -> str:
    return hashlib.sha256(spki_der).hexdigest()


def _time(value, name: str) -> dt.datetime:
    try:
        parsed = dt.datetime.fromisoformat(str(value).replace("Z", "+00:00"))
    except ValueError:
        raise AnswerRejected(name) from None
    if parsed.tzinfo is None:
        raise AnswerRejected(name)
    return parsed


def check_fields(answer_json: str, *, owner: str, rp_id: str, now: dt.datetime) -> dict:
    try:
        answer = json.loads(answer_json)
    except ValueError:
        raise AnswerRejected("answer_json") from None
    if not isinstance(answer, dict) or set(answer) != set(FIELDS):
        raise AnswerRejected("fields")
    if answer["v"] != 1 or answer["type"] != ANSWER_TYPE:
        raise AnswerRejected("version_or_type")
    if not re.fullmatch(r"[0-9a-f-]{36}", str(answer["answer_id"])):
        raise AnswerRejected("answer_id")
    if not isinstance(answer["board_seq"], int) or isinstance(answer["board_seq"], bool) or answer["board_seq"] < 1:
        raise AnswerRejected("board_seq")
    if not re.fullmatch(r"[0-9a-f]{64}", str(answer["request_digest"])):
        raise AnswerRejected("request_digest")
    if answer["decision"] not in DECISIONS:
        raise AnswerRejected("decision")
    if not isinstance(answer["comment"], str) or len(answer["comment"]) > 4000:
        raise AnswerRejected("comment")
    if answer["decision"] == "ask" and not answer["comment"].strip():
        raise AnswerRejected("ask_needs_comment")
    if answer["owner"] != owner:
        raise AnswerRejected("owner")
    if answer["rp_id"] != rp_id:
        raise AnswerRejected("rp_id")
    created = _time(answer["created_at"], "created_at")
    expires = _time(answer["expires_at"], "expires_at")
    if created - now > CLOCK_SKEW:
        raise AnswerRejected("created_at", "in the future")
    if expires <= now:
        raise AnswerRejected("expired")
    if expires - created > MAX_LIFETIME:
        raise AnswerRejected("expires_at", "lifetime over 7 days")
    return answer


def _verify_signature(spki_der: bytes, signature: bytes, message: bytes) -> None:
    key = serialization.load_der_public_key(spki_der)
    try:
        if isinstance(key, ec.EllipticCurvePublicKey):
            key.verify(signature, message, ec.ECDSA(hashes.SHA256()))
        elif isinstance(key, ed25519.Ed25519PublicKey):
            key.verify(signature, message)
        elif isinstance(key, rsa.RSAPublicKey):
            key.verify(signature, message, padding.PKCS1v15(), hashes.SHA256())
        else:
            raise AnswerRejected("key_type")
    except InvalidSignature:
        raise AnswerRejected("signature_invalid") from None


def verify_answer(answer_json: str, credential: dict, *, pinned_keys: dict, owner: str,
                  request_row: dict | None, now: dt.datetime | None = None,
                  rp_id: str = RP_ID, origin: str = ORIGIN) -> dict:
    """Check an answer and its WebAuthn assertion.

    pinned_keys maps credential id -> {"spki_b64url", "fingerprint"}; only keys the
    owner pinned on this machine count. request_row is the board row the answer
    names, or None when that request is no longer open.
    """
    now = now or dt.datetime.now(dt.timezone.utc)
    answer = check_fields(answer_json, owner=owner, rp_id=rp_id, now=now)
    if request_row is None or request_row.get("sequence") != answer["board_seq"]:
        raise AnswerRejected("request_not_open")
    if request_digest(request_row) != answer["request_digest"]:
        raise AnswerRejected("request_changed")

    if not isinstance(credential, dict) or not isinstance(credential.get("response"), dict):
        raise AnswerRejected("credential")
    pin = pinned_keys.get(credential.get("id"))
    if pin is None:
        raise AnswerRejected("key_not_pinned")
    spki = b64url_decode(pin["spki_b64url"])
    if key_fingerprint(spki) != pin["fingerprint"]:
        raise AnswerRejected("pin_corrupt")

    response = credential["response"]
    client_data = b64url_decode(response.get("clientDataJSON", ""))
    auth_data = b64url_decode(response.get("authenticatorData", ""))
    signature = b64url_decode(response.get("signature", ""))
    try:
        client = json.loads(client_data)
    except ValueError:
        raise AnswerRejected("client_data") from None
    if client.get("type") != "webauthn.get":
        raise AnswerRejected("client_data", "type")
    expected = b64url_encode(hashlib.sha256(answer_json.encode("utf-8")).digest())
    if client.get("challenge") != expected:
        raise AnswerRejected("challenge")
    if client.get("origin") != origin:
        raise AnswerRejected("origin")
    if client.get("crossOrigin"):
        raise AnswerRejected("cross_origin")
    if len(auth_data) < 37 or auth_data[:32] != hashlib.sha256(rp_id.encode()).digest():
        raise AnswerRejected("rp_id_hash")
    flags = auth_data[32]
    if not flags & FLAG_USER_PRESENT or not flags & FLAG_USER_VERIFIED:
        raise AnswerRejected("user_verification")
    _verify_signature(spki, signature, auth_data + hashlib.sha256(client_data).digest())
    return answer
