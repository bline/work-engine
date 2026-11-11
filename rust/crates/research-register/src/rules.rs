//! Validation rule identifiers and violations.
//!
//! Each [`RuleId`] is one entry of the R-1a rule catalogue (R-01 to R-18) or one approved
//! F-decision (F-2, F-3, F-5, F-6). Its doc comment states the rule and its source. A violation
//! means the register content is invalid. Being unable to read the content is a different outcome,
//! [`crate::Error`]; violations never carry reading failures (F-4).

use std::fmt;

/// A rule of the register schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuleId {
    /// R-01: the register files are present with regular-file kinds, and `studies/` is a tree of
    /// regular study files. Non-UTF-8 entry names are invalid content (A6). Source: Python
    /// `REGISTER_FILES`; adopted as a documented rule by user decision A4.
    R01,
    /// R-02: every file is UTF-8, every JSON document parses, and every non-empty JSON Lines line
    /// parses. Source: Python `bad-encoding`/`bad-json`; adopted by A4.
    R02,
    /// F-5: no JSON object has a duplicate key, at any depth. Source: user decision F-5.
    F5,
    /// F-6: no number is NaN, Infinity or outside the finite range. Source: user decision F-6.
    F6,
    /// R-03: every record and nested object uses its closed key set: all required keys present,
    /// no key outside the set. Source: README rule 7 (closed key sets); the sets are F-1.
    R03,
    /// R-04: no key naming global usefulness or obsolescence (`/useful|obsolet/i`) in any object
    /// a closed key set covers. Source: README rule 7.
    R04,
    /// R-05: values of relations, authorization statuses, off-repo statuses, `evidence_state`
    /// states, event kinds, and judgment axes and their values. Source: README Conventions and
    /// rule 5.
    R05,
    /// R-05b: values of study kinds, lifecycle states, stop classes and finding bases. Source:
    /// Python constants; approved by F-1.
    R05b,
    /// R-06: identifier formats (`Q<n>`; kebab-case study id equal to its file name; finding id
    /// `<study>#<slug>` under its own study; kebab-case event and applicability ids; decision id
    /// kebab-case with dots) and uniqueness per collection, aliases across studies. Source:
    /// formats F-1; uniqueness README (integrity).
    R06,
    /// R-07: text fields are non-empty text, lists are lists (non-empty where stated), records
    /// are objects, and a finding's `n` is an integer, text or null. Source: Python `bad-value`;
    /// adopted by A4.
    R07,
    /// R-08: dates have the form `YYYY-MM-DD` with ASCII digits. Source: Python `check_date`; F-1.
    R08,
    /// F-3: dates are valid Gregorian calendar dates, year 1 or later. Source: user decision F-3.
    F3,
    /// F-2: a study file is a JSON object, never `null`. Source: user decision F-2.
    F2,
    /// R-09: references resolve: questions from studies; findings from decisions, events,
    /// judgments, applicability and materialized rows; decisions from applicability and
    /// `historical.superseded_by`. Source: README (integrity, Conventions).
    R09,
    /// R-10: an in-repo pin is `{path, commit, blob, lines?}` with a 7-40 hex commit, a 40 hex
    /// blob and `lines` of the form `N` or `N-M`; the commit resolves, `commit:path` is a blob
    /// equal to `blob`, and `lines` lie inside it. Source: README rule 5; resolution domain F-4.
    R10,
    /// R-11: an off-repo source is `{location, sha256|null, custody: off_repo, status, note?}`
    /// with non-empty location, 64 hex or null sha256, and a sha256 when verified. Source: README
    /// rule 5.
    R11,
    /// R-12: a decision `revision` is `git:<7-40 hex>`; the commit and its document resolve;
    /// `lines` need a document and lie inside it. Source: README Conventions.
    R12,
    /// R-13: each `revisions` entry is `git:<7-40 hex>`, resolves like the decision's own pin, and
    /// differs from the current revision and every other entry. Source: README Conventions.
    R13,
    /// R-14: each `revisions` entry is an ancestor of the next and of the current revision.
    /// Source: README Conventions.
    R14,
    /// R-15: `historical.superseded_by` names a different existing decision. Source: README
    /// Conventions.
    R15,
    /// R-16: `findings: []` requires `evidence_state` (`none_linked`, or `none_found` with corpus
    /// `{id, sha256|pin}`, `search_scope`, `coverage`, `assessor`, `date`), and `evidence_state`
    /// is forbidden when `findings` is not `[]` (A7). Source: README Conventions; A7.
    R16,
    /// R-17: applicability carries `decision_revision` of the form `git:<7-40 hex>` equal to the
    /// decision's current or a listed revision, and `authorization.status`; a decided status
    /// requires `at`. Source: README rule 2 and Conventions.
    R17,
    /// R-18: materialized rows use their closed key set and reference existing findings. Source:
    /// Python `MATERIALIZED_KEYS`; F-1.
    R18,
}

impl RuleId {
    /// The stable rule identifier, for example `"R-03"` or `"F-5"`.
    pub fn as_str(self) -> &'static str {
        match self {
            RuleId::R01 => "R-01",
            RuleId::R02 => "R-02",
            RuleId::F5 => "F-5",
            RuleId::F6 => "F-6",
            RuleId::R03 => "R-03",
            RuleId::R04 => "R-04",
            RuleId::R05 => "R-05",
            RuleId::R05b => "R-05b",
            RuleId::R06 => "R-06",
            RuleId::R07 => "R-07",
            RuleId::R08 => "R-08",
            RuleId::F3 => "F-3",
            RuleId::F2 => "F-2",
            RuleId::R09 => "R-09",
            RuleId::R10 => "R-10",
            RuleId::R11 => "R-11",
            RuleId::R12 => "R-12",
            RuleId::R13 => "R-13",
            RuleId::R14 => "R-14",
            RuleId::R15 => "R-15",
            RuleId::R16 => "R-16",
            RuleId::R17 => "R-17",
            RuleId::R18 => "R-18",
        }
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a violation was found: a register-relative file, the line for JSON Lines records (or
/// the parser's line for a parse failure), and a JSON pointer (RFC 6901) inside the document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location {
    pub file: String,
    pub line: Option<u64>,
    pub pointer: String,
}

/// One rule violation in the register content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Violation {
    pub rule: RuleId,
    pub location: Location,
    pub detail: String,
}
