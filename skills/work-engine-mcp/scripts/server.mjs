#!/usr/bin/env node

import { execFile } from "node:child_process";
import { realpath } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import * as z from "zod/v4";

const execFileAsync = promisify(execFile);
const MAX_OUTPUT = 16 * 1024 * 1024;
const HERE = path.dirname(fileURLToPath(import.meta.url));
const WORK_ENGINE_ROOT = path.resolve(HERE, "../../..");
function serverArguments(argv) {
  const options = { repository: process.cwd(), reviewAuthority: null, claimRoot: null };
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (!value || !["--repository", "--review-authority-file", "--claim-root"].includes(flag)) {
      throw new Error(
        "usage: server.mjs [--repository PATH] [--review-authority-file PATH] [--claim-root PATH]",
      );
    }
    if (flag === "--repository") options.repository = value;
    else if (flag === "--review-authority-file") options.reviewAuthority = value;
    else options.claimRoot = value;
  }
  return options;
}

async function runJson(command, args, options = {}) {
  try {
    const { stdout } = await execFileAsync(command, args, {
      ...options,
      encoding: "utf8",
      maxBuffer: MAX_OUTPUT,
    });
    return JSON.parse(stdout);
  } catch (error) {
    const detail = error?.stderr?.trim() || error?.message || String(error);
    throw new Error(detail, { cause: error });
  }
}

function result(value) {
  return {
    content: [{ type: "text", text: JSON.stringify(value, null, 2) }],
    structuredContent: { result: value },
  };
}

const identitySchema = {
  run_id: z.string().min(1).describe("Exact supervisor run ID"),
  slice_number: z.number().int().positive().describe("Exact positive slice number"),
  attempt_id: z.string().min(1).describe("Exact attempt ID"),
  plan_version: z.string().min(1).describe("Exact plan version"),
};

const exactReferenceSchema = z.object({
  owner: z.string().min(1),
  reference: z.string().min(1),
  revision: z.string().min(1),
  integrity_sha256: z.string().regex(/^[0-9a-f]{64}$/),
  freshness_rule: z.string().min(1),
}).strict();

const reviewFindingSchema = z.object({
  finding_id: z.string().min(1),
  attributed_reviewer: z.string().min(1),
  reviewer_generation: z.number().int().positive(),
  severity: z.string().min(1),
  observation: z.string().min(1),
  evidence_references: z.array(exactReferenceSchema).min(1),
  status: z.enum([
    "open",
    "remediation_presented",
    "verified_resolved",
    "withdrawn",
    "unresolved",
  ]),
  remediation_references: z.array(exactReferenceSchema),
}).strict();

const reviewResultPayloadSchema = z.object({
  findings: z.array(reviewFindingSchema),
  unresolved_questions: z.array(z.string().min(1)),
  evidence_references: z.array(exactReferenceSchema),
  claim_references: z.array(exactReferenceSchema),
}).strict();

const reviewTransitionPayloadSchema = z.union([
  reviewResultPayloadSchema,
  z.object({
    reviewed_subject: z.array(exactReferenceSchema).min(1),
    evidence_references: z.array(exactReferenceSchema).min(1),
  }).strict(),
  z.object({
    reason: z.string().min(1),
    reconciliation_action: z.string().min(1),
  }).strict(),
  z.object({
    reason: z.string().min(1),
    pending_next_action: z.string().min(1),
  }).strict(),
  z.object({
    outcome: z.string().min(1),
    reason: z.string().min(1),
    protected_references: z.array(exactReferenceSchema).min(1),
  }).strict(),
]);

async function main() {
  const options = serverArguments(process.argv.slice(2));
  const repository = await realpath(path.resolve(options.repository));
  const resumeScript = path.join(
    WORK_ENGINE_ROOT,
    "skills/slice-supervisor/scripts/resume_active_slice.py",
  );
  const claimScript = path.join(
    WORK_ENGINE_ROOT,
    "skills/claim-evidence/scripts/claim_evidence.py",
  );
  const reviewStateScript = path.join(
    WORK_ENGINE_ROOT,
    "skills/independent-review-state/scripts/independent_review_state.py",
  );
  const reviewAuthority = options.reviewAuthority
    ? await realpath(path.resolve(options.reviewAuthority))
    : null;
  const claimRoot = options.claimRoot
    ? await realpath(path.resolve(options.claimRoot))
    : null;

  const server = new McpServer({ name: "work-engine", version: "0.1.0" });

  server.registerTool(
    "read_active_slice_state",
    {
      description:
        "Read the current or one exact retained revision of an explicitly identified " +
        "slice-supervisor attempt. Read-only: this does not resume a capability or publish state.",
      inputSchema: {
        identity: z.object(identitySchema).strict(),
        revision: z.string().min(1).optional().describe("Exact retained Git object revision"),
      },
      outputSchema: { result: z.unknown() },
    },
    async ({ identity, revision }) => {
      const args = [
        resumeScript,
        "--repository",
        repository,
        "--identity-json",
        JSON.stringify(identity),
      ];
      if (revision) args.push("--revision", revision);
      const state = await runJson("python3", args, { cwd: repository });
      return result({
        projection_owner: "work-engine-mcp",
        semantic_owner: "slice-supervisor",
        authorization_scope: "read_only",
        query: revision ? "exact_revision" : "current",
        state,
      });
    },
  );

  if (reviewAuthority) {
    const reviewArgs = () => [
      reviewStateScript,
      "--repository",
      repository,
      "--authority-file",
      reviewAuthority,
    ];
    const callReviewState = async (args) => result(await runJson(
      "python3", [...reviewArgs(), ...args], { cwd: repository },
    ));

    server.registerTool(
      "begin_independent_review_episode",
      {
        description:
          "Begin the one authority-bound independent-review episode exposed by this server. " +
          "This does not mutate supervisor, implementation, acceptance, claim, or repository state.",
        inputSchema: {
          transition_id: z.string().min(1),
          evidence_references: z.array(exactReferenceSchema),
          claim_references: z.array(exactReferenceSchema),
          unresolved_questions: z.array(z.string().min(1)),
        },
        outputSchema: { result: z.unknown() },
      },
      async (request) => callReviewState([
        "begin", "--request-json", JSON.stringify(request),
      ]),
    );

    server.registerTool(
      "read_independent_review_episode",
      {
        description:
          "Read current or exact retained state for this server's single authority-bound " +
          "review episode. Reading never resumes or replays a review.",
        inputSchema: { revision: z.string().min(1).optional() },
        outputSchema: { result: z.unknown() },
      },
      async ({ revision }) => callReviewState([
        "read", ...(revision ? ["--revision", revision] : []),
      ]),
    );

    server.registerTool(
      "list_independent_review_episode_history",
      {
        description:
          "List bounded newest-first retained revisions for this server's single " +
          "authority-bound review episode.",
        inputSchema: {
          cursor: z.string().min(1).optional(),
          limit: z.number().int().min(1).max(100).default(20),
        },
        outputSchema: { result: z.unknown() },
      },
      async ({ cursor, limit }) => callReviewState([
        "history", "--limit", String(limit), ...(cursor ? ["--cursor", cursor] : []),
      ]),
    );

    server.registerTool(
      "advance_independent_review_episode",
      {
        description:
          "Apply one profile-specific CAS transition to this server's authority-bound review " +
          "episode. The tool cannot accept arbitrary durable keys or payloads.",
        inputSchema: {
          expected_revision: z.string().min(1),
          transition_id: z.string().min(1),
          action: z.enum([
            "record_initial_result",
            "record_remediation_subject",
            "record_re_evaluation",
            "mark_uncertain",
            "replace_writer",
            "retire_episode",
          ]),
          payload: reviewTransitionPayloadSchema.describe(
            "Action-specific payload. record_initial_result and record_re_evaluation use " +
            "the findings payload; every finding requires its own exact evidence_references.",
          ),
        },
        outputSchema: { result: z.unknown() },
      },
      async ({ expected_revision, transition_id, action, payload }) => callReviewState([
        "transition",
        "--expected-revision", expected_revision,
        "--transition-id", transition_id,
        "--action", action,
        "--payload-json", JSON.stringify(payload),
      ]),
    );
  }

  server.registerTool(
    "list_active_slice_history",
    {
      description:
        "List bounded newest-first retained state revisions for one exact slice-supervisor " +
        "attempt. Pass next_cursor back as cursor for the next older page. Read-only.",
      inputSchema: {
        identity: z.object(identitySchema).strict(),
        cursor: z.string().min(1).optional(),
        limit: z.number().int().min(1).max(100).default(20),
      },
      outputSchema: { result: z.unknown() },
    },
    async ({ identity, cursor, limit }) => {
      const args = [
        resumeScript,
        "--repository",
        repository,
        "--identity-json",
        JSON.stringify(identity),
        "--history-limit",
        String(limit),
      ];
      if (cursor) args.push("--history-cursor", cursor);
      const history = await runJson("python3", args, { cwd: repository });
      return result({
        projection_owner: "work-engine-mcp",
        semantic_owner: "slice-supervisor",
        authorization_scope: "read_only",
        ordering: "newest_first",
        ...history,
      });
    },
  );

  if (claimRoot) {
    const callClaimEvidence = async (args) => result(await runJson(
      "python3", [claimScript, "--root", claimRoot, ...args], { cwd: repository },
    ));
    const discoveryCriteriaSchema = z.object({
      namespace: z.string().min(1).optional(),
      subject_kind: z.string().min(1).optional(),
      stable_subject_id: z.string().min(1).optional(),
      profile: z.string().min(1).optional(),
      producer: z.string().min(1).optional(),
      support_qualification: z.string().min(1).optional(),
      sensitivity_reference: z.string().min(1).optional(),
      evidence_baseline: z.string().min(1).optional(),
      content_reference: z.string().min(1).optional(),
      consumer: z.string().min(1).optional(),
    }).strict().refine((criteria) => Object.keys(criteria).length > 0, {
      message: "discovery criteria must not be empty",
    });

    server.registerTool(
      "discover_claim_evidence",
      {
        description:
          "Discover bounded production claim candidates. Results do not assess applicability.",
        inputSchema: { criteria: discoveryCriteriaSchema },
        outputSchema: { result: z.unknown() },
      },
      async ({ criteria }) => callClaimEvidence([
        "discover", "--criteria-json", JSON.stringify(criteria),
      ]),
    );

    server.registerTool(
      "resolve_claim_evidence",
      {
        description: "Resolve one exact production claim or immutable revision identity.",
        inputSchema: { identity: z.string().min(1) },
        outputSchema: { result: z.unknown() },
      },
      async ({ identity }) => callClaimEvidence(["resolve", "--identity", identity]),
    );

    server.registerTool(
      "traverse_claim_evidence",
      {
        description: "Traverse typed predecessors or successors from one exact claim revision.",
        inputSchema: {
          revision: z.string().min(1),
          direction: z.enum(["predecessors", "successors", "both"]).default("both"),
        },
        outputSchema: { result: z.unknown() },
      },
      async ({ revision, direction }) => callClaimEvidence([
        "traverse", "--revision", revision, "--direction", direction,
      ]),
    );

    server.registerTool(
      "query_claim_evidence_reliance",
      {
        description:
          "Inspect direct reliance on one exact claim revision or reverse reliance by consumer.",
        inputSchema: {
          revision: z.string().min(1).optional(),
          consumer: z.string().min(1).optional(),
        },
        outputSchema: { result: z.unknown() },
      },
      async ({ revision, consumer }) => {
        if (Boolean(revision) === Boolean(consumer)) {
          throw new Error("supply exactly one reliance query key");
        }
        return callClaimEvidence([
          "reliance", revision ? "--revision" : "--consumer", revision ?? consumer,
        ]);
      },
    );
  }

  await server.connect(new StdioServerTransport());
}

main().catch((error) => {
  console.error(`work-engine-mcp: ${error.message}`);
  process.exit(1);
});
