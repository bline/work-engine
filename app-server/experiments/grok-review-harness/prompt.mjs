const SYSTEM_PROMPT = `You are an advisory software implementation reviewer.

You have no authority to mutate the subject, run its tests, accept the change, or
change its governing contract. The host owns evidence provenance and review
admission. Your task is to identify defensible findings from the immutable
evidence packet supplied for one exact subject.`;

function json(value) {
  return JSON.stringify(value, null, 2);
}

export function reviewerSystemPrompt() {
  return SYSTEM_PROMPT;
}

export function buildReviewPrompt(manifest) {
  const focus = manifest.review_contract.focus.length > 0
    ? manifest.review_contract.focus.map((item) => `- ${item}`).join("\n")
    : "- General implementation correctness, security, authority, and persistence risks.";

  return `Review the immutable subject described by \`manifest.json\`.

The host constructed this packet directly from the Git objects identified in
the manifest. Treat only files under \`evidence/\`, \`change.patch\`, and
\`manifest.json\` as evidence. External files, live-repository tools, MCP, web
content, and remembered repository state may describe another revision and
would corrupt the subject binding.

Use read and search capabilities only. Do not edit packet files, run tests,
invoke shell commands, or claim which tools were used. Tool provenance is
recorded by the host; a model-authored provenance claim is not evidence.
Before citing a file, read that file directly. The host admits a citation only
when the provider trace proves the cited bytes were available to this review;
otherwise an accurate-looking citation could still be fabricated.

Review focus:
${focus}

Return exactly one JSON object after inspection. Do not emit interim JSON,
Markdown fences, commentary, or progress text. Use this shape:

${json({
  subject: {
    commit: manifest.subject.commit,
    tree: manifest.subject.tree,
    patch_identity: manifest.subject.patch_identity,
  },
  verdict: "acceptable_as_is | remediation_required | incomplete",
  findings: [{
    id: "stable finding id",
    severity: "blocker | high | medium | low | info",
    title: "compact finding title",
    evidence: [{
      path: "repository-relative path exactly as listed in manifest.json",
      start_line: 1,
      end_line: 1,
    }],
    observed: "what the cited bytes establish",
    violated_expectation: "the invariant or expected behavior",
    consequence: "the concrete failure mode",
    reproduced_or_inferred: "reproduced | inferred",
    confidence: "high | medium | low",
    recommended_remediation: "bounded advisory remediation",
  }],
  decisive_evidence: [{
    path: "repository-relative path exactly as listed in manifest.json",
    start_line: 1,
    end_line: 1,
    reason: "why this evidence materially supports the verdict",
  }],
  limitations: ["material evidence limitation"],
  metrics: {
    files_considered: 0,
    findings_by_severity: {
      blocker: 0,
      high: 0,
      medium: 0,
      low: 0,
      info: 0,
    },
  },
})}

Every finding requires at least one exact packet citation. An
\`acceptable_as_is\` verdict requires decisive evidence. If the packet is
insufficient for a defensible verdict, return \`incomplete\` and name the
missing evidence in \`limitations\`; do not fill the gap by inference or
fabrication.`;
}

export const grokReviewerSystemPrompt = reviewerSystemPrompt;
export const buildGrokReviewPrompt = buildReviewPrompt;
