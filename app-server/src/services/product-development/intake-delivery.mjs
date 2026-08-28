import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";

const run = promisify(execFile);
const SHA256 = /^sha256:[0-9a-f]{64}$/;
const OID = /^[0-9a-f]{40,64}$/;
const INTAKE_FILE_NAMES = [
  "record.json", "assessment.md", "authority.md", "repository-observations.md",
  "source-checkpoint.json",
];
const INTAKE_FILES = new Set(INTAKE_FILE_NAMES);

function sha256(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(`${label} is required`);
  return value;
}

async function invoke(command, args, options = {}) {
  const { stdout } = await run(command, args, {
    encoding: "utf8", timeout: 30_000, maxBuffer: 2 * 1024 * 1024, ...options,
  });
  return stdout;
}

export function createIntakeDelivery({ repositoryRoot, snapshotRoot, artifacts, invokeProcess = invoke }) {
  const repository = path.resolve(repositoryRoot);
  const validator = path.join(path.resolve(snapshotRoot), "skills/idea-intake/scripts/idea_intake.py");

  const readSource = async (request) => {
    const revision = text(request?.repository_revision, "source repository revision");
    const repositoryPath = text(request?.repository_path, "source repository path");
    const expected = text(request?.sha256, "source sha256");
    const startLine = request?.start_line;
    const endLine = request?.end_line;
    if (!OID.test(revision) || path.isAbsolute(repositoryPath)
        || repositoryPath.split("/").some((segment) => segment === ".." || segment === "")) {
      throw new TypeError("source binding is invalid");
    }
    if (!SHA256.test(expected)) throw new TypeError("source sha256 is invalid");
    if (!Number.isInteger(startLine) || !Number.isInteger(endLine)
        || startLine < 1 || endLine < startLine) {
      throw new TypeError("source line range is invalid");
    }
    const source = await invokeProcess("git", ["-C", repository, "show", `${revision}:${repositoryPath}`]);
    const lines = source.split(/(?<=\n)/);
    if (endLine > lines.length) throw new TypeError("source line range exceeds the exact source");
    const content = lines.slice(startLine - 1, endLine).join("");
    if (sha256(content) !== expected) throw new Error("source bytes do not match the requested digest");
    return { schema_version: 1, source: { repository_revision: revision, repository_path: repositoryPath, range: { start_line: startLine, end_line: endLine }, sha256: expected }, content };
  };

  const publish = async (request) => {
    const ideaId = text(request?.idea_id, "idea id");
    if (!Array.isArray(request?.files)
        || request.files.some((file) => !INTAKE_FILES.has(file?.path))) {
      throw new TypeError("intake publication contains an unsupported artifact reference");
    }
    const recordFile = request.files.find((file) => file.path === "record.json");
    if (!recordFile) throw new TypeError("intake publication requires record.json");
    const record = JSON.parse(recordFile.content);
    if (record?.idea_id !== ideaId) throw new TypeError("intake request identity does not match record.json");
    const result = await artifacts.publishCreateOnly({
      operationId: text(request?.operation_id, "operation id"),
      artifactKind: "idea-intake",
      artifactId: ideaId,
      destination: ["ideas", "intake", ideaId],
      files: request?.files,
      prepare: async (stage) => {
        const output = JSON.parse(await invokeProcess("python3", [
          validator, "project", path.join(stage, "record.json"), "--repository", repository,
        ]));
        if (output?.status !== "valid" || !output.projection) {
          throw new Error(output?.error ?? "idea intake validator refused publication");
        }
        const projectionText = `${JSON.stringify(output.projection, null, 2)}\n`;
        const { writeFile } = await import("node:fs/promises");
        await writeFile(path.join(stage, "projection.json"), projectionText, { encoding: "utf8", flag: "wx" });
        return {
          source_binding: output.projection.source,
          validator: { name: "idea-intake", script_sha256: sha256(await (await import("node:fs/promises")).readFile(validator)), outcome: "valid", projection_sha256: sha256(JSON.stringify(output.projection)) },
          projection: output.projection,
        };
      },
    });
    return result;
  };

  return { readSource, publish };
}

export const intakeCapabilityDefinitions = Object.freeze({
  "product-development.intake.read-source": Object.freeze({
    namespace: "development", name: "read_source",
    description: "Read one exact Git-bound raw idea after the host verifies its revision, path, and digest.",
    inputSchema: { type: "object", required: ["repository_revision", "repository_path", "start_line", "end_line", "sha256"], properties: { repository_revision: { type: "string" }, repository_path: { type: "string" }, start_line: { type: "integer", minimum: 1 }, end_line: { type: "integer", minimum: 1 }, sha256: { type: "string" } }, additionalProperties: false },
  }),
  "product-development.intake.publish": Object.freeze({
    namespace: "development", name: "publish_intake",
    description: "Request create-only publication of a complete intake bundle. The host derives its destination, validates the canonical record, and returns a mechanical receipt.",
    inputSchema: {
      type: "object",
      required: ["operation_id", "idea_id", "files"],
      properties: {
        operation_id: { type: "string" },
        idea_id: { type: "string" },
        files: {
          type: "array",
          minItems: 1,
          maxItems: INTAKE_FILE_NAMES.length,
          items: {
            type: "object",
            required: ["path", "content"],
            properties: {
              path: { type: "string", enum: INTAKE_FILE_NAMES },
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
