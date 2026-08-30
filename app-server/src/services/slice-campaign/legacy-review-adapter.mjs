import { requireRecord, requireText } from "./contract.mjs";

export function createLegacyReviewCompatibilityAdapter({ invoke }) {
  if (typeof invoke !== "function") throw new TypeError("legacy review compatibility adapter requires invoke");
  return Object.freeze({
    async review({ subject, profile, selectionPlan }) {
      requireRecord(subject, "legacy review subject");
      requireRecord(profile, "legacy review physical profile");
      requireRecord(selectionPlan, "legacy review selection plan");
      requireText(subject.commit, "legacy review subject commit");
      const result = await invoke(Object.freeze({ subject, profile, selectionPlan }));
      requireRecord(result, "legacy review result");
      if (!new Set(["passed", "failed", "blocked"]).has(result.status)) throw new TypeError("legacy review result status is unsupported");
      return Object.freeze({ compatibility: "legacy_review_v1", ...result });
    },
  });
}
