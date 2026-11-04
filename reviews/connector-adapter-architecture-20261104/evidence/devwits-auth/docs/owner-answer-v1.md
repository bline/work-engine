# Owner answer, format v1

An owner answer is the owner's decision on one coordination-board request,
signed with the owner's approval passkey over the answer's exact bytes. It lets
any verifier on the laptop check that the owner, and not an agent, made that
decision about that exact request.

Reference verifier: [`laptop/owner_answer.py`](../laptop/owner_answer.py)
(`verify_answer`), with tests in `laptop/test_owner_answer.py`.

## Flow

1. `approval_sync.py push` sends open board requests addressed to the owner
   (default addressee `role:owner`) to the private mailbox at
   `https://auth.devwits.ai`. Each request is sent as `request_json`, the
   exact bytes its digest covers.
2. The owner signs in (Stytch, passkey), opens `/approve`, reads a request,
   chooses a decision and signs. The page shows a request only if
   SHA-256(`request_json`) equals its `request_digest`.
3. The mailbox checks the answer and its signature, then stores it.
4. `approval_sync.py pull` verifies every answer again against keys pinned on
   the laptop, and posts each valid one to the board as a `reply` to the
   request. Invalid answers are recorded as rejected and never posted.

The mailbox is a transport. Neither its storage nor its token can produce an
answer that verifies.

## The answer document

`answer_json` is a UTF-8 JSON object with exactly these fields:

| field | value |
|---|---|
| `v` | `1` |
| `type` | `"work-engine.owner-answer"` |
| `answer_id` | lowercase UUID, unique per answer; also the board reply's `message_id` |
| `board_seq` | sequence number of the board request answered |
| `request_digest` | lowercase hex SHA-256 of `request_json(row)` for that request (below) |
| `decision` | `approve`, `reject`, `defer` or `ask` |
| `comment` | string, at most 4,000 characters; required (non-blank) for `ask` |
| `owner` | the owner's Stytch user id, `user-test-8b1b7b52-0fa4-4b70-8594-b2b043592a1c` |
| `created_at` | ISO 8601 UTC time of signing |
| `expires_at` | ISO 8601 UTC; after this the answer must not be acted on; at most 7 days after `created_at` (the page uses 24 h) |
| `rp_id` | `"auth.devwits.ai"` |

`request_json(row)` is the full board row as stored, serialised as
`json.dumps(row, sort_keys=True, separators=(",", ":"), ensure_ascii=False)`.
Board rows are append-only, so the digest of a request never changes.

The signature covers the exact `answer_json` string. Verifiers hash that string
as given and never re-serialise the parsed object.

## The signature

A WebAuthn authentication (`navigator.credentials.get`) with:

- `challenge` = SHA-256 of the UTF-8 bytes of `answer_json`;
- RP ID `auth.devwits.ai`, origin `https://auth.devwits.ai`;
- user verification required;
- `allowCredentials` limited to registered approval passkeys.

Approval passkeys are registered at `/approve/setup` with user handle
`approvals:<owner id>`, separate from the Stytch sign-in passkey. The site keeps
each key's public key as SPKI DER (`spki_b64url`) and its fingerprint,
SHA-256 hex of the SPKI bytes.

## Board reply

`approval_sync.py pull` posts:

- `kind` `reply`, `in_reply_to` = `board_seq`, `topic` = the request's topic,
  `to` = `session:<request session_id>`;
- `author` `owner-signed-answer`, `session_id`
  `uuid5(NAMESPACE_URL, "https://auth.devwits.ai/approval-sync")`,
  `message_id` = `answer_id`;
- `data.disposition`: `approve` → `answered`, `reject` → `declined`,
  `defer` → `deferred`, `ask` → `partial`. Only `answered` and `declined`
  close the request;
- `data.decision`, `data.comment`, `data.answer_id`;
- `data.signed_answer`: `format` (`work-engine.owner-answer/v1`), `answer_json`,
  `credential_id`, `authenticator_data`, `client_data_json`, `signature`
  (the last three base64url, unpadded, exactly as the browser returned them);
- `data.verified`: who verified it, the key fingerprint and when. This field is
  informational: a consumer must verify the signature itself.

## Verifying

A consumer that acts on an owner answer checks, at the time it acts:

1. `answer_json` parses to an object with exactly the fields above and valid
   values; `owner` and `rp_id` match; `created_at` is not more than 10 minutes in
   the future; `expires_at` is in the future and at most 7 days after
   `created_at`.
2. The request `board_seq` is still open, and `request_digest` equals the digest
   of its board row now.
3. `credential_id` is a key the owner pinned on this machine
   (`~/.config/devwits/approval-keys.json`, below), and the pinned fingerprint
   matches the pinned SPKI.
4. `client_data_json`: `type` is `webauthn.get`, `challenge` equals
   base64url(SHA-256(`answer_json`)), `origin` is `https://auth.devwits.ai`, and
   `crossOrigin` is absent or false.
5. `authenticator_data`: the first 32 bytes are SHA-256(`auth.devwits.ai`), and
   the flags byte has user-present (0x01) and user-verified (0x04) set.
6. `signature` verifies over `authenticator_data || SHA-256(client_data_json)`
   with the pinned key: ES256 (ECDSA P-256, DER signature), EdDSA (Ed25519) or
   RS256 (PKCS#1 v1.5).
7. `answer_id` has not been acted on before.

The signature counter is not checked: synced passkeys commonly report 0.

### Qualified present-task consumer addendum

The approved phase 1 consumer contract refines check 2 for typed
`dot.task-intent/v1-proposal` requests. This addendum changes neither the answer
format nor the laptop transport's verification against an open request before
posting a reply. Its execution use remains disabled until the qualified
release/profile and exact task activation are approved.

For that consumer, check 2 above may accept the immutable request closed by
**this exact independently verified approving board reply**. The authoritative
reply must be available; an answer merely stored in the mailbox is insufficient.
The exact task and proposal reference (ID, revision and digest), trusted key,
authority, deadlines, permitted actor/effects and canonical budget reservation
must still be current. Withdrawal, supersession, another answer or conflicting
admission does not qualify. The first profile holds every nonempty signed
comment and requires complete supported task terms in the review view.

Atomically bind `answer_id` and exact intent to one admission/outbox intent.
An exact retry returns that same admission without another native effect;
changed content under an existing identity conflicts. A lost native start
acknowledgment remains unknown and cannot justify another launch. Existing
signature, pinned-key, RP/origin, user-verification and expiry checks still apply.
See [typed task boundary and observation transport](task-ui.md).

## Pinned keys

`~/.config/devwits/approval-keys.json` (mode 600):

```json
{"keys": {"<credential_id>": {"spki_b64url": "…", "fingerprint": "<sha256 hex>",
  "cose_alg": -7, "backed_up": true, "pinned_at": "…"}}}
```

A key is trusted only after the owner pins it by fingerprint with
`approval_sync.py pin <prefix>`, comparing it with the fingerprint shown on
`/approve/setup`. Keys registered on the site but not pinned are never trusted.

## Limits

Execution consumers independently verify exact signed answers against
owner-pinned trust policy. This release assumes a trusted workstation account,
verifier/configuration and production publisher. It does not establish
resistance to unrestricted software with the same privileges, or a malicious
signing-page publisher. Native execution additionally requires its separately
qualified, currently authorized profile and exact task approval.

The approval passkey lives in Proton Pass, which syncs it to the laptop:
something with full control of the owner's account and the vault's unlock
could sign. A hardware security key can be registered later as another
approval passkey with no format change.
