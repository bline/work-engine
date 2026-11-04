import { Redis } from "@upstash/redis";

export const MAILBOX_PREFIX = "devwits:mailbox:";

// The factory also lets integration checks use a separate namespace in the
// real database. Production credentials are read only on the first operation.
export function createMailboxStore(getClient, prefix = MAILBOX_PREFIX) {
  const readHash = async (name) =>
    Object.entries((await getClient().hgetall(prefix + name)) || {})
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([, value]) => value);

  return {
    latestPush: () => getClient().get(prefix + "requests"),
    storePush: async (snapshot) => {
      await getClient().set(prefix + "requests", snapshot);
      return prefix + "requests";
    },
    initializePush: async (snapshot) =>
      (await getClient().set(prefix + "requests", snapshot, { nx: true })) ===
      "OK",
    approvalKeys: () => readHash("approval-keys"),
    storeApprovalKey: async (key) =>
      (await getClient().hsetnx(
        prefix + "approval-keys",
        key.credential_id,
        key,
      )) === 1,
    deleteApprovalKey: async (id) =>
      (await getClient().hdel(prefix + "approval-keys", id)) > 0,
    answers: () => readHash("answers"),
    storeAnswer: async (id, answer) =>
      (await getClient().hsetnx(prefix + "answers", id, answer)) === 1,
    deleteAnswer: async (id) => {
      await getClient().hdel(prefix + "answers", id);
    },
    storeChallenge: async (id, challenge, ttlSeconds = 300) => {
      const result = await getClient().set(
        prefix + "reg-challenge:" + id,
        challenge,
        { ex: ttlSeconds, nx: true },
      );
      if (result !== "OK")
        throw new Error("Registration challenge already exists");
    },
    consumeChallenge: (id) =>
      getClient().getdel(prefix + "reg-challenge:" + id),
  };
}

let client;
function getClient() {
  if (!client) {
    const url =
      process.env.UPSTASH_REDIS_REST_URL || process.env.KV_REST_API_URL;
    const token =
      process.env.UPSTASH_REDIS_REST_TOKEN || process.env.KV_REST_API_TOKEN;
    if (!url || !token)
      throw new Error("Redis REST credentials are not configured");
    client = new Redis({ url, token });
  }
  return client;
}

export const {
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
} = createMailboxStore(getClient);
