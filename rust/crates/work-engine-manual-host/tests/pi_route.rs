use serde_json::json;
use work_engine_manual_host::pi_route::{
    Decision, Disposition, FailoverBinding, Policy, ResetObservation, Route, RouteDirective, Router,
};

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
        None,
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
        Router::recover(other, None, &[intent1.clone()], &[]).err(),
        Some("route_policy_mismatch")
    );
    assert_eq!(
        Router::recover(policy(), None, &[intent2.clone()], &[]).err(),
        Some("route_intent_sequence")
    );
    assert_eq!(
        Router::recover(policy(), None, &[], &[settle1]).err(),
        Some("route_settle_without_intent")
    );
}

#[test]
fn not_dispatched_releases_and_ends_the_request_without_switching() {
    let mut r = Router::new(policy());
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 0));
    r.settle(a, Disposition::NotDispatched, json!(null))
        .unwrap();
    assert_eq!(r.reserved_units(), 0);
    assert_eq!(
        r.next(1, "gpt-6-luna", 0),
        Decision::Unavailable("request_cancelled")
    );
    // The route stays usable for later requests.
    assert_eq!(attempt(r.next(2, "gpt-6-luna", 0)).0, 0);
}

fn binding() -> FailoverBinding {
    FailoverBinding {
        job_id: "job-1".into(),
        table_digest: "table".into(),
        policy_version: "v1".into(),
    }
}
fn directive(action: &str, route: &str) -> RouteDirective {
    RouteDirective {
        job_id: "job-1".into(),
        table_digest: "table".into(),
        policy_version: "v1".into(),
        state_observation_sha256: "obs".into(),
        action: action.into(),
        route: Some(route.into()),
        decided_ms: 500,
        reset_observation: None,
    }
}
fn reset(request: u64, attempt: u64, at: u64) -> ResetObservation {
    ResetObservation {
        job_id: "job-1".into(),
        request,
        attempt,
        available_again_at: at,
    }
}

#[test]
fn a_directive_selects_only_an_admitted_route_under_its_binding() {
    let mut r = Router::new(policy());
    assert_eq!(
        r.next_with(1, "gpt-6-luna", 500, &directive("skip", "secondary")),
        Decision::Unavailable("decision_without_binding")
    );
    let mut r = Router::with_failover(policy(), binding());
    for (field, d) in [
        (
            "job",
            RouteDirective {
                job_id: "job-2".into(),
                ..directive("skip", "secondary")
            },
        ),
        (
            "table",
            RouteDirective {
                table_digest: "other".into(),
                ..directive("skip", "secondary")
            },
        ),
        (
            "policy",
            RouteDirective {
                policy_version: "v2".into(),
                ..directive("skip", "secondary")
            },
        ),
    ] {
        assert_eq!(
            r.next_with(1, "gpt-6-luna", 500, &d),
            Decision::Unavailable("decision_binding_mismatch"),
            "{field}"
        );
    }
    assert_eq!(
        r.next_with(1, "gpt-6-luna", 500, &directive("skip", "fabricated")),
        Decision::Unavailable("decision_route_not_admitted")
    );
    let mut p = policy();
    p.routes[1].authorized = false;
    let mut unauthorized = Router::with_failover(p, binding());
    assert_eq!(
        unauthorized.next_with(1, "gpt-6-luna", 500, &directive("skip", "secondary")),
        Decision::Unavailable("decision_route_not_admitted")
    );
    // A proactive skip: secondary without a refusal of primary; nothing is
    // recorded as refused.
    let (s, a) = attempt(r.next_with(1, "gpt-6-luna", 500, &directive("skip", "secondary")));
    assert_eq!(s, 1);
    let rec = r.last().unwrap();
    assert_eq!(rec.reason, "failover-skip");
    assert_eq!(rec.decision, Some(directive("skip", "secondary")));
    r.settle(a, Disposition::Completed, json!(null)).unwrap();
    assert_eq!(r.snapshot(2).refused, vec![None, None]);
    assert_eq!(r.snapshot(2).default_route, Some(0));
    // Without a decision the router's default applies.
    assert_eq!(attempt(r.next(2, "gpt-6-luna", 500)).0, 0);
    // Pause admits nothing.
    let mut r = Router::with_failover(policy(), binding());
    let pause = RouteDirective {
        route: None,
        ..directive("pause", "primary")
    };
    assert_eq!(
        r.next_with(1, "gpt-6-luna", 500, &pause),
        Decision::Unavailable("failover_pause")
    );
    assert!(r.attempts().is_empty());
}

#[test]
fn a_refused_route_needs_a_reset_observation_of_this_job_after_the_refusal() {
    let mut p = policy();
    p.max_attempts_per_request = 4;
    let mut r = Router::with_failover(p, binding());
    let (_, a1) = attempt(r.next(1, "gpt-6-luna", 500));
    r.settle(a1, refused(), json!(null)).unwrap();
    assert_eq!(r.snapshot(1).refused, vec![Some((1, 1)), None]);
    let back = |o: Option<ResetObservation>| RouteDirective {
        reset_observation: o,
        ..directive("fail_back", "primary")
    };
    let other_job = ResetObservation {
        job_id: "job-2".into(),
        ..reset(1, 1, 400)
    };
    for (case, o) in [
        ("none", None),
        ("other job", Some(other_job)),
        ("before the refusal", Some(reset(0, 9, 400))),
        ("reset not passed", Some(reset(1, 1, 501))),
    ] {
        assert_eq!(
            r.next_with(1, "gpt-6-luna", 500, &back(o)),
            Decision::Unavailable("refused_route_without_reset_observation"),
            "{case}"
        );
    }
    // Reset passed exactly at now, evidence later than the refusal.
    let o = reset(1, 9, 500);
    let (p0, a2) = attempt(r.next_with(1, "gpt-6-luna", 500, &back(Some(o.clone()))));
    assert_eq!(p0, 0);
    assert_eq!(r.last().unwrap().reason, "failover-fail_back");
    assert_eq!(
        r.snapshot(1).verified,
        vec![("primary".to_string(), o.clone())]
    );
    // Refused again: the same observation is spent for this request.
    r.settle(a2, refused(), json!(null)).unwrap();
    assert_eq!(r.snapshot(1).refused[0], Some((1, 2)));
    assert_eq!(
        r.next_with(1, "gpt-6-luna", 500, &back(Some(o.clone()))),
        Decision::Unavailable("verification_spent")
    );
    // Falling to the secondary still works; another request may verify once.
    let (s, a3) = attempt(r.next(1, "gpt-6-luna", 500));
    assert_eq!(s, 1);
    r.settle(a3, Disposition::Completed, json!(null)).unwrap();
    assert!(r.snapshot(2).verified.is_empty());
    assert_eq!(
        attempt(r.next_with(2, "gpt-6-luna", 500, &back(Some(o)))).0,
        0
    );
}

#[test]
fn a_directive_never_resends_an_uncertain_attempt_and_keeps_every_limit() {
    let mut r = Router::with_failover(policy(), binding());
    let (_, a) = attempt(r.next_with(1, "gpt-6-luna", 500, &directive("send", "primary")));
    assert_eq!(
        r.next_with(1, "gpt-6-luna", 500, &directive("skip", "secondary")),
        Decision::Unavailable("attempt_unsettled")
    );
    r.settle(
        a,
        Disposition::Uncertain {
            reason: "200:failed".into(),
        },
        json!(null),
    )
    .unwrap();
    for d in [directive("skip", "secondary"), directive("send", "primary")] {
        assert_eq!(
            r.next_with(1, "gpt-6-luna", 500, &d),
            Decision::Unavailable("uncertain_predecessor")
        );
    }
    // Per-request limit.
    let mut p = policy();
    p.max_attempts_per_request = 1;
    let mut r = Router::with_failover(p, binding());
    let (_, a) = attempt(r.next(1, "gpt-6-luna", 500));
    r.settle(a, refused(), json!(null)).unwrap();
    assert_eq!(
        r.next_with(1, "gpt-6-luna", 500, &directive("skip", "secondary")),
        Decision::Unavailable("attempt_limit")
    );
    // Per-route limit and revocation.
    let mut p = policy();
    p.routes[1].max_attempts = 1;
    let mut r = Router::with_failover(p, binding());
    let (_, a) = attempt(r.next_with(1, "gpt-6-luna", 500, &directive("skip", "secondary")));
    r.settle(a, Disposition::Completed, json!(null)).unwrap();
    assert_eq!(
        r.next_with(2, "gpt-6-luna", 500, &directive("skip", "secondary")),
        Decision::Unavailable("route_limit")
    );
    let mut p = policy();
    p.routes[1].revoked = true;
    let mut r = Router::with_failover(p, binding());
    assert_eq!(
        r.next_with(1, "gpt-6-luna", 500, &directive("skip", "secondary")),
        Decision::Unavailable("route_revoked")
    );
}

#[test]
fn recover_replays_decisions_and_refuses_a_route_change_without_one() {
    let mut p = policy();
    p.max_attempts_per_request = 3;
    let mut r = Router::with_failover(p.clone(), binding());
    // Request 1: proactive skip to the secondary, completed.
    let (_, a1) = attempt(r.next_with(1, "gpt-6-luna", 500, &directive("skip", "secondary")));
    r.settle(a1, Disposition::Completed, json!(null)).unwrap();
    // Request 2: primary refused, then a fail-back on its reset, completed.
    let (_, a2) = attempt(r.next(2, "gpt-6-luna", 500));
    r.settle(a2, refused(), json!(null)).unwrap();
    let back = RouteDirective {
        reset_observation: Some(reset(2, 1, 400)),
        ..directive("fail_back", "primary")
    };
    let (_, a3) = attempt(r.next_with(2, "gpt-6-luna", 500, &back));
    r.settle(a3, Disposition::Completed, json!(null)).unwrap();
    let log: Vec<_> = r.attempts().to_vec();
    let intents: Vec<_> = log
        .iter()
        .map(|a| {
            let mut a = a.clone();
            a.disposition = None;
            a.usage = json!(null);
            a
        })
        .collect();
    let f = Router::recover(p.clone(), Some(binding()), &intents, &log).unwrap();
    assert_eq!(
        serde_json::to_value(f.attempts()).unwrap(),
        serde_json::to_value(r.attempts()).unwrap()
    );
    assert_eq!(f.snapshot(3), r.snapshot(3));
    assert_eq!(f.reserved_units(), r.reserved_units());
    // A route change without its decision is refused.
    let mut stripped = intents.clone();
    stripped[0].decision = None;
    assert_eq!(
        Router::recover(p.clone(), Some(binding()), &stripped, &log).err(),
        Some("route_change_without_decision")
    );
    // A fail-back without its decision is a change without a decision too.
    let mut stripped = intents.clone();
    stripped[2].decision = None;
    assert_eq!(
        Router::recover(p.clone(), Some(binding()), &stripped, &log).err(),
        Some("route_change_without_decision")
    );
    // Decisions need the binding they were made under.
    assert_eq!(
        Router::recover(p.clone(), None, &intents, &log).err(),
        Some("route_decision_without_binding")
    );
    let other = FailoverBinding {
        policy_version: "v2".into(),
        ..binding()
    };
    assert_eq!(
        Router::recover(p.clone(), Some(other), &intents, &log).err(),
        Some("route_decision_not_reproduced")
    );
    // A decision naming another route than the intent is not reproduced.
    let mut moved = intents.clone();
    moved[0].decision = Some(directive("send", "primary"));
    assert_eq!(
        Router::recover(p, Some(binding()), &moved, &log).err(),
        Some("route_decision_not_reproduced")
    );
}
