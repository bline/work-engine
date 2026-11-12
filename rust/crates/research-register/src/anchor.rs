//! Decision anchors: whether each decision's anchored text still stands where the register says,
//! at the register's own commit.
//!
//! This reproduces `metrics/scripts/register.py anchors` (`classify_decision` and its helpers)
//! for the one target this crate allows: the commit of a [`ValidRegister`]. Every object comes
//! from the snapshot's verified [`GitObjects`]; nothing runs git, reads a working tree or writes.
//! The report is read-only: nothing is re-anchored, no record is changed and no authorization is
//! transferred. Applicability rows, notes and the summary are not part of it.
//!
//! Interim (owner question Q-R2-2): rename detection is not implemented. When the document is not
//! a blob at its revision path at the target commit, the entry has no class and the error
//! `not_established`; `register.py` would report `absent` or follow a rename.

pub mod diff;

use crate::error::{BadRequest, CannotRead, Error};
use crate::model::{DECISIONS, Decision, Revision};
use crate::objects::{Ancestry, EntryKind, GitObjects, Prefix};
use crate::parse::{self, Json};
use crate::snapshot::PinnedSnapshot;
use crate::validate::ValidRegister;
use gix::hash::ObjectId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt;

pub use diff::{Hunk, line_hunks};

/// Lines of context on each side of an anchored range (`register.py` `CONTEXT_LINES`).
pub const CONTEXT_LINES: usize = 3;
pub const STATEMENT: &str =
    "Nothing was re-anchored, no register record was changed and no authorization was transferred.";
pub const NOT_MARKDOWN: &str = "not applicable (not a Markdown document)";
pub const RENAME_INTERIM: &str = "document is not at its revision path at the target commit; rename detection is not implemented natively (owner question Q-R2-2)";

/// The approved anchor classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnchorClass {
    Unchanged,
    Moved,
    Modified,
    RelocatedVerbatim,
    Ambiguous,
    Absent,
}

impl AnchorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            AnchorClass::Unchanged => "unchanged",
            AnchorClass::Moved => "moved",
            AnchorClass::Modified => "modified",
            AnchorClass::RelocatedVerbatim => "relocated_verbatim",
            AnchorClass::Ambiguous => "ambiguous",
            AnchorClass::Absent => "absent",
        }
    }
}

/// Why no report was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorError {
    /// A reading failure (F-4), or the refusal of a snapshot that is not the validated
    /// register's (`bad_request`).
    Read(Error),
    /// A property a valid register guarantees did not hold (for example a revision that has
    /// become ambiguous). Never reported as a class.
    Invariant(String),
}

impl AnchorError {
    pub fn category(&self) -> &'static str {
        match self {
            AnchorError::Read(e) => e.category(),
            AnchorError::Invariant(_) => "invariant_failure",
        }
    }
}

impl fmt::Display for AnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnchorError::Read(e) => e.fmt(f),
            AnchorError::Invariant(s) => write!(f, "invariant failure: {s}"),
        }
    }
}

impl std::error::Error for AnchorError {}

impl From<Error> for AnchorError {
    fn from(e: Error) -> Self {
        AnchorError::Read(e)
    }
}

/// One decision's result. [`AnchorEntry::json`] holds every field under `register.py`'s names.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorEntry {
    pub decision_id: String,
    /// `None` for an error entry (only the rename interim produces one).
    pub class: Option<AnchorClass>,
    pub owner_review_required: bool,
    json: Value,
}

impl AnchorEntry {
    pub fn json(&self) -> &Value {
        &self.json
    }
}

/// The anchors of every decision of one valid register, at its own commit.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorReport {
    pub pinned_commit: ObjectId,
    pub register_path: String,
    pub register_tree: ObjectId,
    /// Sorted by `decision_id` (unique in a valid register, R-06).
    pub entries: Vec<AnchorEntry>,
}

impl AnchorReport {
    /// Deterministic JSON form (object keys sorted).
    pub fn to_json(&self) -> Value {
        json!({
            "pinned_commit": self.pinned_commit.to_string(),
            "register_path": self.register_path,
            "register_tree": self.register_tree.to_string(),
            "statement": STATEMENT,
            "entries": self.entries.iter().map(|e| e.json.clone()).collect::<Vec<_>>(),
        })
    }
}

/// Classify every decision of `valid` at its own commit. `snapshot` must be the snapshot `valid`
/// was validated from (same commit, register path and register tree), else `bad_request`.
pub fn anchors(
    valid: &ValidRegister,
    snapshot: &PinnedSnapshot,
) -> Result<AnchorReport, AnchorError> {
    if snapshot.commit() != valid.commit()
        || snapshot.register_path() != valid.register_path()
        || snapshot.register_tree() != valid.register_tree()
    {
        return Err(Error::from(BadRequest::Revision {
            rev: snapshot.requested().to_owned(),
            reason: format!(
                "snapshot {} {} {} is not the validated register {} {} {}",
                snapshot.commit(),
                snapshot.register_path(),
                snapshot.register_tree(),
                valid.commit(),
                valid.register_path(),
                valid.register_tree()
            ),
        })
        .into());
    }
    let raw = RawDecisions::read(snapshot)?;
    let target = valid.commit();
    let mut decisions: Vec<&Decision> = valid.records().decisions.iter().collect();
    decisions.sort_by(|a, b| a.decision_id.cmp(&b.decision_id));
    let mut entries = Vec::with_capacity(decisions.len());
    let mut ancestry = HashMap::new();
    for d in decisions {
        entries.push(classify(
            snapshot.objects(),
            target,
            d,
            raw.lines(d)?,
            &mut ancestry,
        )?);
    }
    Ok(AnchorReport {
        pinned_commit: target,
        register_path: valid.register_path().to_owned(),
        register_tree: valid.register_tree(),
        entries,
    })
}

/// `decisions.json` as parsed JSON, for the `lines` text exactly as written (the typed record
/// keeps only the parsed range; `register.py` echoes the string).
struct RawDecisions(Json);

impl RawDecisions {
    fn read(snapshot: &PinnedSnapshot) -> Result<Self, AnchorError> {
        let bytes = snapshot
            .entry(DECISIONS)
            .and_then(|e| e.bytes())
            .ok_or_else(|| AnchorError::Invariant(format!("{DECISIONS} is not in the snapshot")))?;
        parse::document(DECISIONS, bytes, &mut Vec::new())
            .map(RawDecisions)
            .ok_or_else(|| AnchorError::Invariant(format!("{DECISIONS} does not parse")))
    }

    fn lines(&self, d: &Decision) -> Result<Option<String>, AnchorError> {
        let Some(range) = d.lines else {
            return Ok(None);
        };
        let index =
            d.at.pointer
                .strip_prefix('/')
                .and_then(|s| s.parse::<usize>().ok());
        let text = match (&self.0, index) {
            (Json::Array(items), Some(i)) => items
                .get(i)
                .and_then(|o| o.get("lines"))
                .and_then(Json::as_str),
            _ => None,
        };
        match text {
            Some(t) => Ok(Some(t.to_owned())),
            None => Err(AnchorError::Invariant(format!(
                "decision {} lines {}-{} not found at {}",
                d.decision_id, range.first, range.last, d.at.pointer
            ))),
        }
    }
}

/// `register.py` `split_lines`: split on `\n`, dropping one trailing empty part.
pub fn split_lines(data: &[u8]) -> Vec<&[u8]> {
    let mut parts: Vec<&[u8]> = data.split(|&c| c == b'\n').collect();
    if parts.last() == Some(&&b""[..]) {
        parts.pop();
    }
    parts
}

/// `register.py` `fmt_lines`.
pub fn fmt_lines(lo: usize, hi: usize) -> String {
    if lo == hi {
        lo.to_string()
    } else {
        format!("{lo}-{hi}")
    }
}

/// `register.py` `anchored_sha`.
pub fn anchored_sha(lines: &[&[u8]]) -> String {
    let mut h = Sha256::new();
    for (i, l) in lines.iter().enumerate() {
        if i > 0 {
            h.update(b"\n");
        }
        h.update(l);
    }
    h.update(b"\n");
    format!("{:x}", h.finalize())
}

/// `register.py` `find_occurrences`: 1-based starts of every exact occurrence of `anchor`.
pub fn find_occurrences(target: &[&[u8]], anchor: &[&[u8]]) -> Vec<usize> {
    let k = anchor.len();
    if k == 0 || target.len() < k {
        return Vec::new();
    }
    (0..=target.len() - k)
        .filter(|&i| target[i..i + k] == *anchor)
        .map(|i| i + 1)
        .collect()
}

/// `register.py` `normalise_hunks`: drop lines a hunk marks as changed although their text is
/// equal on both sides (a final newline added or removed).
pub fn normalise_hunks(hunks: &[Hunk], old: &[&[u8]], new: &[&[u8]]) -> Vec<Hunk> {
    let mut out = Vec::new();
    for &(a, b, c, d) in hunks {
        if b == 0 || d == 0 {
            out.push((a, b, c, d));
            continue;
        }
        let mut i = 0;
        while i < b && i < d && old[a - 1 + i] == new[c - 1 + i] {
            i += 1;
        }
        let mut j = 0;
        while j < b - i && j < d - i && old[a + b - 2 - j] == new[c + d - 2 - j] {
            j += 1;
        }
        let (nb, nd) = (b - i - j, d - i - j);
        if nb == 0 && nd == 0 {
            continue;
        }
        out.push(if nb == 0 {
            (a + i - 1, 0, c + i, nd)
        } else if nd == 0 {
            (a + i, nb, c + i - 1, 0)
        } else {
            (a + i, nb, c + i, nd)
        });
    }
    out
}

/// `register.py` `track`: `(touched, new_lo, span)` for old lines `lo..=hi`.
pub fn track(hunks: &[Hunk], lo: usize, hi: usize) -> (bool, i64, Option<(i64, i64)>) {
    let h: Vec<(i64, i64, i64, i64)> = hunks
        .iter()
        .map(|&(a, b, c, d)| (a as i64, b as i64, c as i64, d as i64))
        .collect();
    let (lo, hi) = (lo as i64, hi as i64);
    let mut touched = h.iter().any(|&(a, b, _, _)| b == 0 && a >= lo && a < hi);
    let nlo = lo
        + h.iter()
            .map(|&(a, b, _, d)| {
                if b > 0 && a + b - 1 < lo {
                    d - b
                } else if b == 0 && a < lo {
                    d
                } else {
                    0
                }
            })
            .sum::<i64>();
    let mut nums = Vec::new();
    for line in lo..=hi {
        let (mut off, mut inside) = (0, None);
        for &(a, b, c, d) in &h {
            if b > 0 {
                if a + b - 1 < line {
                    off += d - b;
                } else if a <= line {
                    inside = Some((a, c, d));
                }
            } else if a < line {
                off += d;
            }
        }
        match inside {
            None => nums.push(line + off),
            Some((a, c, d)) => {
                touched = true;
                if d > 0 {
                    nums.push(c + (line - a).min(d - 1));
                }
            }
        }
    }
    let span = match (nums.iter().min(), nums.iter().max()) {
        (Some(&x), Some(&y)) => Some((x, y)),
        _ => None,
    };
    (touched, nlo, span)
}

/// Python's `str.isspace()` set, which `str.strip()` and the `re` class `\s` use: Unicode
/// White_Space plus U+001C..U+001F (absent from [`char::is_whitespace`]).
pub fn py_space(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r'
            | '\u{1c}'..='\u{1f}'
            | ' '
            | '\u{85}'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
    )
}

/// A Markdown heading of a section chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: usize,
    pub line: usize,
    pub text: String,
}

impl Heading {
    fn json(&self) -> Value {
        json!({"level": self.level, "line": self.line, "text": self.text})
    }
}

/// `FENCE_OPEN = ^\s*(`{3,}|~{3,})(.*)$`: the fence character, its run length and the rest.
fn fence(text: &str) -> Option<(char, usize, &str)> {
    let rest = text.trim_start_matches(py_space);
    let c = rest.chars().next().filter(|&c| c == '`' || c == '~')?;
    let n = rest.bytes().take_while(|&b| b == c as u8).count();
    (n >= 3).then(|| (c, n, &rest[n..]))
}

/// `MD_HEADING = ^ {0,3}#{1,6}(?:[ \t]|$)`; the level is the run of `#`.
fn heading_level(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    let spaces = b.iter().take_while(|&&c| c == b' ').count();
    if spaces > 3 {
        return None;
    }
    let hashes = b[spaces..].iter().take_while(|&&c| c == b'#').count();
    let next = b.get(spaces + hashes);
    ((1..=6).contains(&hashes) && matches!(next, None | Some(b' ') | Some(b'\t'))).then_some(hashes)
}

/// `register.py` `markdown_chain`: the headings enclosing line `lineno` (1-based; a heading on
/// that line counts), outermost first, fenced code excluded. Setext headings are not recognised.
pub fn markdown_chain(lines: &[&[u8]], lineno: usize) -> Vec<Heading> {
    let mut chain: Vec<Heading> = Vec::new();
    let mut open: Option<(char, usize)> = None;
    for (i, raw) in lines.iter().take(lineno).enumerate() {
        let text = String::from_utf8_lossy(raw);
        if let Some((c, n)) = open {
            if let Some((fc, fn_, rest)) = fence(&text)
                && fc == c
                && fn_ >= n
                && rest.trim_matches(py_space).is_empty()
            {
                open = None;
            }
            continue;
        }
        if let Some((c, n, rest)) = fence(&text)
            && !(c == '`' && rest.contains('`'))
        {
            open = Some((c, n));
            continue;
        }
        if let Some(level) = heading_level(&text) {
            while chain.last().is_some_and(|h| h.level >= level) {
                chain.pop();
            }
            chain.push(Heading {
                level,
                line: i + 1,
                text: text.trim_matches(py_space).to_owned(),
            });
        }
    }
    chain
}

fn section_identity(chain: &[Heading]) -> Vec<(usize, &str)> {
    chain.iter().map(|h| (h.level, h.text.as_str())).collect()
}

fn text_of(lines: &[&[u8]]) -> Vec<String> {
    lines
        .iter()
        .map(|l| String::from_utf8_lossy(l).into_owned())
        .collect()
}

/// `register.py` `surroundings`, with the chain also returned for the section comparison.
fn surroundings(lines: &[&[u8]], lo: usize, hi: usize, is_md: bool) -> (Value, Vec<Heading>) {
    let chain = if is_md {
        markdown_chain(lines, lo)
    } else {
        Vec::new()
    };
    let (heading, section_chain) = if is_md {
        (
            chain.last().map_or(Value::Null, Heading::json),
            Value::Array(chain.iter().map(Heading::json).collect()),
        )
    } else {
        (json!(NOT_MARKDOWN), json!(NOT_MARKDOWN))
    };
    let before = &lines[(lo - 1).saturating_sub(CONTEXT_LINES)..lo - 1];
    let after = &lines[hi.min(lines.len())..(hi + CONTEXT_LINES).min(lines.len())];
    let v = json!({
        "lines": fmt_lines(lo, hi),
        "heading": heading,
        "section_chain": section_chain,
        "before": text_of(before),
        "after": text_of(after),
    });
    (v, chain)
}

fn is_markdown(path: &str) -> bool {
    let p = path.to_lowercase();
    p.ends_with(".md") || p.ends_with(".markdown")
}

/// The one commit a decision revision names, as validation resolves it.
fn revision_commit(objects: &GitObjects, d: &Decision) -> Result<ObjectId, AnchorError> {
    match objects.resolve_prefix(&d.revision.commit)? {
        Prefix::Unique(id) => {
            objects.commit(id)?;
            Ok(id)
        }
        other => Err(unresolved_revision(&other, &d.decision_id, &d.revision)),
    }
}

/// Why a validated revision no longer resolves to one commit. A commit that has become
/// unavailable is `cannot_read` (F-4), as validation reports it; an ambiguity is excluded by
/// validation (A5), so it is an invariant failure.
fn unresolved_revision(prefix: &Prefix, decision_id: &str, revision: &Revision) -> AnchorError {
    match prefix {
        Prefix::Missing => AnchorError::Read(
            CannotRead::ObjectUnavailable {
                id: revision.commit.clone(),
            }
            .into(),
        ),
        other => AnchorError::Invariant(format!(
            "decision {decision_id} revision {} does not resolve to one commit ({other:?})",
            revision.text
        )),
    }
}

/// `register.py` `classify_decision` for one decision of a valid register.
fn classify(
    objects: &GitObjects,
    target: ObjectId,
    d: &Decision,
    raw_lines: Option<String>,
    ancestry: &mut HashMap<ObjectId, Ancestry>,
) -> Result<AnchorEntry, AnchorError> {
    let invariant =
        |what: String| AnchorError::Invariant(format!("decision {}: {what}", d.decision_id));
    let rev = revision_commit(objects, d)?;
    let answer = match ancestry.get(&rev) {
        Some(&a) => a,
        None => {
            let a = objects.is_ancestor(rev, target)?;
            ancestry.insert(rev, a);
            a
        }
    };
    let ancestor = match answer {
        Ancestry::Yes => json!(true),
        Ancestry::No => json!(false),
        Ancestry::CannotEstablish => Value::Null,
    };
    let doc = d.document.as_str();
    let rev_entry = match objects.entry_at(rev, doc)? {
        Some(e) if e.kind.is_file() => e,
        other => {
            return Err(invariant(format!(
                "{doc} is not a regular file at {rev} ({:?})",
                other.map(|e| e.kind)
            )));
        }
    };
    let rev_bytes = objects.blob(rev_entry.id)?;
    let rev_lines = split_lines(&rev_bytes);
    if let Some(l) = d.lines
        && (l.last as usize > rev_lines.len() || l.first < 1)
    {
        return Err(invariant(format!(
            "lines {}-{} exceed the {} lines of {doc} at {rev}",
            l.first,
            l.last,
            rev_lines.len()
        )));
    }
    // The document at the target: a blob entry at the same path (a regular file or a symbolic
    // link, as `cat-file -t C:path` reports `blob` for both). Its object is read and verified.
    let tgt = match objects.entry_at(target, doc)? {
        Some(e) if e.kind.is_file() || e.kind == EntryKind::Link => {
            let bytes = objects.blob(e.id)?;
            Some((e.id, bytes))
        }
        _ => None,
    };
    let mut e = Builder::new(d, raw_lines, rev, ancestor);
    if let Some((id, _)) = &tgt {
        e.set(
            "document_at_target",
            json!({"path": doc, "renamed": false, "rename_status": null, "blob": id.to_string()}),
        );
    }
    let mut before = serde_json::Map::new();
    before.insert("revision".into(), json!(d.revision.text));
    before.insert("revision_commit".into(), json!(rev.to_string()));
    before.insert("document_blob".into(), json!(rev_entry.id.to_string()));
    let Some(range) = d.lines else {
        e.set("before", Value::Object(before));
        return Ok(match tgt {
            None => e.not_established(),
            Some((id, _)) if id == rev_entry.id => {
                e.set("after", json!({"document_blob": id.to_string()}));
                e.class(AnchorClass::Unchanged)
            }
            Some((id, _)) => {
                e.set("after", json!({"document_blob": id.to_string()}));
                e.set(
                    "candidates",
                    json!([{"document": doc, "basis": "whole document differs; candidate only"}]),
                );
                e.class(AnchorClass::Modified)
            }
        });
    };
    let (lo, hi) = (range.first as usize, range.last as usize);
    let anchor = &rev_lines[lo - 1..hi];
    before.insert("lines".into(), json!(fmt_lines(lo, hi)));
    before.insert("anchored_text_sha256".into(), json!(anchored_sha(anchor)));
    e.set("before", Value::Object(before));
    let Some((_, t_bytes)) = tgt else {
        return Ok(e.not_established());
    };
    let t_lines = split_lines(&t_bytes);
    let hunks = normalise_hunks(&line_hunks(&rev_bytes, &t_bytes), &rev_lines, &t_lines);
    let is_md = is_markdown(doc);
    let occ = find_occurrences(&t_lines, anchor);
    let (touched, nlo, span_new) = track(&hunks, lo, hi);
    let n = hi - lo + 1;
    if occ.len() > 1 {
        let c: Vec<Value> = occ
            .iter()
            .map(|&o| json!({"lines": fmt_lines(o, o + n - 1), "basis": "exact anchored text occurs here"}))
            .collect();
        e.set("candidates", Value::Array(c));
        return Ok(e.class(AnchorClass::Ambiguous));
    }
    if let [o] = occ[..] {
        e.set("after", json!({"lines": fmt_lines(o, o + n - 1)}));
        let at_nlo = o as i64 == nlo;
        if !touched && at_nlo && o == lo {
            return Ok(e.class(AnchorClass::Unchanged));
        }
        let class = if !touched && at_nlo {
            AnchorClass::Moved
        } else {
            AnchorClass::RelocatedVerbatim
        };
        let (cb, chain_b) = surroundings(&rev_lines, lo, hi, is_md);
        let (ca, chain_a) = surroundings(&t_lines, o, o + n - 1, is_md);
        e.set("context_before", cb);
        e.set("context_after", ca);
        if class == AnchorClass::RelocatedVerbatim {
            e.review("relocated_verbatim");
        } else if is_md && section_identity(&chain_b) != section_identity(&chain_a) {
            e.review("section_identity_changed");
        }
        return Ok(e.class(class));
    }
    Ok(match span_new {
        Some((first, last)) => {
            let last = last.min(t_lines.len() as i64);
            let lines = if first == last {
                first.to_string()
            } else {
                format!("{first}-{last}")
            };
            e.set(
                "candidates",
                json!([{"lines": lines, "basis": "line tracking through the diff; candidate only, not chosen"}]),
            );
            e.class(AnchorClass::Modified)
        }
        None => e.class(AnchorClass::Absent),
    })
}

/// An entry under construction, starting from `register.py` `blank_entry` (without `notes`).
struct Builder {
    decision_id: String,
    review: bool,
    reasons: Vec<&'static str>,
    fields: serde_json::Map<String, Value>,
}

impl Builder {
    fn new(d: &Decision, raw_lines: Option<String>, rev: ObjectId, ancestor: Value) -> Self {
        let mut fields = serde_json::Map::new();
        for (k, v) in [
            ("decision_id", json!(d.decision_id)),
            ("document", json!(d.document)),
            ("anchor", json!(d.anchor)),
            ("revision", json!(d.revision.text)),
            ("lines", json!(raw_lines)),
            ("owner", json!(d.owner)),
            ("class", Value::Null),
            ("error", Value::Null),
            ("document_at_target", Value::Null),
            ("candidates", json!([])),
            ("before", Value::Null),
            ("after", Value::Null),
            ("context_before", Value::Null),
            ("context_after", Value::Null),
            ("revision_commit", json!(rev.to_string())),
            ("revision_is_ancestor_of_target", ancestor),
        ] {
            fields.insert(k.into(), v);
        }
        Builder {
            decision_id: d.decision_id.clone(),
            review: false,
            reasons: Vec::new(),
            fields,
        }
    }

    fn set(&mut self, k: &str, v: Value) {
        self.fields.insert(k.into(), v);
    }

    fn review(&mut self, reason: &'static str) {
        self.review = true;
        self.reasons.push(reason);
    }

    fn finish(mut self, class: Option<AnchorClass>) -> AnchorEntry {
        self.fields
            .insert("class".into(), json!(class.map(AnchorClass::as_str)));
        self.fields
            .insert("owner_review_required".into(), json!(self.review));
        self.fields
            .insert("owner_review_reasons".into(), json!(self.reasons));
        AnchorEntry {
            decision_id: self.decision_id,
            class,
            owner_review_required: self.review,
            json: Value::Object(self.fields),
        }
    }

    fn class(self, class: AnchorClass) -> AnchorEntry {
        self.finish(Some(class))
    }

    /// The rename interim (Q-R2-2).
    fn not_established(mut self) -> AnchorEntry {
        self.set(
            "error",
            json!({"kind": "not_established", "reason": RENAME_INTERIM}),
        );
        self.finish(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F-01: a missing revision is F-4 `cannot_read`; an ambiguous one stays an invariant failure.
    /// (Ambiguity cannot be produced end to end: a register revision has at least 7 hex digits,
    /// so it would need a 28-bit commit-prefix collision after validation.)
    #[test]
    fn unresolved_revision_classes() {
        let rev = Revision {
            text: "git:0123456789".into(),
            commit: "0123456789".into(),
        };
        let missing = unresolved_revision(&Prefix::Missing, "d-x", &rev);
        assert_eq!(missing.category(), "cannot_read");
        assert_eq!(
            missing,
            AnchorError::Read(Error::CannotRead(CannotRead::ObjectUnavailable {
                id: "0123456789".into()
            }))
        );
        let ambiguous = unresolved_revision(&Prefix::Ambiguous, "d-x", &rev);
        assert_eq!(ambiguous.category(), "invariant_failure");
        assert_eq!(
            ambiguous.to_string(),
            "invariant failure: decision d-x revision git:0123456789 does not resolve to one \
             commit (Ambiguous)"
        );
    }

    #[test]
    fn split_lines_matches_python() {
        assert_eq!(split_lines(b""), Vec::<&[u8]>::new());
        assert_eq!(split_lines(b"\n"), vec![&b""[..]]);
        assert_eq!(split_lines(b"a\nb"), vec![&b"a"[..], b"b"]);
        assert_eq!(split_lines(b"a\nb\n"), vec![&b"a"[..], b"b"]);
        assert_eq!(split_lines(b"a\n\n"), vec![&b"a"[..], b""]);
        assert_eq!(split_lines(b"a\r\nb"), vec![&b"a\r"[..], b"b"]);
    }

    #[test]
    fn anchored_sha_joins_with_final_newline() {
        let want = format!("{:x}", Sha256::digest(b"a\nb\n"));
        assert_eq!(anchored_sha(&[b"a", b"b"]), want);
        assert_eq!(fmt_lines(3, 3), "3");
        assert_eq!(fmt_lines(3, 5), "3-5");
    }

    #[test]
    fn occurrences() {
        let t: Vec<&[u8]> = vec![b"a", b"b", b"a", b"b", b"a"];
        assert_eq!(find_occurrences(&t, &[b"a", b"b"]), vec![1, 3]);
        assert_eq!(find_occurrences(&t, &[b"b", b"a"]), vec![2, 4]);
        assert_eq!(find_occurrences(&t, &[b"c"]), Vec::<usize>::new());
        assert_eq!(
            find_occurrences(&t[..1], &[b"a", b"b"]),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn normalise_drops_final_newline_only_change() {
        let old: Vec<&[u8]> = vec![b"a", b"b"];
        let new: Vec<&[u8]> = vec![b"a", b"b"];
        // git marks line 2 changed when only its final newline differs.
        assert_eq!(normalise_hunks(&[(2, 1, 2, 1)], &old, &new), vec![]);
        let new2: Vec<&[u8]> = vec![b"a", b"b", b"c"];
        assert_eq!(
            normalise_hunks(&[(2, 1, 2, 2)], &old, &new2),
            vec![(2, 0, 3, 1)]
        );
        let old3: Vec<&[u8]> = vec![b"a", b"x", b"b"];
        assert_eq!(
            normalise_hunks(&[(2, 2, 2, 1)], &old3, &new),
            vec![(2, 1, 1, 0)]
        );
        // Pure insertions and deletions pass through.
        assert_eq!(
            normalise_hunks(&[(1, 0, 2, 1), (3, 1, 3, 0)], &old3, &new),
            vec![(1, 0, 2, 1), (3, 1, 3, 0)]
        );
    }

    #[test]
    fn track_cases() {
        // Insertion of two lines before line 5: lines 5-6 move to 7-8, untouched.
        assert_eq!(track(&[(2, 0, 3, 2)], 5, 6), (false, 7, Some((7, 8))));
        // Insertion between anchored lines touches.
        assert_eq!(track(&[(5, 0, 6, 1)], 5, 6), (true, 5, Some((5, 7))));
        // Deletion of line 1 before: shift up.
        assert_eq!(track(&[(1, 1, 0, 0)], 3, 4), (false, 2, Some((2, 3))));
        // Replacement of line 5 by 1 line, anchor 4-6.
        assert_eq!(track(&[(5, 1, 5, 1)], 4, 6), (true, 4, Some((4, 6))));
        // Replacement of 3 lines by 1 line, clamped to the replacement.
        assert_eq!(track(&[(4, 3, 4, 1)], 4, 6), (true, 4, Some((4, 4))));
        // Every anchored line deleted.
        assert_eq!(track(&[(4, 3, 3, 0)], 4, 6), (true, 4, None));
        // Insertion just after the range does not touch.
        assert_eq!(track(&[(6, 0, 7, 1)], 4, 6), (false, 4, Some((4, 6))));
    }

    fn chain(src: &str, line: usize) -> Vec<(usize, usize, String)> {
        let lines = split_lines(src.as_bytes());
        markdown_chain(&lines, line)
            .into_iter()
            .map(|h| (h.level, h.line, h.text))
            .collect()
    }

    #[test]
    fn markdown_levels_and_fences() {
        let src = "# A\n## B\ntext\n```\n# not\n```\n### C\n## D\nx\n";
        assert_eq!(
            chain(src, 7),
            vec![
                (1, 1, "# A".into()),
                (2, 2, "## B".into()),
                (3, 7, "### C".into())
            ]
        );
        assert_eq!(
            chain(src, 9),
            vec![(1, 1, "# A".into()), (2, 8, "## D".into())]
        );
        assert_eq!(
            chain(src, 5),
            vec![(1, 1, "# A".into()), (2, 2, "## B".into())]
        );
        // Four spaces, seven hashes, missing space: not headings. A bare `#` is.
        assert_eq!(
            chain("    # a\n####### b\n#c\n#\n", 4),
            vec![(1, 4, "#".into())]
        );
        // Tab after hashes; text is stripped.
        assert_eq!(chain("   ##\tT  \n", 1), vec![(2, 1, "##\tT".into())]);
        // Tilde fence closes only on tildes, at least as long, with nothing after.
        let t = "~~~~\n# no\n```\n~~~ \n# no\n~~~~~ x\n# no\n~~~~~\n# yes\n";
        assert_eq!(chain(t, 9), vec![(1, 9, "# yes".into())]);
        // A backtick fence whose info string has a backtick does not open.
        assert_eq!(chain("``` a`b\n# yes\n", 2), vec![(1, 2, "# yes".into())]);
        // Setext is not recognised.
        assert_eq!(chain("Title\n=====\n", 2), vec![]);
    }

    #[test]
    fn markdown_python_whitespace_edge() {
        // U+001F is Python whitespace (str.strip, re \s) but not Rust's char::is_whitespace.
        assert!(py_space('\u{1f}') && !'\u{1f}'.is_whitespace());
        assert!(!py_space('\u{200b}') && !py_space('\u{180e}'));
        // A fence indented by U+001C opens (\s*), so the heading inside is excluded.
        let src = "\u{1c}```\n# hidden\n```\n";
        assert_eq!(chain(src, 2), vec![]);
        // Heading text is stripped of U+001F and U+3000 at the end.
        assert_eq!(chain("# T\u{1f}\u{3000}\n", 1), vec![(1, 1, "# T".into())]);
        // A closing fence followed by U+001E only counts as empty remainder.
        let src2 = "```\n```\u{1e}\n# after\n";
        assert_eq!(chain(src2, 3), vec![(1, 3, "# after".into())]);
        // Invalid UTF-8 is replaced, not refused.
        let lines: Vec<&[u8]> = vec![b"# a\xff"];
        assert_eq!(markdown_chain(&lines, 1)[0].text, "# a\u{fffd}");
    }
}
