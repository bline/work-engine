import { createLegacyReviewSubjectBackend } from "./legacy-backend-adapter.mjs";
import { requireOperation, requireRecord } from "./contract.mjs";

export function createReviewSubjectService({ workspaceRoot, backend = null, ...backendOptions } = {}) {
  const selectedBackend = backend ?? createLegacyReviewSubjectBackend({ workspaceRoot, ...backendOptions });
  if (!selectedBackend || typeof selectedBackend.invoke !== "function") {
    throw new TypeError("review-subject service requires a mediated backend");
  }

  const invoke = async ({ operation, input }) => {
    requireOperation(operation);
    requireRecord(input, "review-subject operation input");
    return selectedBackend.invoke({ operation, input });
  };

  return Object.freeze({
    invoke,
    createCandidate(request) {
      return invoke({ operation: "create_candidate", input: { request } });
    },
    transitionCandidate({ candidate, kind, expectedAccepted = null }) {
      return invoke({
        operation: "transition_candidate",
        input: { candidate, kind, expected_accepted: expectedAccepted },
      });
    },
    validateCheckpoint({ receipt, kind, requirePaths = true }) {
      return invoke({
        operation: "validate_checkpoint",
        input: { receipt, kind, require_paths: requirePaths },
      });
    },
    createPhysicalProfile({ subject, repository = null }) {
      return invoke({ operation: "create_physical_profile", input: { subject, repository } });
    },
    validatePhysicalProfile({ profile }) {
      return invoke({ operation: "validate_physical_profile", input: { profile } });
    },
  });
}
