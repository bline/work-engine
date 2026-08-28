import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";
import { promisify } from "node:util";
import path from "node:path";

const run = promisify(execFile);
const SHA256 = /^sha256:[0-9a-f]{64}$/;
const PROPOSAL_FILE_NAMES = [
  "packet.json", "proposal.md", "placement.md", "relationships.md",
  "evidence.md", "formation-evidence.md", "implementation-plan.md",
];
const PROPOSAL_FILES = new Set(PROPOSAL_FILE_NAMES);

function sha256(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(`${label} is required`);
  return value;
}

async function invoke(command, args, options = {}) {
  const { stdout } = await run(command, args, { encoding: "utf8", timeout: 30_000, maxBuffer: 2 * 1024 * 1024, ...options });
  return stdout;
}

export function createProposalDelivery({ repositoryRoot, snapshotRoot, artifacts, invokeProcess = invoke }) {
  const repository = path.resolve(repositoryRoot);
  const intakeValidator = path.join(path.resolve(snapshotRoot), "skills/idea-intake/scripts/idea_intake.py");
  const packetValidator = path.join(path.resolve(snapshotRoot), "skills/proposal-packets/scripts/proposal_packets.py");

  const readIntake = async (request) => {
    const ideaId = text(request?.idea_id, "idea id");
    const record = await artifacts.resolveExistingFile({
      destination: ["ideas", "intake", ideaId],
      file: "record.json",
    });
    const expectedRecordSha256 = text(request?.record_sha256, "intake record sha256");
    if (!SHA256.test(expectedRecordSha256)
        || sha256(await readFile(record)) !== expectedRecordSha256) {
      throw new Error("canonical intake record does not match the requested digest");
    }
    const validatorRecord = await artifacts.resolveExistingFile({
      destination: ["ideas", "intake", ideaId],
      file: "record.json",
    });
    if (validatorRecord !== record) throw new Error("canonical intake record changed before validation");
    const output = JSON.parse(await invokeProcess("python3", [intakeValidator, "project", validatorRecord, "--repository", repository]));
    if (output?.status !== "valid" || !output.projection) throw new Error(output?.error ?? "intake projection is invalid");
    return { schema_version: 1, projection: output.projection, projection_sha256: sha256(JSON.stringify(output.projection)) };
  };

  const publish = async (request) => {
    const familyId = text(request?.family_id, "proposal family id");
    const proposalId = text(request?.proposal_id, "proposal id");
    const intake = await readIntake({
      idea_id: request?.idea_id,
      record_sha256: request?.intake_record_sha256,
    });
    if (!Array.isArray(request?.files)
        || request.files.some((file) => !PROPOSAL_FILES.has(file?.path))) {
      throw new TypeError("proposal publication contains an unsupported artifact reference");
    }
    const packetFile = request.files.find((file) => file.path === "packet.json");
    if (!packetFile) throw new TypeError("proposal publication requires packet.json");
    const packet = JSON.parse(packetFile.content);
    if (packet?.proposal_id !== proposalId || packet?.family_id !== familyId) {
      throw new TypeError("proposal request identity does not match packet.json");
    }
    return artifacts.publishCreateOnly({
      operationId: text(request?.operation_id, "operation id"), artifactKind: "proposal-packet",
      artifactId: proposalId, destination: ["proposals", familyId, proposalId], files: request?.files,
      prepare: async (stage) => {
        const output = JSON.parse(await invokeProcess("python3", [packetValidator, "validate", stage]));
        if (output?.status !== "valid" || output.packet_count !== 1
            || !Array.isArray(output.proposal_ids)
            || output.proposal_ids.length !== 1
            || output.proposal_ids[0] !== proposalId) {
          throw new Error(output?.error ?? "proposal packet validator refused publication");
        }
        return {
          source_binding: {
            idea_id: request.idea_id,
            record_sha256: request.intake_record_sha256,
            projection_sha256: intake.projection_sha256,
          },
          validator: { name: "proposal-packets", script_sha256: sha256(await readFile(packetValidator)), outcome: "valid", proposal_ids: output.proposal_ids },
        };
      },
    });
  };
  return { readIntake, publish };
}

export const proposalCapabilityDefinitions = Object.freeze({
  "product-development.proposal.read-intake": Object.freeze({
    namespace: "development", name: "read_intake",
    description: "Load a host-validated proposal-formation projection for one logical idea identifier.",
    inputSchema: { type: "object", required: ["idea_id", "record_sha256"], properties: { idea_id: { type: "string" }, record_sha256: { type: "string" } }, additionalProperties: false },
  }),
  "product-development.proposal.publish": Object.freeze({
    namespace: "development", name: "publish_packets",
    description: "Request create-only publication of one complete proposal packet. The host derives its destination, validates the staged packet, and returns a non-authorizing receipt.",
    inputSchema: {
      type: "object",
      required: ["operation_id", "idea_id", "intake_record_sha256", "family_id", "proposal_id", "files"],
      properties: {
        operation_id: { type: "string" },
        idea_id: { type: "string" },
        intake_record_sha256: { type: "string" },
        family_id: { type: "string" },
        proposal_id: { type: "string" },
        files: {
          type: "array",
          minItems: 1,
          maxItems: PROPOSAL_FILE_NAMES.length,
          items: {
            type: "object",
            required: ["path", "content"],
            properties: {
              path: { type: "string", enum: PROPOSAL_FILE_NAMES },
              content: { type: "string" },
            },
            additionalProperties: false,
          },
        },
      },
      additionalProperties: false,
    },
  }),
});
