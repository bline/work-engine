use serde_json::json;
use work_engine_manual_host::pi_route::{Decision, Disposition, Policy, Route, Router};

fn route(id: &str, payer: &str) -> Route {
    Route {
        id: id.into(),
        connection: format!("conn-{id}"),
        model: "gpt-6-luna".into(),
        payer: payer.into(),
        authorized: true,
        revoked: false,
        max_attempts: 8,
    }
}
fn policy() -> Policy {
    Policy {
        model: "gpt-6-luna".into(),
        routes: vec![route("primary", "subscription"), route("secondary", "api")],
        max_attempts_per_request: 2,
        allowance_units: 10,
        reserve_units: 2,
        expires_ms: 1_000,
    }
}
fn refused() -> Disposition {
    Disposition::Refused {
        status: 429,
        code: "insufficient_quota".into(),
    }
}
fn attempt(d: Decision) -> (usize, u64) {
    match d {
        Decision::Attempt { route, attempt } => (route, attempt),
        u => panic!("{u:?}"),
    }
}

#[test]
fn definitive_refusal_fails_over_with_provenance_and_stays_on_secondary() {
    let mut r = Router::new(policy());
    let (p, a1) = attempt(r.next(1, "gpt-6-luna", 0));
    assert_eq!(p, 0);
    r.settle(a1, refused(), json!(null)).unwrap();
    assert_eq!(
        r.reserved_units(),
        0,
        "definitive refusal releases its reservation"
    );
    let (s, a2) = attempt(r.next(1, "gpt-6-luna", 0));
    assert_eq!(s, 1);
    let rec = r.last().unwrap();
    assert_eq!(rec.predecessor, Some(a1));
    assert_eq!(rec.reason, "failover-after-429-insufficient_quota");
    assert_eq!(
        (rec.payer.as_str(), rec.model.as_str()),
        ("api", "gpt-6-luna")
    );
    assert_eq!(rec.policy, r.digest());
    r.settle(a2, Disposition::Completed, json!({"totalTokens":2}))
        .unwrap();
    assert_eq!(
        r.reserved_units(),
        2,
        "completed attempt keeps a conservative obligation"
    );
    // Next request: stable routing, no needless return to the refused primary.
    let (s2, _) = attempt(r.next(2, "gpt-6-luna", 0));
    assert_eq!(s2, 1);
    assert_eq!(r.last().unwrap().reason, "preferred");
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("request_completed")
    );
}

#[test]
fn uncertain_or_unsettled_predecessor_never_switches() {
    let mut r = Router::new(policy());
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 0));
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("attempt_unsettled")
    );
    r.settle(
        a,
        Disposition::Uncertain {
            reason: "provider-failed-after-output:insufficient_quota".into(),
        },
        json!(null),
    )
    .unwrap();
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("uncertain_predecessor")
    );
    assert_eq!(r.reserved_units(), 2);
    assert_eq!(
        r.settle(a, Disposition::Completed, json!(null)),
        Err("route_attempt_settled")
    );
}

#[test]
fn refuses_unauthorized_revoked_incompatible_stale_and_over_budget() {
    // Secondary listed but not authorized: subscription refusal grants nothing.
    let mut p = policy();
    p.routes[1].authorized = false;
    let mut r = Router::new(p);
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 0));
    r.settle(a, refused(), json!(null)).unwrap();
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("no_permitted_route")
    );

    // Revoked secondary.
    let mut p = policy();
    p.routes[1].revoked = true;
    let mut r = Router::new(p);
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 0));
    r.settle(a, refused(), json!(null)).unwrap();
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("route_revoked")
    );
    // Revoked primary never grants the secondary without a refusal.
    let mut p = policy();
    p.routes[0].revoked = true;
    let mut r = Router::new(p);
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("route_revoked")
    );

    // Secondary serves another model: billing fallback never substitutes models.
    let mut p = policy();
    p.routes[1].model = "other-model".into();
    let mut r = Router::new(p);
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 0));
    r.settle(a, refused(), json!(null)).unwrap();
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("no_permitted_route")
    );
    assert_eq!(
        r.next(2, "other-model", 0),
        Decision::Unavailable("model_incompatible")
    );

    // Stale policy.
    let mut r = Router::new(policy());
    assert_eq!(
        r.next(1, "gpt-6-luna", 1_000),
        Decision::Unavailable("policy_stale")
    );

    // Allowance: five conservative reservations exhaust 10 units.
    let mut r = Router::new(policy());
    for req in 1..=5 {
        let (_, a) = attempt(r.next(req, "gpt-6-luna", 0));
        r.settle(a, Disposition::Completed, json!(null)).unwrap();
    }
    assert_eq!(
        r.next(6, "gpt-6-luna", 0),
        Decision::Unavailable("allowance_exhausted")
    );
}

#[test]
fn per_request_and_per_route_limits() {
    let mut p = policy();
    p.max_attempts_per_request = 1;
    let mut r = Router::new(p);
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 0));
    r.settle(a, refused(), json!(null)).unwrap();
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("attempt_limit")
    );

    let mut p = policy();
    p.routes[0].max_attempts = 1;
    p.routes.truncate(1);
    let mut r = Router::new(p);
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 0));
    r.settle(a, Disposition::Completed, json!(null)).unwrap();
    assert_eq!(
        r.next(2, "gpt-6-luna", 0),
        Decision::Unavailable("route_limit")
    );
    assert_eq!(r.evidence()["routeState"][0]["attempts"], 1);
}

#[test]
fn exhausted_route_limit_does_not_switch_without_refusal() {
    let mut p = policy();
    p.routes[0].max_attempts = 1;
    let mut r = Router::new(p);
    let (route0, a) = attempt(r.next(1, "gpt-6-luna", 0));
    assert_eq!(route0, 0);
    r.settle(a, Disposition::Completed, json!(null)).unwrap();
    assert_eq!(
        r.next(2, "gpt-6-luna", 0),
        Decision::Unavailable("route_limit")
    );
    assert!(r.attempts().iter().all(|a| a.route == "primary"));
}

#[test]
fn allowance_arithmetic_fails_closed_at_extremes() {
    let mut p = policy();
    p.allowance_units = u64::MAX;
    p.reserve_units = u64::MAX;
    let mut r = Router::new(p);
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 0));
    r.settle(a, Disposition::Completed, json!(null)).unwrap();
    assert_eq!(r.reserved_units(), u64::MAX);
    // reserved + reserve would overflow: refuse, never wrap or panic.
    assert_eq!(
        r.next(2, "gpt-6-luna", 0),
        Decision::Unavailable("allowance_exhausted")
    );
    assert_eq!(r.reserved_units(), u64::MAX);
}

#[test]
fn fresh_router_recovers_mid_switch_without_retry_or_lost_attempts() {
    let mut r = Router::new(policy());
    let (_, a1) = attempt(r.next(1, "gpt-6-luna", 0));
    let intent1 = r.last().unwrap().clone();
    r.settle(a1, refused(), json!(null)).unwrap();
    let settle1 = r.attempts()[0].clone();
    let (_, _a2) = attempt(r.next(1, "gpt-6-luna", 0));
    let intent2 = r.last().unwrap().clone();
    // Crash: attempt 2's intent is durable, its settlement is not.
    let mut f = Router::recover(
        policy(),
        &[intent2.clone(), intent1.clone()],
        &[settle1.clone()],
    )
    .unwrap();
    assert_eq!(f.attempts().len(), 2);
    assert_eq!(f.attempts()[0].disposition, Some(refused()));
    assert_eq!(
        f.attempts()[1].disposition,
        Some(Disposition::Uncertain {
            reason: "restart-unsettled".into()
        })
    );
    assert_eq!(
        f.reserved_units(),
        2,
        "unsettled attempt keeps its reservation"
    );
    assert_eq!(
        f.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("uncertain_predecessor")
    );
    // Stable routing survives restart: a new request goes to the secondary.
    assert_eq!(attempt(f.next(2, "gpt-6-luna", 0)).0, 1);

    // Another policy, gaps, or settlement without intent are refused.
    let mut other = policy();
    other.allowance_units = 11;
    assert_eq!(
        Router::recover(other, &[intent1.clone()], &[]).err(),
        Some("route_policy_mismatch")
    );
    assert_eq!(
        Router::recover(policy(), &[intent2.clone()], &[]).err(),
        Some("route_intent_sequence")
    );
    assert_eq!(
        Router::recover(policy(), &[], &[settle1]).err(),
        Some("route_settle_without_intent")
    );
}
