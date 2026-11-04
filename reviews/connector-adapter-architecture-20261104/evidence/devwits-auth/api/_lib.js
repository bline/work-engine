// Shared helpers for the approval mailbox API. Files starting with "_" are not
// deployed as routes.
//
// Trust model: the mailbox is a transport, not an authority. The laptop pushes
// open board requests; the owner's signed answers come back. Every answer is
// signed by the owner's approval passkey over its exact bytes, and the laptop
// verifies that signature again before anything reaches the board. This server
// checks the same things early so mistakes surface while the owner is looking.
import {
  createHash,
  createPublicKey,
  randomUUID,
  timingSafeEqual,
} from "node:crypto";
import { createRemoteJWKSet, jwtVerify } from "jose";
import {
  decodeCredentialPublicKey,
  cose,
} from "@simplewebauthn/server/helpers";
export {
  latestPush,
  storePush,
  approvalKeys,
  storeApprovalKey,
  deleteApprovalKey,
  answers,
  storeAnswer,
  deleteAnswer,
  storeChallenge,
  consumeChallenge,
} from "./_storage.js";

export const PROJECT_ID = "project-test-5f16c2cc-3020-4a9a-93af-b3a0f22aa9bd";
export const PROJECT_DOMAIN =
  "https://checkered-replace-0099.customers.stytch.dev";
export const OWNER_USER_ID = "user-test-8b1b7b52-0fa4-4b70-8594-b2b043592a1c";
export const RP_ID = "auth.devwits.ai";
export const ORIGIN = "https://auth.devwits.ai";
export const ANSWER_TYPE = "work-engine.owner-answer";
export const DECISIONS = ["approve", "reject", "defer", "ask"];

const JWKS = createRemoteJWKSet(
  new URL(`https://test.stytch.com/v1/sessions/jwks/${PROJECT_ID}`),
);
const ISSUERS = [`stytch.com/${PROJECT_ID}`, PROJECT_DOMAIN];

export class HttpError extends Error {
  constructor(status, code, detail) {
    super(code);
    this.status = status;
    this.code = code;
    this.detail = detail;
  }
}

export function json(status, body) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Cache-Control": "no-store",
    },
  });
}

// Wrap a handler so thrown HttpErrors become JSON replies and nothing else leaks.
export function route(handler) {
  return async (request) => {
    try {
      return await handler(request);
    } catch (error) {
      if (error instanceof HttpError) {
        return json(error.status, {
          error: error.code,
          ...(error.detail ? { detail: error.detail } : {}),
        });
      }
      console.error(error);
      return json(500, { error: "internal_error" });
    }
  };
}

function bearer(request) {
  const header = request.headers.get("authorization") || "";
  const match = header.match(/^Bearer (.+)$/);
  if (!match) throw new HttpError(401, "missing_bearer_token");
  return match[1];
}

// The owner, signed in on this site: a Stytch session JWT for the pinned user.
export async function requireOwner(request) {
  let payload;
  try {
    ({ payload } = await jwtVerify(bearer(request), JWKS, {
      issuer: ISSUERS,
      audience: PROJECT_ID,
      algorithms: ["RS256"],
    }));
  } catch (error) {
    if (error instanceof HttpError) throw error;
    throw new HttpError(
      401,
      "session_invalid",
      error.code || "verification_failed",
    );
  }
  if (payload.sub !== OWNER_USER_ID) throw new HttpError(403, "not_owner");
  return payload;
}

// The laptop sync job: a shared bearer token. Holding it lets a caller push
// requests and collect answers, never create or alter a signed answer.
export function requireLaptop(request) {
  const expected = process.env.MAILBOX_TOKEN;
  if (!expected) throw new HttpError(503, "mailbox_token_not_configured");
  const given = Buffer.from(bearer(request));
  const wanted = Buffer.from(expected);
  if (given.length !== wanted.length || !timingSafeEqual(given, wanted)) {
    throw new HttpError(401, "laptop_token_invalid");
  }
}

export async function readBody(request, maxBytes = 512 * 1024) {
  const text = await request.text();
  if (text.length > maxBytes) throw new HttpError(413, "body_too_large");
  try {
    return JSON.parse(text);
  } catch {
    throw new HttpError(400, "body_not_json");
  }
}

export const sha256Hex = (text) =>
  createHash("sha256").update(text, "utf8").digest("hex");
export const sha256B64url = (text) =>
  createHash("sha256").update(text, "utf8").digest("base64url");
export const b64url = (bytes) => Buffer.from(bytes).toString("base64url");
export const fromB64url = (text) =>
  new Uint8Array(Buffer.from(text, "base64url"));
export const newId = () => randomUUID();

export function publicKeyView(key) {
  const { cose_b64url, ...rest } = key;
  return rest;
}

// COSE public key (from the attestation) -> SubjectPublicKeyInfo DER, which the
// laptop verifier loads with ordinary crypto libraries.
export function coseToSpki(coseBytes) {
  const key = decodeCredentialPublicKey(coseBytes);
  const kty = key.get(cose.COSEKEYS.kty);
  const alg = key.get(cose.COSEKEYS.alg);
  let jwk;
  if (kty === cose.COSEKTY.EC2) {
    if (key.get(cose.COSEKEYS.crv) !== cose.COSECRV.P256)
      throw new HttpError(400, "unsupported_curve");
    jwk = {
      kty: "EC",
      crv: "P-256",
      x: b64url(key.get(cose.COSEKEYS.x)),
      y: b64url(key.get(cose.COSEKEYS.y)),
    };
  } else if (kty === cose.COSEKTY.OKP) {
    if (key.get(cose.COSEKEYS.crv) !== cose.COSECRV.ED25519)
      throw new HttpError(400, "unsupported_curve");
    jwk = { kty: "OKP", crv: "Ed25519", x: b64url(key.get(cose.COSEKEYS.x)) };
  } else if (kty === cose.COSEKTY.RSA) {
    jwk = {
      kty: "RSA",
      n: b64url(key.get(cose.COSEKEYS.n)),
      e: b64url(key.get(cose.COSEKEYS.e)),
    };
  } else {
    throw new HttpError(400, "unsupported_key_type");
  }
  const spki = createPublicKey({ key: jwk, format: "jwk" }).export({
    type: "spki",
    format: "der",
  });
  return { spki: new Uint8Array(spki), alg };
}

// Validate an answer document against its own rules and the request it names.
export function checkAnswer(answer, push, now = Date.now()) {
  const fail = (code) => {
    throw new HttpError(400, "answer_invalid", code);
  };
  if (!answer || typeof answer !== "object" || Array.isArray(answer))
    fail("not_object");
  const allowed = [
    "v",
    "type",
    "answer_id",
    "board_seq",
    "request_digest",
    "decision",
    "comment",
    "owner",
    "created_at",
    "expires_at",
    "rp_id",
  ];
  if (Object.keys(answer).some((key) => !allowed.includes(key)))
    fail("unknown_field");
  if (answer.v !== 1 || answer.type !== ANSWER_TYPE) fail("version_or_type");
  if (!/^[0-9a-f-]{36}$/.test(answer.answer_id || "")) fail("answer_id");
  if (!Number.isInteger(answer.board_seq) || answer.board_seq < 1)
    fail("board_seq");
  if (!/^[0-9a-f]{64}$/.test(answer.request_digest || ""))
    fail("request_digest");
  if (!DECISIONS.includes(answer.decision)) fail("decision");
  if (typeof answer.comment !== "string" || answer.comment.length > 4000)
    fail("comment");
  if (answer.decision === "ask" && !answer.comment.trim())
    fail("ask_needs_comment");
  if (answer.owner !== OWNER_USER_ID) fail("owner");
  if (answer.rp_id !== RP_ID) fail("rp_id");
  const created = Date.parse(answer.created_at);
  const expires = Date.parse(answer.expires_at);
  if (!Number.isFinite(created) || Math.abs(created - now) > 10 * 60 * 1000)
    fail("created_at");
  if (
    !Number.isFinite(expires) ||
    expires <= now ||
    expires - created > 7 * 24 * 3600 * 1000
  )
    fail("expires_at");
  const request = push?.requests?.find(
    (item) => item.board_seq === answer.board_seq,
  );
  if (!request) throw new HttpError(409, "request_not_open");
  if (request.request_digest !== answer.request_digest)
    throw new HttpError(409, "request_changed");
}
