//! Deterministic implementation-review v1 result validation. The original
//! validator's no-authority verdict/finding shape is preserved here; host
//! provenance and claims admission remain outside this pure module.

use std::collections::{BTreeMap, BTreeSet};

use crate::codec::{JsString, JsValue, digest, exact_fields, field, string_is};
use crate::identity::Revision;
use crate::{EpisodeError, EpisodeResult, ErrorKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    AcceptableAsIs,
    RemediationRequired,
    Incomplete,
}

#[derive(Clone, Debug)]
pub struct ReviewResult {
    pub value: JsValue,
    pub verdict: Verdict,
    pub findings: Vec<JsValue>,
}

impl ReviewResult {
    pub fn parse(value: JsValue) -> EpisodeResult<Self> {
        Self::parse_inner(value).map_err(|error| error.with_kind(ErrorKind::ResultContract))
    }

    fn parse_inner(value: JsValue) -> EpisodeResult<Self> {
        exact_fields(
            &value,
            &[
                "schemaVersion",
                "subject",
                "verdict",
                "findings",
                "decisiveEvidence",
                "limitations",
            ],
            "implementation review result",
        )?;
        if field(&value, "schemaVersion")? != &JsValue::Number(1.0) {
            return Err(EpisodeError::new(
                "implementation review schema version is invalid",
            ));
        }
        validate_subject(field(&value, "subject")?)?;
        let verdict = match field(&value, "verdict")? {
            item if string_is(item, "acceptable_as_is") => Verdict::AcceptableAsIs,
            item if string_is(item, "remediation_required") => Verdict::RemediationRequired,
            item if string_is(item, "incomplete") => Verdict::Incomplete,
            _ => {
                return Err(EpisodeError::new(
                    "implementation review verdict is invalid",
                ));
            }
        };
        let findings = field(&value, "findings")?.as_array()?.to_vec();
        let mut ids = BTreeSet::new();
        let mut pending = false;
        for finding in &findings {
            validate_finding(finding)?;
            let id = field(finding, "id")?.as_text()?;
            if !ids.insert(id) {
                return Err(EpisodeError::new(
                    "implementation review finding IDs must be unique",
                ));
            }
            let status = field(finding, "status")?;
            pending |= string_is(status, "open")
                || string_is(status, "remediation_presented")
                || string_is(status, "unresolved");
        }
        let decisive = field(&value, "decisiveEvidence")?.as_array()?;
        for item in decisive {
            validate_evidence(item)?;
        }
        let limitations = field(&value, "limitations")?.as_array()?;
        if limitations.iter().any(|item| {
            item.nonempty_text("implementation review limitation")
                .is_err()
        }) {
            return Err(EpisodeError::new(
                "implementation review limitations are invalid",
            ));
        }
        match verdict {
            Verdict::AcceptableAsIs
                if decisive.is_empty() || !limitations.is_empty() || pending =>
            {
                return Err(EpisodeError::new(
                    "acceptable_as_is requires decisive evidence and no limitation or unresolved finding",
                ));
            }
            Verdict::RemediationRequired if !pending => {
                return Err(EpisodeError::new(
                    "remediation_required requires an unresolved evidence-bearing finding",
                ));
            }
            Verdict::Incomplete if limitations.is_empty() => {
                return Err(EpisodeError::new(
                    "incomplete requires explicit limitations",
                ));
            }
            _ => {}
        }
        Ok(Self {
            value,
            verdict,
            findings,
        })
    }

    pub fn subject_digest(&self) -> EpisodeResult<Revision> {
        Ok(Revision(digest(field(&self.value, "subject")?)))
    }
}

pub fn validate_subject(value: &JsValue) -> EpisodeResult<()> {
    exact_fields(
        value,
        &["commit", "tree", "patchIdentity"],
        "review subject",
    )?;
    for key in ["commit", "tree", "patchIdentity"] {
        field(value, key)?.nonempty_text(&format!("review subject.{key}"))?;
    }
    Ok(())
}

fn validate_evidence(value: &JsValue) -> EpisodeResult<()> {
    exact_fields(
        value,
        &["path", "startLine", "endLine", "sha256"],
        "review evidence",
    )?;
    field(value, "path")?.nonempty_text("review evidence.path")?;
    let start = field(value, "startLine")?.as_safe_positive_integer("review evidence.startLine")?;
    let end = field(value, "endLine")?.as_safe_positive_integer("review evidence.endLine")?;
    if end < start {
        return Err(EpisodeError::new("review evidence line bounds are invalid"));
    }
    Revision::parse(field(value, "sha256")?, "review evidence.sha256")?;
    Ok(())
}

fn validate_finding(value: &JsValue) -> EpisodeResult<()> {
    exact_fields(
        value,
        &[
            "id",
            "severity",
            "title",
            "evidence",
            "observed",
            "violatedExpectation",
            "consequence",
            "basis",
            "confidence",
            "recommendedRemediation",
            "status",
            "remediationEvidence",
        ],
        "review finding",
    )?;
    for key in [
        "id",
        "title",
        "observed",
        "violatedExpectation",
        "consequence",
        "recommendedRemediation",
    ] {
        field(value, key)?.nonempty_text(&format!("review finding.{key}"))?;
    }
    one_of(
        field(value, "severity")?,
        &["blocker", "high", "medium", "low", "info"],
        "severity",
    )?;
    one_of(field(value, "basis")?, &["reproduced", "inferred"], "basis")?;
    one_of(
        field(value, "confidence")?,
        &["high", "medium", "low"],
        "confidence",
    )?;
    one_of(
        field(value, "status")?,
        &[
            "open",
            "remediation_presented",
            "verified_resolved",
            "withdrawn",
            "unresolved",
        ],
        "status",
    )?;
    let evidence = field(value, "evidence")?.as_array()?;
    if evidence.is_empty() {
        return Err(EpisodeError::new("review finding requires evidence"));
    }
    for item in evidence {
        validate_evidence(item)?;
    }
    let remediation = field(value, "remediationEvidence")?.as_array()?;
    for item in remediation {
        validate_evidence(item)?;
    }
    let status = field(value, "status")?;
    if (string_is(status, "remediation_presented") || string_is(status, "verified_resolved"))
        && remediation.is_empty()
    {
        return Err(EpisodeError::new(
            "review finding remediation status requires remediation evidence",
        ));
    }
    Ok(())
}

fn one_of(value: &JsValue, allowed: &[&str], label: &str) -> EpisodeResult<()> {
    if allowed.iter().any(|item| string_is(value, item)) {
        Ok(())
    } else {
        Err(EpisodeError::new(format!(
            "review finding.{label} is invalid"
        )))
    }
}

pub fn preserve_finding_lineage(previous: &[JsValue], next: &[JsValue]) -> EpisodeResult<()> {
    preserve_finding_lineage_inner(previous, next)
        .map_err(|error| error.with_kind(ErrorKind::ResultContract))
}

fn preserve_finding_lineage_inner(previous: &[JsValue], next: &[JsValue]) -> EpisodeResult<()> {
    let mut current: BTreeMap<&JsString, &JsValue> = BTreeMap::new();
    for finding in next {
        current.insert(field(finding, "id")?.as_text()?, finding);
    }
    for prior in previous {
        let id = field(prior, "id")?.as_text()?;
        let Some(finding) = current.get(id) else {
            return Err(EpisodeError::new(
                "review episode findings cannot be deleted",
            ));
        };
        for key in [
            "id",
            "severity",
            "title",
            "evidence",
            "observed",
            "violatedExpectation",
            "consequence",
            "basis",
            "confidence",
            "recommendedRemediation",
        ] {
            if digest(field(prior, key)?) != digest(field(finding, key)?) {
                return Err(EpisodeError::new(
                    "review episode finding attribution or evidence cannot be rewritten",
                ));
            }
        }
    }
    Ok(())
}
