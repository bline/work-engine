// Open owner requests from the coordination board.
//   PUT  (laptop) replaces the open set with a new snapshot.
//   GET  (owner)  returns the latest snapshot.
import {
  HttpError,
  json,
  latestPush,
  storePush,
  readBody,
  requireLaptop,
  requireOwner,
  route,
  sha256Hex,
} from "./_lib.js";
import { validProjectionEnvelope } from "../src/task-projection.js";

export const PUT = route(async (request) => {
  requireLaptop(request);
  const body = await readBody(request, 2 * 1024 * 1024);
  if (!Array.isArray(body.requests) || body.requests.length > 200) {
    throw new HttpError(400, "requests_invalid");
  }
  const requests = body.requests.map((item) => {
    const { board_seq, request_json, request_digest } = item || {};
    if (
      !Number.isInteger(board_seq) ||
      typeof request_json !== "string" ||
      typeof request_digest !== "string"
    ) {
      throw new HttpError(400, "request_item_invalid");
    }
    if (sha256Hex(request_json) !== request_digest)
      throw new HttpError(400, "request_digest_mismatch", board_seq);
    return { board_seq, request_json, request_digest };
  });
  const pushedAt = new Date().toISOString();
  const taskProjection = body.task_projection ?? null;
  if (taskProjection !== null && !validProjectionEnvelope(taskProjection)) {
    throw new HttpError(400, "task_projection_invalid");
  }
  const stored = await storePush({
    pushed_at: pushedAt,
    owner_addresses: Array.isArray(body.owner_addresses)
      ? body.owner_addresses.slice(0, 10)
      : [],
    board_revision:
      typeof body.board_revision === "string" ? body.board_revision : null,
    requests,
    task_projection: taskProjection,
  });
  return json(200, { stored, count: requests.length });
});

export const GET = route(async (request) => {
  await requireOwner(request);
  const push = await latestPush();
  return json(200, push || { pushed_at: null, requests: [] });
});
