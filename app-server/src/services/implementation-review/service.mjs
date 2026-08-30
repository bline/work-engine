import { digest, freeze, validateImplementationReviewResult } from "./contract.mjs";

export function createImplementationReviewService() {
  return Object.freeze({
    admit({ result, expectedSubject }) {
      validateImplementationReviewResult(result);
      validateImplementationReviewResult({ ...result, subject: expectedSubject });
      if (digest(result.subject) !== digest(expectedSubject)) throw new TypeError("implementation review result subject does not match the expected immutable subject");
      const admitted = structuredClone(result);
      return freeze({
        schemaVersion: 1,
        result: admitted,
        resultRevision: digest(admitted),
        authority: freeze({
          mutationAuthorized: false,
          implementationAcceptanceAuthorized: false,
          architectureChoiceAuthorized: false,
          reviewerSelectionAuthorized: false,
          humanAuthorityConferred: false,
          independenceClaimed: false,
        }),
      });
    },
  });
}
