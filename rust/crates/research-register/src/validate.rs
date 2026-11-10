//! Whole-register validation of one [`PinnedSnapshot`].
//!
//! [`validate`] checks the register file set (R-01), decodes the records ([`crate::model::decode`]),
//! resolves references between records (R-09, R-15, R-17's revision match) and checks every pin
//! and decision revision against the snapshot's repository (R-10, R-12, R-13, R-14).
//!
//! Outcomes are kept apart. Rule violations make the register [`Outcome::Invalid`]. A Git object
//! that cannot be read, or an ancestry question that cannot be answered, is a reading outcome and
//! returns [`Error`] (F-4): validation stops there, whatever violations were already found,
//! because the register's validity cannot be established. An ambiguous abbreviated commit is a
//! violation of the pin's rule (A5), and so is a commit id naming an object that is not a commit.
//!
//! Reference checks (R-09, R-15, R-17) look only at records whose every field decoded. A record
//! with another defect is already a violation; its references are not checked a second time.

use crate::error::{BadRequest, CannotRead, Error};
use crate::model::{self, DecisionPin, Index, Lines, Pin, REGISTER_FILES, Records, STUDIES_DIR};
use crate::objects::{Ancestry, EntryKind, GitObjects, Prefix};
use crate::rules::{Location, RuleId, Violation};
use crate::snapshot::PinnedSnapshot;
use gix::hash::ObjectId;
use std::collections::HashMap;

/// A register that passed every rule at one commit. It is constructed only by [`validate`].
#[derive(Debug)]
pub struct ValidRegister {
    commit: ObjectId,
    register_path: String,
    register_tree: ObjectId,
    records: Records,
}

impl ValidRegister {
    fn new(snapshot: &PinnedSnapshot, records: Records) -> Self {
        ValidRegister {
            commit: snapshot.commit(),
            register_path: snapshot.register_path().to_owned(),
            register_tree: snapshot.register_tree(),
            records,
        }
    }
    pub fn commit(&self) -> ObjectId {
        self.commit
    }
    pub fn register_path(&self) -> &str {
        &self.register_path
    }
    pub fn register_tree(&self) -> ObjectId {
        self.register_tree
    }
    pub fn records(&self) -> &Records {
        &self.records
    }
}

/// The result of validating a register that could be read.
#[derive(Debug)]
pub enum Outcome {
    Valid(Box<ValidRegister>),
    /// At least one violation, in check order: R-01, decoding, references, Git-backed rules.
    Invalid(Vec<Violation>),
}

/// Validate the register in `snapshot`.
pub fn validate(snapshot: &PinnedSnapshot) -> Result<Outcome, Error> {
    let mut out = file_set(snapshot);
    let files = snapshot
        .entries()
        .iter()
        .filter(|(path, e)| {
            e.kind().is_file()
                && (REGISTER_FILES.contains(&path.as_str()) || model::study_name(path).is_some())
        })
        .filter_map(|(path, e)| e.bytes().map(|b| (path.as_str(), b)));
    let decoded = model::decode(files);
    out.extend(decoded.violations);
    references(&decoded.records, &decoded.index, &mut out);
    let mut git = Git::new(snapshot.objects());
    for pin in &decoded.pins {
        git.pin(pin, &mut out)?;
    }
    for d in &decoded.decision_pins {
        git.decision(d, &mut out)?;
    }
    Ok(if out.is_empty() {
        Outcome::Valid(Box::new(ValidRegister::new(snapshot, decoded.records)))
    } else {
        Outcome::Invalid(out)
    })
}

fn at(file: &str, line: Option<u64>, pointer: impl Into<String>) -> Location {
    Location {
        file: file.to_owned(),
        line,
        pointer: pointer.into(),
    }
}

fn violation(rule: RuleId, location: Location, detail: impl Into<String>) -> Violation {
    Violation {
        rule,
        location,
        detail: detail.into(),
    }
}

/// R-01: the register files are regular files, `studies/` is a tree, study files (`*.json`
/// directly under `studies/`) are regular files, and no entry name is non-UTF-8 (A6).
fn file_set(snapshot: &PinnedSnapshot) -> Vec<Violation> {
    let mut out = Vec::new();
    let need_file = |path: &str, out: &mut Vec<Violation>| match snapshot.entry(path) {
        None => out.push(violation(
            RuleId::R01,
            at(path, None, ""),
            format!("{path} is absent"),
        )),
        Some(e) if !e.kind().is_file() => out.push(violation(
            RuleId::R01,
            at(path, None, ""),
            format!("{path} is a {}, not a regular file", e.kind().as_str()),
        )),
        Some(_) => {}
    };
    for name in REGISTER_FILES {
        need_file(name, &mut out);
    }
    match snapshot.entry(STUDIES_DIR) {
        Some(e) if e.kind() == EntryKind::Tree => {
            for path in snapshot
                .entries()
                .keys()
                .filter(|p| model::study_name(p).is_some())
            {
                need_file(path, &mut out);
            }
        }
        other => out.push(violation(
            RuleId::R01,
            at("studies/", None, ""),
            match other {
                None => "studies/ is absent".to_owned(),
                Some(e) => format!("studies is a {}, not a tree", e.kind().as_str()),
            },
        )),
    }
    for name in snapshot.invalid_names() {
        out.push(violation(
            RuleId::R01,
            at(name, None, ""),
            "entry name is not UTF-8 (invalid content, A6)",
        ));
    }
    out
}

/// R-09 references, R-15 `superseded_by` naming a different decision, R-17 revision match.
fn references(r: &Records, ix: &Index, out: &mut Vec<Violation>) {
    let finding = |id: &str| ix.finding_ids.contains(id);
    let unknown = |loc: Location, what: &str, id: &str, out: &mut Vec<Violation>| {
        out.push(violation(
            RuleId::R09,
            loc,
            format!("{what} {id:?} does not exist"),
        ));
    };
    for s in &r.studies {
        for (i, q) in s.questions.iter().enumerate() {
            if !ix.question_ids.contains(q) {
                unknown(
                    at(
                        &s.at.file,
                        s.at.line,
                        format!("{}/questions/{i}", s.at.pointer),
                    ),
                    "question",
                    q,
                    out,
                );
            }
        }
    }
    for d in &r.decisions {
        for (i, f) in d.findings.iter().enumerate() {
            if !finding(f) {
                unknown(
                    at(
                        &d.at.file,
                        d.at.line,
                        format!("{}/findings/{i}", d.at.pointer),
                    ),
                    "finding",
                    f,
                    out,
                );
            }
        }
        if let Some(target) = d
            .historical
            .as_ref()
            .and_then(|h| h.superseded_by.as_deref())
        {
            let loc = at(
                &d.at.file,
                d.at.line,
                format!("{}/historical/superseded_by", d.at.pointer),
            );
            if !ix.decisions.contains_key(target) {
                unknown(loc, "decision", target, out);
            } else if target == d.decision_id {
                out.push(violation(
                    RuleId::R15,
                    loc,
                    "superseded_by must name a different decision",
                ));
            }
        }
    }
    for e in &r.events {
        if !finding(&e.subject_finding) {
            unknown(
                at(&e.at.file, e.at.line, "/subject_finding"),
                "finding",
                &e.subject_finding,
                out,
            );
        }
        if let Some(by) = &e.by_finding
            && !finding(by)
        {
            unknown(at(&e.at.file, e.at.line, "/by_finding"), "finding", by, out);
        }
    }
    for j in &r.judgments {
        if !finding(&j.finding) {
            unknown(
                at(&j.at.file, j.at.line, "/finding"),
                "finding",
                &j.finding,
                out,
            );
        }
    }
    for a in &r.applicability {
        if !finding(&a.finding) {
            unknown(
                at(&a.at.file, a.at.line, "/finding"),
                "finding",
                &a.finding,
                out,
            );
        }
        match ix.decisions.get(&a.decision_id) {
            None => unknown(
                at(&a.at.file, a.at.line, "/decision_id"),
                "decision",
                &a.decision_id,
                out,
            ),
            Some(revs) => {
                let text = &a.decision_revision.text;
                if revs.current.as_deref() != Some(text.as_str()) && !revs.listed.contains(text) {
                    out.push(violation(
                        RuleId::R17,
                        at(&a.at.file, a.at.line, "/decision_revision"),
                        format!(
                            "decision_revision {text:?} is neither the decision's current revision nor a listed one"
                        ),
                    ));
                }
            }
        }
    }
    for m in &r.materialized {
        if !finding(&m.finding) {
            unknown(
                at(&m.at.file, m.at.line, "/finding"),
                "finding",
                &m.finding,
                out,
            );
        }
    }
}

/// A pin commit, resolved.
#[derive(Clone)]
enum Resolved {
    Commit(ObjectId),
    /// A5: an abbreviation matching more than one object.
    Ambiguous,
    /// The id names an object of another type.
    NotACommit(String),
}

/// What a path names at a commit.
#[derive(Clone)]
enum AtPath {
    Absent,
    /// The path is not a plain relative path (empty, `.` or `..` components).
    BadPath,
    Entry {
        kind: EntryKind,
        id: ObjectId,
        /// Lines as Python counts them (`\n` count, plus one for an unterminated last line), for
        /// regular file entries (mode 100644 or 100755), whose blob has been read and verified.
        /// A link's blob is read by [`Git::pin`], never here.
        lines: Option<u64>,
    },
}

struct Git<'a> {
    objects: &'a GitObjects,
    commits: HashMap<String, Resolved>,
    paths: HashMap<(ObjectId, String), AtPath>,
    ancestry: HashMap<(ObjectId, ObjectId), bool>,
}

fn line_count(data: &[u8]) -> u64 {
    let newlines = data.iter().filter(|&&b| b == b'\n').count() as u64;
    newlines + u64::from(!data.is_empty() && !data.ends_with(b"\n"))
}

impl<'a> Git<'a> {
    fn new(objects: &'a GitObjects) -> Self {
        Git {
            objects,
            commits: HashMap::new(),
            paths: HashMap::new(),
            ancestry: HashMap::new(),
        }
    }

    /// Resolve a 7-40 hex commit. An absent object is `cannot_read` (F-4).
    fn commit(&mut self, hex: &str) -> Result<Resolved, Error> {
        if let Some(r) = self.commits.get(hex) {
            return Ok(r.clone());
        }
        let r = match self.objects.resolve_prefix(hex)? {
            Prefix::Missing => {
                return Err(CannotRead::ObjectUnavailable { id: hex.to_owned() }.into());
            }
            Prefix::Ambiguous => Resolved::Ambiguous,
            Prefix::Unique(id) => match self.objects.commit(id) {
                Ok(_) => Resolved::Commit(id),
                Err(Error::CannotRead(CannotRead::TypeMismatch {
                    id: got, actual, ..
                })) if ObjectId::from_hex(got.as_bytes()).ok() == Some(id) => {
                    Resolved::NotACommit(actual)
                }
                Err(e) => return Err(e),
            },
        };
        self.commits.insert(hex.to_owned(), r.clone());
        Ok(r)
    }

    fn at_path(&mut self, commit: ObjectId, path: &str) -> Result<AtPath, Error> {
        let key = (commit, path.to_owned());
        if let Some(a) = self.paths.get(&key) {
            return Ok(a.clone());
        }
        let a = match self.objects.entry_at(commit, path) {
            Err(Error::BadRequest(BadRequest::Path { .. })) => AtPath::BadPath,
            Err(e) => return Err(e),
            Ok(None) => AtPath::Absent,
            Ok(Some(e)) => {
                let lines = if e.kind.is_file() {
                    Some(line_count(&self.objects.blob(e.id)?))
                } else {
                    None
                };
                AtPath::Entry {
                    kind: e.kind,
                    id: e.id,
                    lines,
                }
            }
        };
        self.paths.insert(key, a.clone());
        Ok(a)
    }

    /// `ancestor` is an ancestor of `descendant`; an unanswerable question is `cannot_read`.
    fn is_ancestor(&mut self, ancestor: ObjectId, descendant: ObjectId) -> Result<bool, Error> {
        if let Some(&b) = self.ancestry.get(&(ancestor, descendant)) {
            return Ok(b);
        }
        let b = match self.objects.is_ancestor(ancestor, descendant)? {
            Ancestry::Yes => true,
            Ancestry::No => false,
            Ancestry::CannotEstablish => {
                return Err(CannotRead::Ancestry {
                    ancestor: ancestor.to_string(),
                    descendant: descendant.to_string(),
                }
                .into());
            }
        };
        self.ancestry.insert((ancestor, descendant), b);
        Ok(b)
    }

    /// The commit of a pin or revision, or the violation (under `rule`) that it does not name one.
    fn named_commit(
        &mut self,
        hex: &str,
        rule: RuleId,
        loc: &Location,
        out: &mut Vec<Violation>,
    ) -> Result<Option<ObjectId>, Error> {
        Ok(match self.commit(hex)? {
            Resolved::Commit(id) => Some(id),
            Resolved::Ambiguous => {
                out.push(violation(
                    rule,
                    loc.clone(),
                    format!("commit {hex} is ambiguous (A5)"),
                ));
                None
            }
            Resolved::NotACommit(kind) => {
                out.push(violation(
                    rule,
                    loc.clone(),
                    format!("commit {hex} names a {kind}, not a commit"),
                ));
                None
            }
        })
    }

    /// `lines` must lie inside a blob of `count` lines (as Python counts).
    fn lines(
        lines: Option<Lines>,
        count: Option<u64>,
        rule: RuleId,
        loc: Location,
        out: &mut Vec<Violation>,
    ) {
        if let (Some(l), Some(count)) = (lines, count)
            && l.last > count
        {
            out.push(violation(
                rule,
                loc,
                format!(
                    "lines {}-{} exceed the {count} lines of the file",
                    l.first, l.last
                ),
            ));
        }
    }

    /// R-10: the commit resolves, `commit:path` is a blob equal to `blob`, `lines` lie inside it.
    fn pin(&mut self, pin: &Pin, out: &mut Vec<Violation>) -> Result<(), Error> {
        let Some(commit) = self.named_commit(&pin.commit, RuleId::R10, &pin.at, out)? else {
            return Ok(());
        };
        let entry = self.at_path(commit, &pin.path)?;
        // A link's object is a blob (stage B choice 3): read and verify it here, at the point
        // where every pin entry's blob is read, so a missing link blob stays `cannot_read`.
        let count = match &entry {
            AtPath::Entry {
                kind: EntryKind::Link,
                id,
                ..
            } => Some(line_count(&self.objects.blob(*id)?)),
            AtPath::Entry { lines, .. } => *lines,
            _ => None,
        };
        let bad = |detail: String, out: &mut Vec<Violation>| {
            out.push(violation(RuleId::R10, pin.at.clone(), detail))
        };
        match &entry {
            AtPath::Absent | AtPath::BadPath => {
                bad(format!("{}:{} does not resolve", pin.commit, pin.path), out);
            }
            AtPath::Entry { kind, id, .. } => {
                if ObjectId::from_hex(pin.blob.as_bytes()).ok() != Some(*id) {
                    bad(
                        format!(
                            "{}:{} is {id}, the pin says {}",
                            pin.commit, pin.path, pin.blob
                        ),
                        out,
                    );
                } else if !(kind.is_file() || *kind == EntryKind::Link) {
                    // A link's object is a blob, which the catalogue and Python accept.
                    bad(
                        format!(
                            "{}:{} is a {}, not a blob",
                            pin.commit,
                            pin.path,
                            kind.as_str()
                        ),
                        out,
                    );
                } else {
                    let mut lines_at = pin.at.clone();
                    lines_at.pointer.push_str("/lines");
                    Self::lines(pin.lines, count, RuleId::R10, lines_at, out);
                }
            }
        }
        Ok(())
    }

    /// A decision's (R-12) or listed revision's (R-13) document at its resolved commit: it must
    /// be a regular file entry (mode 100644 or 100755) whose object is read and verified as a
    /// blob, whether or not `lines` is given. A link, tree, gitlink or other entry is a violation
    /// decided by its mode, without reading its object; an unreadable blob of a regular file, or
    /// an unreadable tree on the way to it, is `cannot_read`.
    fn document(
        &mut self,
        commit: ObjectId,
        document: &str,
        lines: Option<Lines>,
        rule: RuleId,
        loc: &Location,
        out: &mut Vec<Violation>,
    ) -> Result<(), Error> {
        match self.at_path(commit, document)? {
            AtPath::Absent | AtPath::BadPath => out.push(violation(
                rule,
                loc.clone(),
                format!("{commit}:{document} does not resolve"),
            )),
            AtPath::Entry { kind, .. } if !kind.is_file() => out.push(violation(
                rule,
                loc.clone(),
                format!(
                    "{commit}:{document} is a {}, not a regular file",
                    kind.as_str()
                ),
            )),
            AtPath::Entry { lines: count, .. } => {
                let mut lines_at = loc.clone();
                lines_at.pointer.push_str("/lines");
                Self::lines(lines, count, rule, lines_at, out);
            }
        }
        Ok(())
    }

    /// R-12 (current revision and document), R-13 (each listed revision resolves, differs from
    /// the current one and from the others), R-14 (ancestry).
    fn decision(&mut self, d: &DecisionPin, out: &mut Vec<Violation>) -> Result<(), Error> {
        let mut rev_at = d.at.clone();
        rev_at.pointer.push_str("/revision");
        let mut current = None;
        if let Some(rev) = &d.revision {
            current = self.named_commit(&rev.commit, RuleId::R12, &rev_at, out)?;
            if let (Some(id), Some(doc)) = (current, &d.document) {
                self.document(id, doc, d.lines, RuleId::R12, &d.at, out)?;
            }
        }
        let mut resolved: Vec<(usize, ObjectId)> = Vec::new();
        let mut seen: HashMap<ObjectId, usize> = HashMap::new();
        for (i, r) in d.revisions.iter().enumerate() {
            let Some(rev) = &r.revision else { continue };
            let Some(id) = self.named_commit(&rev.commit, RuleId::R13, &r.at, out)? else {
                continue;
            };
            if Some(id) == current {
                out.push(violation(
                    RuleId::R13,
                    r.at.clone(),
                    format!("revision {:?} is the current revision", rev.text),
                ));
            } else if let Some(j) = seen.get(&id) {
                out.push(violation(
                    RuleId::R13,
                    r.at.clone(),
                    format!("revision {:?} repeats revisions[{j}]", rev.text),
                ));
            } else {
                seen.insert(id, i);
            }
            resolved.push((i, id));
            if let Some(doc) = &r.document {
                self.document(id, doc, r.lines, RuleId::R13, &r.at, out)?;
            }
        }
        for pair in resolved.windows(2) {
            let ((i, a), (j, b)) = (pair[0], pair[1]);
            if a != b && !self.is_ancestor(a, b)? {
                out.push(violation(
                    RuleId::R14,
                    d.revisions[i].at.clone(),
                    format!("revisions[{i}] is not an ancestor of revisions[{j}]"),
                ));
            }
        }
        if let Some(cur) = current {
            for &(i, a) in &resolved {
                if a != cur && !self.is_ancestor(a, cur)? {
                    out.push(violation(
                        RuleId::R14,
                        d.revisions[i].at.clone(),
                        format!("revisions[{i}] is not an ancestor of the current revision"),
                    ));
                }
            }
        }
        Ok(())
    }
}
