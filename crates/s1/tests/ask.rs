mod common;

use common::{Department, Frustration, Triage};
use s1::{ChoiceQuestion, Policy, QuestionSet, S1, ScoreQuestion, Verdict};
use s1_test::{FakeClient, Script};

#[tokio::test]
async fn ask_through_fake_client() {
    let fake = FakeClient::new().on::<Triage>(|_state| {
        Script::new()
            .choice(Department::Billing, 0.91)
            .score::<Frustration>(1.2)
            .noul("urgent", 0.97)
    });
    let s1 = S1::new(fake);
    let t = s1
        .ask::<Triage>(&"Help! My payouts have been failing for 3 days.")
        .await
        .expect("ask");

    assert_eq!(t.department.value(), Department::Billing);
    assert!((t.department.prob(Department::Billing) - 0.91).abs() < 1e-9);
    assert_eq!(t.frustration.nearest(), Frustration::Annoyed);
    assert!((t.urgent.p() - 0.97).abs() < 1e-9);
    assert_eq!(t.meta.request_id.as_deref(), Some("fake"));

    match t.department.gate(Policy::act(0.85).review(0.6)) {
        Verdict::Act(Department::Billing) => {}
        other => panic!("expected Act(Billing), got {other:?}"),
    }
}

#[tokio::test]
async fn fake_client_can_branch_on_state() {
    let fake = FakeClient::new().on::<Triage>(|state| {
        let text = state.as_str().unwrap_or("");
        if text.contains("invoice") {
            Script::new()
                .choice(Department::Billing, 0.95)
                .score::<Frustration>(0.1)
                .noul("urgent", 0.2)
        } else {
            Script::new()
                .choice(Department::Technical, 0.8)
                .score::<Frustration>(2.0)
                .noul("urgent", 0.9)
        }
    });
    let s1 = S1::new(fake);
    let t = s1
        .ask::<Triage>(&"Where is my invoice?")
        .await
        .expect("ask");
    assert_eq!(t.department.value(), Department::Billing);
}

#[tokio::test]
async fn unregistered_question_set_errors() {
    #[derive(s1::Choice)]
    #[s1(instructions = "yes or no")]
    enum Yn {
        #[s1("yes")]
        Yes,
        #[s1("no")]
        No,
    }
    #[derive(s1::Questions)]
    #[allow(dead_code)]
    struct Other {
        yn: Yn,
    }

    let fake = FakeClient::new().on::<Triage>(|_s| {
        Script::new()
            .choice(Department::Sales, 0.7)
            .score::<Frustration>(0.0)
            .noul("urgent", 0.1)
    });
    let s1 = S1::new(fake);
    let err = s1
        .ask::<Other>(&"x")
        .await
        .expect_err("should miss handler");
    match err {
        s1::S1Error::Backend(_) => {}
        other => panic!("expected backend error, got {other}"),
    }
}

#[test]
fn question_set_keys_match_fake_registration() {
    let keys: Vec<_> = Triage::questions().keys().cloned().collect();
    assert_eq!(keys, vec!["department", "frustration", "urgent"]);
    assert_eq!(Department::VARIANTS.len(), 3);
    assert_eq!(Frustration::LEVELS.len(), 3);
}
