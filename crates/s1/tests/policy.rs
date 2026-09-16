mod common;

use common::Department;
use s1::{ChoiceAnswer, NoulAnswer, NoulPolicy, Policy, PolicyError, Signal, Verdict};

fn choice_with_confidence(value: Department, confidence: f64) -> ChoiceAnswer<Department> {
    let rest = (1.0 - confidence).max(0.0) / 2.0;
    ChoiceAnswer::from_distribution(
        value,
        [
            (
                Department::Billing,
                if value == Department::Billing {
                    confidence
                } else {
                    rest
                },
            ),
            (
                Department::Technical,
                if value == Department::Technical {
                    confidence
                } else {
                    rest
                },
            ),
            (
                Department::Sales,
                if value == Department::Sales {
                    confidence
                } else {
                    rest
                },
            ),
        ],
        confidence,
    )
}

#[test]
fn choice_gate_act_review_escalate() {
    let policy = Policy::act(0.85).review(0.6);
    assert_eq!(
        choice_with_confidence(Department::Billing, 0.91).gate(&policy),
        Verdict::Act(Department::Billing)
    );
    assert_eq!(
        choice_with_confidence(Department::Technical, 0.70).gate(&policy),
        Verdict::Review(Department::Technical)
    );
    assert_eq!(
        choice_with_confidence(Department::Sales, 0.40).gate(&policy),
        Verdict::Escalate
    );
}

#[test]
fn gate_accepts_policy_by_value_matching_vision_snippet() {
    let v = choice_with_confidence(Department::Billing, 0.91).gate(Policy::act(0.85).review(0.6));
    assert!(matches!(v, Verdict::Act(Department::Billing)));
}

#[test]
fn per_variant_override() {
    let policy = Policy::act(0.7)
        .review(0.4)
        .for_variant(Department::Sales, 0.95, 0.8);
    assert_eq!(
        choice_with_confidence(Department::Billing, 0.75).gate(&policy),
        Verdict::Act(Department::Billing)
    );
    assert_eq!(
        choice_with_confidence(Department::Sales, 0.75).gate(&policy),
        Verdict::Escalate
    );
}

#[test]
fn try_act_rejects_out_of_unit_interval() {
    assert!(matches!(
        Policy::<Department>::try_act(1.2),
        Err(PolicyError::OutOfUnitInterval(_))
    ));
    assert!(matches!(
        Policy::<Department>::try_act(-0.1),
        Err(PolicyError::OutOfUnitInterval(_))
    ));
}

#[test]
fn try_review_rejects_act_below_review() {
    let err = Policy::<Department>::try_act(0.4)
        .expect("act")
        .try_review(0.8)
        .expect_err("review > act");
    assert!(matches!(err, PolicyError::ActBelowReview { .. }));
}

#[test]
fn noul_gate_yes_no_review_escalate() {
    let policy = NoulPolicy::new(0.9, 0.1, 0.4, 0.6);
    assert_eq!(
        NoulAnswer::from_p(0.95).unwrap().gate(&policy),
        Verdict::Act(true)
    );
    assert_eq!(
        NoulAnswer::from_p(0.05).unwrap().gate(&policy),
        Verdict::Act(false)
    );
    assert_eq!(
        NoulAnswer::from_p(0.5).unwrap().gate(&policy),
        Verdict::Review(true)
    );
    assert_eq!(
        NoulAnswer::from_p(0.75).unwrap().gate(&policy),
        Verdict::Escalate
    );
}

#[test]
fn noul_try_new_rejects_yes_below_no() {
    assert!(matches!(
        NoulPolicy::try_new(0.2, 0.8, 0.3, 0.4),
        Err(PolicyError::YesBelowNo { .. })
    ));
}

#[test]
fn verdict_is_monotone_in_confidence() {
    let policy = Policy::act(0.8).review(0.5);
    let mut prev_rank = 0u8;
    let mut first = true;
    for i in 0..=20 {
        let c = i as f64 / 20.0;
        let rank = choice_with_confidence(Department::Billing, c)
            .gate(&policy)
            .rank();
        if !first {
            assert!(
                rank >= prev_rank,
                "rank dropped from {prev_rank} to {rank} at confidence {c}"
            );
        }
        first = false;
        prev_rank = rank;
    }
}

#[test]
fn top_prob_signal() {
    let policy = Policy::act(0.9).review(0.5).using(Signal::TopProb);
    let answer = choice_with_confidence(Department::Billing, 0.91);
    assert_eq!(answer.gate(&policy), Verdict::Act(Department::Billing));
}

#[test]
fn noul_from_p_rejects_out_of_range() {
    assert!(NoulAnswer::from_p(-0.01).is_err());
    assert!(NoulAnswer::from_p(1.01).is_err());
}
