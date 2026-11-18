//! Outcome 3: task rendering.

mod common;

use work_engine_review_hosted::revision::{REVISION_REQUIREMENTS, prepare_world};
use work_engine_review_hosted::task::{
    HOST_SUFFIX_BYTES, Limits, MAX_PROMPT_BYTES, TEMPLATE, TEMPLATE_SHA256, render,
};
use work_engine_review_hosted::{Refusal, sha256_hex};

const LIMITS: Limits = Limits {
    max_requests: 8,
    max_reads: 16,
};

#[test]
fn template_is_digest_pinned() {
    assert_eq!(sha256_hex(TEMPLATE.as_bytes()), TEMPLATE_SHA256);
    assert!(TEMPLATE.starts_with("Template work-engine.review-hosted-task.v1."));
}

#[test]
fn task_is_template_then_contract_verbatim_and_fits_the_prompt_bound() {
    let subject = common::frozen_subject();
    let frozen = prepare_world(REVISION_REQUIREMENTS, &subject, "job-1").unwrap();
    let task = render(frozen.contract(), &subject, LIMITS).unwrap();
    assert!(
        task.ends_with(frozen.contract()),
        "the contract is appended verbatim"
    );
    let head = &task[..task.len() - frozen.contract().len()];
    assert!(head.ends_with("--- BEGIN CONTRACT ---\n"));
    assert!(head.contains(&serde_json::to_string(&subject).unwrap()));
    assert!(head.contains("at most 8 model requests and 16 snapshot reads"));
    assert!(head.contains("no code fence"));
    assert!(head.contains("exactly as snapshot_list reported it"));
    assert!(!head.contains("{{"));
    assert_eq!(
        render(frozen.contract(), &subject, LIMITS).unwrap(),
        task,
        "deterministic"
    );
    assert!(task.len() + HOST_SUFFIX_BYTES <= MAX_PROMPT_BYTES);
    eprintln!(
        "task: {} bytes (+{HOST_SUFFIX_BYTES} Host suffix, bound {MAX_PROMPT_BYTES})",
        task.len()
    );
}

#[test]
fn task_over_the_prompt_bound_is_refused() {
    let subject = common::frozen_subject();
    let head = render("", &subject, LIMITS).unwrap().len();
    let fits = "x".repeat(MAX_PROMPT_BYTES - HOST_SUFFIX_BYTES - head);
    assert!(render(&fits, &subject, LIMITS).is_ok());
    let over = format!("{fits}x");
    assert!(matches!(
        render(&over, &subject, LIMITS),
        Err(Refusal::PromptBound { .. })
    ));
}

#[test]
fn limits_outside_the_host_bounds_are_refused() {
    let subject = common::frozen_subject();
    for limits in [
        Limits {
            max_requests: 0,
            max_reads: 16,
        },
        Limits {
            max_requests: 33,
            max_reads: 16,
        },
        Limits {
            max_requests: 8,
            max_reads: 0,
        },
        Limits {
            max_requests: 8,
            max_reads: 65,
        },
    ] {
        assert!(
            matches!(render("{}", &subject, limits), Err(Refusal::Task(_))),
            "{limits:?}"
        );
    }
}
