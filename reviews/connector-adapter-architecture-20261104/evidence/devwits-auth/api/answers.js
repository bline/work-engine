// Signed owner answers.
//   POST   (owner)  stores one answer after checking it and its passkey signature.
//   GET    (laptop) returns all stored answers.
//   DELETE (laptop) removes one answer once the laptop has put it on the board.
import { verifyAuthenticationResponse } from "@simplewebauthn/server";
import {
  HttpError,
  ORIGIN,
  RP_ID,
  approvalKeys,
  checkAnswer,
  fromB64url,
  json,
  latestPush,
  answers,
  storeAnswer,
  deleteAnswer,
  readBody,
  requireLaptop,
  requireOwner,
  route,
  sha256B64url,
} from "./_lib.js";

export const POST = route(async (request) => {
  await requireOwner(request);
  const body = await readBody(request);
  const { answer_json: answerJson, credential } = body;
  if (typeof answerJson !== "string" || answerJson.length > 16384)
    throw new HttpError(400, "answer_json_invalid");
  let answer;
  try {
    answer = JSON.parse(answerJson);
  } catch {
    throw new HttpError(400, "answer_json_invalid");
  }
  checkAnswer(answer, await latestPush());

  const keys = await approvalKeys();
  const key = keys.find((item) => item.credential_id === credential?.id);
  if (!key) throw new HttpError(403, "approval_key_unknown");
  let verification;
  try {
    verification = await verifyAuthenticationResponse({
      response: credential,
      expectedChallenge: sha256B64url(answerJson),
      expectedOrigin: ORIGIN,
      expectedRPID: RP_ID,
      requireUserVerification: true,
      credential: {
        id: key.credential_id,
        publicKey: fromB64url(key.cose_b64url),
        counter: 0,
      },
    });
  } catch (error) {
    throw new HttpError(
      400,
      "signature_invalid",
      String(error.message || error).slice(0, 200),
    );
  }
  if (!verification.verified) throw new HttpError(400, "signature_invalid");

  const stored = await storeAnswer(answer.answer_id, {
    stored_at: new Date().toISOString(),
    answer_json: answerJson,
    credential,
  });
  if (!stored) {
    throw new HttpError(409, "answer_exists");
  }
  return json(200, { stored: answer.answer_id });
});

export const GET = route(async (request) => {
  requireLaptop(request);
  return json(200, { answers: await answers() });
});

export const DELETE = route(async (request) => {
  requireLaptop(request);
  const id = new URL(request.url).searchParams.get("answer_id") || "";
  if (!/^[0-9a-f-]{36}$/.test(id))
    throw new HttpError(400, "answer_id_invalid");
  await deleteAnswer(id);
  return json(200, { deleted: id });
});
