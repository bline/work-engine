import assert from "node:assert/strict";
import test from "node:test";

import { validateDocument } from "../scripts/validate-idea-status.mjs";

const BASE_STATUS = `architectural_supersession: none
architectural_supersession_note: "Checked against plausible canonical owners; no match found."
residue: none
backlog: none
audit_scope:
  - keyword-scan: full_document
audit_scope_completeness: complete
status_as_of: 2026-09-17`;

const BASE_PROVENANCE = `origin: direct_capture`;

function fixture({ status = BASE_STATUS, provenance = BASE_PROVENANCE, body = "" } = {}) {
  return [
    "# Fixture idea document",
    "",
    "```yaml",
    "idea_status:",
    status.replace(/^/gm, "  "),
    "```",
    "",
    "```yaml",
    "idea_provenance:",
    provenance.replace(/^/gm, "  "),
    "```",
    "",
    body,
  ].join("\n");
}

test("residue: present backed only by a YAML-block description of a tag fails validation", () => {
  // The exact case the grammar's own evidence requirement exists to catch: a status
  // block that describes a KIND/disposition pair in its own prose (residue_ledger)
  // without that tag ever actually appearing in the document body it is a status
  // block *about*. A document must not be able to validate its own claim by
  // restating it inside the very field that claim is supposed to be evidence for.
  const doc = fixture({
    status: BASE_STATUS.replace("residue: none", 'residue: present\nresidue_ledger: "KIND: RESIDUE, OPEN"'),
    body: "This document has ordinary prose and no inline KIND tag anywhere.",
  });
  const result = validateDocument("fixture.md", doc);
  assert.ok(
    result.errors.some((error) => error.includes("residue: present has no [KIND: RESIDUE] ... OPEN tag")),
    `expected a residue-support error, got: ${JSON.stringify(result.errors)}`,
  );
});

test("residue: present backed by a real inline KIND tag in the body passes validation", () => {
  const doc = fixture({
    status: BASE_STATUS.replace("residue: none", 'residue: present\nresidue_ledger: "See inline tag."'),
    body: "Some open question. [KIND: RESIDUE] [OPEN -- no owner found]",
  });
  const result = validateDocument("fixture.md", doc);
  assert.deepEqual(result.errors, []);
});

test("residue: present backed by a real prose-form KIND tag in the body passes validation", () => {
  // The prose form (`KIND: RESIDUE, OPEN`) is legitimate evidence when it appears in
  // the document's own body -- only the YAML-block form of the same text is circular.
  const doc = fixture({
    status: BASE_STATUS.replace("residue: none", 'residue: present\nresidue_ledger: "See inline note."'),
    body: "Some open question -- KIND: RESIDUE, OPEN, not independently re-verified item-by-item this pass.",
  });
  const result = validateDocument("fixture.md", doc);
  assert.deepEqual(result.errors, []);
});

test("backlog: present backed only by a YAML-block description of a PLAN tag fails validation", () => {
  const doc = fixture({
    status: BASE_STATUS.replace("backlog: none", 'backlog: present\nbacklog_ledger: "[PLAN: OPEN]"'),
    body: "This document has ordinary prose and no inline PLAN tag anywhere.",
  });
  const result = validateDocument("fixture.md", doc);
  assert.ok(
    result.errors.some((error) => error.includes("backlog: present has no [KIND: BACKLOG] ... OPEN tag or [PLAN: OPEN/PARTIAL] tag")),
    `expected a backlog-support error, got: ${JSON.stringify(result.errors)}`,
  );
});

test("architectural_supersession requires a non-empty note regardless of value", () => {
  const doc = fixture({ status: BASE_STATUS.replace(/^architectural_supersession_note:.*$/m, "").trim() });
  const result = validateDocument("fixture.md", doc);
  assert.ok(
    result.errors.some((error) => error.includes("requires a non-empty architectural_supersession_note")),
    `expected a missing-note error, got: ${JSON.stringify(result.errors)}`,
  );
});

test("architectural_supersession: unknown does not require audit_scope_completeness: partial", () => {
  // Regression for the removed rule: completeness tracks declared-scope ledger/plan
  // coverage (SS5), a different question from whether canonical views were read for
  // semantic coverage (SS3.1). A fully audit_scope_completeness: complete document can
  // still have architectural_supersession: unknown if that specific check was never done.
  const doc = fixture({
    status: BASE_STATUS
      .replace("architectural_supersession: none", "architectural_supersession: unknown")
      .replace('architectural_supersession_note: "Checked against plausible canonical owners; no match found."', 'architectural_supersession_note: "Never checked against any plausible canonical owner."'),
  });
  const result = validateDocument("fixture.md", doc);
  assert.deepEqual(result.errors, []);
});

test("architectural_supersession: full requires scope on every superseded_by entry", () => {
  // scope was previously required only for partial; full's claim to cover the whole
  // document makes the scope no less necessary to state than partial's narrower one.
  const status = `architectural_supersession: full
architectural_supersession_note: "Every claim checked and covered."
superseded_by:
  - view: app-server/docs/architecture/some-view.md
residue: none
backlog: none
audit_scope:
  - keyword-scan: full_document
audit_scope_completeness: complete
status_as_of: 2026-09-17`;
  const result = validateDocument("fixture.md", fixture({ status }));
  assert.ok(
    result.errors.some((error) => error.includes("superseded_by[0] is missing scope, required for architectural_supersession: full")),
    `expected a missing-scope error for full, got: ${JSON.stringify(result.errors)}`,
  );
});

test("architectural_supersession: full passes when superseded_by carries scope", () => {
  const status = `architectural_supersession: full
architectural_supersession_note: "Every claim checked and covered."
superseded_by:
  - view: app-server/docs/architecture/some-view.md
    scope: "The whole document, section by section."
residue: none
backlog: none
audit_scope:
  - keyword-scan: full_document
audit_scope_completeness: complete
status_as_of: 2026-09-17`;
  const result = validateDocument("fixture.md", fixture({ status }));
  assert.deepEqual(result.errors, []);
});

test("retirement-candidate query fires for not_applicable + residue: none + backlog: none, same as full", () => {
  const notApplicable = fixture({ status: BASE_STATUS.replace("architectural_supersession: none", "architectural_supersession: not_applicable") });
  const notApplicableResult = validateDocument("fixture.md", notApplicable);
  assert.deepEqual(notApplicableResult.errors, []);
  assert.ok(
    notApplicableResult.infos.some((info) => info.includes("retirement-candidate: architectural_supersession: not_applicable")),
    `expected a retirement-candidate info for not_applicable, got: ${JSON.stringify(notApplicableResult.infos)}`,
  );

  const full = fixture({
    status: `architectural_supersession: full
architectural_supersession_note: "Every claim checked and covered."
superseded_by:
  - view: app-server/docs/architecture/some-view.md
    scope: "The whole document, section by section."
residue: none
backlog: none
audit_scope:
  - keyword-scan: full_document
audit_scope_completeness: complete
status_as_of: 2026-09-17`,
  });
  const fullResult = validateDocument("fixture.md", full);
  assert.deepEqual(fullResult.errors, []);
  assert.ok(
    fullResult.infos.some((info) => info.includes("retirement-candidate: architectural_supersession: full")),
    `expected a retirement-candidate info for full, got: ${JSON.stringify(fullResult.infos)}`,
  );
});

test("a clean minimal fixture has zero errors", () => {
  const result = validateDocument("fixture.md", fixture());
  assert.deepEqual(result.errors, []);
  assert.equal(result.unaudited, false);
});
