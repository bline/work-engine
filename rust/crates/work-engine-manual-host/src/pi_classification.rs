//! Host classification of non-success response heads before relay.
//!
//! The classification table is the route availability view's pinned table
//! (`hosted_route_availability::provisional`), linked into the Host, so the
//! Host and the view classify with the same bytes. A plan names the table it
//! is admitted under by digest (`classification_table_sha256`, bound by the
//! runtime admission). Classification is active only when that digest equals
//! the linked table's; absent or different, heads are handled exactly as
//! before.
//!
//! A head is held only when its status is outside the definitive refusal set
//! (`pi_route::refusal_status`) and the table has a failover-class rule for
//! the route's (provider, funding) and that status. A held head whose body
//! classifies to a failover class is refused like a definitive refusal.
use crate::pi_route::refusal_status;
use crate::pi_stream::{HOLD_MAX_BYTES, HOLD_MAX_WAIT, HeadClass, HeadClassifier, Hold};
use hosted_route_availability::{
    self as availability,
    provisional::{self, Class, Table},
};
use std::time::Instant;

/// Classes a held head is refused on, so the router fails over within the
/// request: quota, rate limit, credit and overload.
pub fn failover_class(class: Class) -> bool {
    matches!(
        class,
        Class::SubscriptionQuotaExhausted
            | Class::RateLimited
            | Class::ApiCreditExhausted
            | Class::Overload
    )
}
/// The table's name of a class.
pub fn class_name(class: Class) -> String {
    serde_json::to_value(class)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .expect("a class serializes to its name")
}
/// Whether `name` is the table's name of a failover class.
pub fn failover_class_name(name: &str) -> bool {
    serde_json::from_value::<Class>(serde_json::Value::from(name)).is_ok_and(failover_class)
}

/// The admitted classification table digest against the linked table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableBinding {
    /// No table admitted: no classification.
    Absent,
    /// The admitted digest is the linked table's: classification active.
    Active,
    /// The admitted digest is another table's: no classification.
    Mismatch,
}
pub fn table_binding(admitted_sha256: Option<&str>) -> TableBinding {
    match admitted_sha256 {
        None => TableBinding::Absent,
        Some(sha) if sha == provisional::table_sha256() => TableBinding::Active,
        Some(_) => TableBinding::Mismatch,
    }
}

/// Classifier of one route's heads under the linked table.
pub struct Classifier {
    provider: &'static str,
    funding: &'static str,
    table: Table,
}
impl Classifier {
    /// The route's classifier when classification is active for it: the
    /// admitted digest is the linked table's and the table covers the
    /// route's endpoint kind.
    pub fn for_route(admitted_sha256: Option<&str>, endpoint: &str) -> Option<Self> {
        if table_binding(admitted_sha256) != TableBinding::Active {
            return None;
        }
        let (provider, funding) = availability::endpoint_provider_funding(endpoint)?;
        Some(Self {
            provider,
            funding,
            table: provisional::table(),
        })
    }
    /// The session hold for this classifier: the classification budget,
    /// never past `window_until`.
    pub fn hold(self, window_until: Option<Instant>) -> Hold {
        Hold {
            classifier: Box::new(self),
            max_wait: HOLD_MAX_WAIT,
            max_bytes: HOLD_MAX_BYTES,
            window_until,
        }
    }
}
impl HeadClassifier for Classifier {
    fn holds(&self, status: u16) -> bool {
        !refusal_status(status)
            && self.table.head_rules.iter().any(|r| {
                r.provider == self.provider
                    && r.funding == self.funding
                    && r.status == status
                    && failover_class(r.class)
            })
    }
    fn classify(&self, status: u16, body: &[u8]) -> HeadClass {
        let class = self
            .table
            .classify_head(self.provider, self.funding, status, body);
        HeadClass {
            class: class_name(class),
            error: provisional::error_name(body),
            failover: failover_class(class),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn linked() -> String {
        provisional::table_sha256()
    }
    fn classifier(endpoint: &str) -> Classifier {
        Classifier::for_route(Some(&linked()), endpoint).unwrap()
    }
    const OVERLOAD: &[u8] =
        br#"{"type":"error","error":{"type":"overloaded_error","message":"x"}}"#;

    #[test]
    fn classification_is_active_only_under_the_linked_table_and_a_covered_endpoint() {
        assert_eq!(table_binding(None), TableBinding::Absent);
        assert_eq!(table_binding(Some(&linked())), TableBinding::Active);
        assert_eq!(table_binding(Some(&"0".repeat(64))), TableBinding::Mismatch);
        assert!(Classifier::for_route(None, "https:anthropic-api").is_none());
        assert!(Classifier::for_route(Some(&"0".repeat(64)), "https:anthropic-api").is_none());
        assert!(Classifier::for_route(Some(&linked()), "loopback:1").is_none());
        for e in [
            "https:anthropic-oauth",
            "https:anthropic-api",
            "https:chatgpt-codex",
            "https:openai-responses",
        ] {
            assert!(Classifier::for_route(Some(&linked()), e).is_some(), "{e}");
        }
    }

    #[test]
    fn only_out_of_set_statuses_with_a_failover_rule_are_held() {
        for e in ["https:anthropic-oauth", "https:anthropic-api"] {
            let c = classifier(e);
            assert!(c.holds(529), "{e}");
            // In the definitive set: refused by status, never held.
            for s in [401, 402, 403, 429] {
                assert!(!c.holds(s), "{e} {s}");
            }
            // No rule: relayed at once.
            for s in [200, 400, 500, 503] {
                assert!(!c.holds(s), "{e} {s}");
            }
        }
        // The provisional table has no OpenAI rule outside the set.
        for e in ["https:chatgpt-codex", "https:openai-responses"] {
            let c = classifier(e);
            for s in [400, 500, 503, 529] {
                assert!(!c.holds(s), "{e} {s}");
            }
        }
    }

    #[test]
    fn classification_is_a_function_of_bytes_and_table() {
        let api = classifier("https:anthropic-api");
        let sub = classifier("https:anthropic-oauth");
        let overload = api.classify(529, OVERLOAD);
        assert_eq!(
            overload,
            HeadClass {
                class: "overload".into(),
                error: Some("overloaded_error".into()),
                failover: true
            }
        );
        // Same bytes, same table: same class, every time and for a fresh
        // classifier.
        for _ in 0..3 {
            assert_eq!(api.classify(529, OVERLOAD), overload);
            assert_eq!(
                classifier("https:anthropic-api").classify(529, OVERLOAD),
                overload
            );
        }
        assert_eq!(sub.classify(529, OVERLOAD), overload);
        // A different error name or status, or a body without one: unmatched.
        let unmatched = api.classify(529, br#"{"error":{"type":"api_error"}}"#);
        assert_eq!(unmatched.class, "unclassified");
        assert!(!unmatched.failover);
        assert!(!api.classify(500, OVERLOAD).failover);
        assert!(!api.classify(529, b"not json").failover);
        assert_eq!(api.classify(529, b"not json").error, None);
        // Another provider's table rows never match.
        assert!(
            !classifier("https:openai-responses")
                .classify(529, OVERLOAD)
                .failover
        );
        // In-set statuses classify as evidence (`authorization_failure` is
        // not a failover class).
        let auth = api.classify(401, br#"{"error":{"type":"authentication_error"}}"#);
        assert_eq!(auth.class, "authorization_failure");
        assert!(!auth.failover);
        assert!(
            api.classify(402, br#"{"error":{"type":"billing_error"}}"#)
                .failover
        );
    }

    #[test]
    fn failover_classes_are_quota_rate_credit_and_overload() {
        for (c, f) in [
            (Class::SubscriptionQuotaExhausted, true),
            (Class::RateLimited, true),
            (Class::ApiCreditExhausted, true),
            (Class::Overload, true),
            (Class::AuthorizationFailure, false),
            (Class::Unclassified, false),
        ] {
            assert_eq!(failover_class(c), f, "{c:?}");
            assert_eq!(failover_class_name(&class_name(c)), f, "{c:?}");
        }
        assert!(!failover_class_name("credit"));
    }
}
