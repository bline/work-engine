//! Total required-check accounting (PLAN-R4). The required identities come from the validated
//! specification. Every identity starts `Unperformed`; a check can only be marked `Passed` by
//! running its evaluator to completion with no finding. [`Ledger::conclude`] is the only way to
//! obtain [`Qualified`], and it requires the passed identity set to equal the required set. An
//! empty finding list, an early return or a skipped evaluator cannot create success.
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Why a check did not pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Required evidence absent.
    Missing,
    /// Present but not decodable as the required representation (including duplicate keys,
    /// wrong types, unknown or missing fields, malformed timestamps or protocol lines).
    Malformed,
    /// A domain owner (episode core, preparation/fixture consumer, delivery adapter, compiler)
    /// refused the evidence.
    OwnerRejected,
    /// Decoded and owner-valid, but not corresponding to the expected identity or artifact.
    Mismatch,
    /// A process outcome, disposition or test result that cannot qualify.
    Failed,
    /// The evaluator itself panicked; the check is not complete.
    EvaluatorPanic,
}

impl Class {
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Missing => "missing",
            Class::Malformed => "malformed",
            Class::OwnerRejected => "owner_rejected",
            Class::Mismatch => "mismatch",
            Class::Failed => "failed",
            Class::EvaluatorPanic => "evaluator_panic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub check: String,
    pub artifact: String,
    pub field: String,
    pub class: Class,
    pub expected: String,
    pub observed: String,
    pub detail: String,
}

impl Finding {
    pub fn to_json(&self) -> Value {
        json!({"check": self.check, "artifact": self.artifact, "field": self.field, "class": self.class.as_str(),
            "expected": self.expected, "observed": self.observed, "detail": self.detail})
    }
}

/// Findings produced by one evaluator. The ledger stamps the check identity.
#[derive(Debug, Default)]
pub struct Defects(pub Vec<Finding>);

impl Defects {
    pub fn add(
        &mut self,
        artifact: &str,
        field: &str,
        class: Class,
        expected: impl ToString,
        observed: impl ToString,
        detail: impl Into<String>,
    ) {
        self.0.push(Finding {
            check: String::new(),
            artifact: artifact.to_owned(),
            field: field.to_owned(),
            class,
            expected: expected.to_string(),
            observed: observed.to_string(),
            detail: detail.into(),
        });
    }
    pub fn malformed(&mut self, artifact: &str, detail: impl Into<String>) {
        self.add(artifact, "", Class::Malformed, "", "", detail);
    }
    pub fn missing(&mut self, artifact: &str, detail: impl Into<String>) {
        self.add(artifact, "", Class::Missing, "present", "absent", detail);
    }
    pub fn owner(&mut self, artifact: &str, owner: &str, detail: impl Into<String>) {
        self.add(
            artifact,
            "",
            Class::OwnerRejected,
            format!("accepted by {owner}"),
            "refused",
            detail,
        );
    }
    pub fn mismatch(
        &mut self,
        artifact: &str,
        field: &str,
        expected: impl ToString,
        observed: impl ToString,
        detail: impl Into<String>,
    ) {
        self.add(artifact, field, Class::Mismatch, expected, observed, detail);
    }
    pub fn failed(
        &mut self,
        artifact: &str,
        field: &str,
        expected: impl ToString,
        observed: impl ToString,
        detail: impl Into<String>,
    ) {
        self.add(artifact, field, Class::Failed, expected, observed, detail);
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Converts a decode error into a malformed finding and stops the evaluator.
    pub fn take<T>(&mut self, artifact: &str, r: Result<T, String>) -> Option<T> {
        match r {
            Ok(v) => Some(v),
            Err(e) => {
                self.malformed(artifact, e);
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Unperformed,
    Passed,
    Failed(Vec<Finding>),
    /// A dependency did not pass; this check was not performed and remains incomplete.
    Blocked(Vec<String>),
}

#[derive(Debug)]
pub struct Ledger {
    order: Vec<String>,
    deps: BTreeMap<String, Vec<String>>,
    status: BTreeMap<String, Status>,
}

/// Proof that every required check identity completed successfully. Not constructible outside
/// [`Ledger::conclude`].
#[derive(Debug)]
pub struct Qualified {
    checks: BTreeSet<String>,
}

impl Qualified {
    pub fn checks(&self) -> &BTreeSet<String> {
        &self.checks
    }
}

impl Ledger {
    /// `required` is the validated specification's ordered list of (identity, dependencies).
    pub fn new(required: &[(String, Vec<String>)]) -> Self {
        let mut l = Ledger {
            order: Vec::new(),
            deps: BTreeMap::new(),
            status: BTreeMap::new(),
        };
        for (id, deps) in required {
            l.order.push(id.clone());
            l.deps.insert(id.clone(), deps.clone());
            l.status.insert(id.clone(), Status::Unperformed);
        }
        l
    }

    pub fn passed(&self, id: &str) -> bool {
        self.status.get(id) == Some(&Status::Passed)
    }

    pub fn status(&self, id: &str) -> Option<&Status> {
        self.status.get(id)
    }

    /// Runs one required check. Unknown identities are refused (a programming error that must
    /// not silently pass). Dependencies that did not pass block the check. The evaluator's value
    /// is returned only when it completed with no finding.
    pub fn run<T>(&mut self, id: &str, eval: impl FnOnce(&mut Defects) -> Option<T>) -> Option<T> {
        let Some(deps) = self.deps.get(id).cloned() else {
            panic!("check `{id}` is not in the validated specification");
        };
        if self.status.get(id) != Some(&Status::Unperformed) {
            panic!("check `{id}` evaluated twice");
        }
        let unmet: Vec<String> = deps.into_iter().filter(|d| !self.passed(d)).collect();
        if !unmet.is_empty() {
            self.status.insert(id.to_owned(), Status::Blocked(unmet));
            return None;
        }
        let mut defects = Defects::default();
        let outcome = catch_unwind(AssertUnwindSafe(|| eval(&mut defects)));
        let value = match outcome {
            Ok(v) => v,
            Err(p) => {
                let text = p
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| p.downcast_ref::<String>().cloned())
                    .unwrap_or_default();
                defects.add("", "", Class::EvaluatorPanic, "completion", "panic", text);
                None
            }
        };
        match (value, defects.is_empty()) {
            (Some(v), true) => {
                self.status.insert(id.to_owned(), Status::Passed);
                Some(v)
            }
            (_, empty) => {
                let mut findings = defects.0;
                if empty {
                    findings.push(Finding {
                        check: String::new(),
                        artifact: String::new(),
                        field: String::new(),
                        class: Class::Failed,
                        expected: "completed evaluation".into(),
                        observed: "evaluator returned no result".into(),
                        detail: "an evaluator that stops early does not complete its check".into(),
                    });
                }
                for f in &mut findings {
                    f.check = id.to_owned();
                }
                self.status.insert(id.to_owned(), Status::Failed(findings));
                None
            }
        }
    }

    /// Identities that are not `Passed` (failed, blocked or never performed).
    pub fn incomplete(&self) -> Vec<String> {
        self.order
            .iter()
            .filter(|id| !self.passed(id))
            .cloned()
            .collect()
    }

    pub fn findings(&self) -> Vec<&Finding> {
        self.order
            .iter()
            .filter_map(|id| match &self.status[id] {
                Status::Failed(f) => Some(f.iter()),
                _ => None,
            })
            .flatten()
            .collect()
    }

    pub fn conclude(&self) -> Result<Qualified, Vec<String>> {
        let required: BTreeSet<String> = self.order.iter().cloned().collect();
        let passed: BTreeSet<String> = self
            .status
            .iter()
            .filter(|(_, s)| **s == Status::Passed)
            .map(|(k, _)| k.clone())
            .collect();
        if required.is_empty() || passed != required {
            return Err(self.incomplete());
        }
        Ok(Qualified { checks: passed })
    }

    pub fn to_json(&self, mode: &str, facts: &serde_json::Map<String, Value>) -> Value {
        let checks: Vec<Value> = self
            .order
            .iter()
            .map(|id| match &self.status[id] {
                Status::Unperformed => json!({"id": id, "status": "unperformed"}),
                Status::Passed => json!({"id": id, "status": "passed"}),
                Status::Failed(f) => json!({"id": id, "status": "failed", "findings": f.iter().map(Finding::to_json).collect::<Vec<_>>()}),
                Status::Blocked(d) => json!({"id": id, "status": "blocked", "blocked_by": d}),
            })
            .collect();
        let qualified = self.conclude().is_ok();
        json!({
            "schema": "org-evidence-gate-report/v2",
            "mode": mode,
            "qualified": qualified,
            "required_checks": self.order.len(),
            "passed_checks": self.order.iter().filter(|id| self.passed(id)).count(),
            "incomplete": self.incomplete(),
            "checks": checks,
            "facts": facts,
        })
    }
}
