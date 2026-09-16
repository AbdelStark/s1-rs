mod common;

use common::{Department, Frustration, Triage};
use pretty_assertions::assert_eq;
use s1::{Choice, ChoiceQuestion, QuestionSet, ScoreQuestion};

#[test]
fn triage_questions_match_golden_json() {
    let actual = serde_json::to_value(Triage::questions()).expect("serialize questions");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/golden/triage.json"))
            .expect("parse golden");
    assert_eq!(actual, expected);
}

#[test]
fn triage_questions_pretty_matches_golden_file() {
    let actual = serde_json::to_string_pretty(&Triage::questions()).expect("pretty");
    let expected = include_str!("../../../tests/golden/triage.json");
    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn choice_labels_are_snake_case_by_default() {
    assert_eq!(Department::Billing.label(), "billing");
    assert_eq!(Department::Technical.label(), "technical");
    assert_eq!(Department::from_label("sales"), Some(Department::Sales));
    assert_eq!(Department::from_label("Sales"), None);
}

#[test]
fn score_index_follows_declaration_order() {
    assert_eq!(Frustration::Calm.index(), 0);
    assert_eq!(Frustration::Annoyed.index(), 1);
    assert_eq!(Frustration::Furious.index(), 2);
    assert_eq!(Frustration::from_index(1), Some(Frustration::Annoyed));
}

#[test]
fn custom_label_and_key_round_trip_on_the_wire() {
    #[derive(Choice)]
    #[s1(instructions = "Which team?", rename_all = "snake_case")]
    enum Team {
        #[s1("Payment or subscription issues")]
        Billing,
        #[s1(label = "tech", desc = "Bugs or integration problems")]
        Technical,
    }

    #[derive(s1::Questions)]
    #[allow(dead_code)]
    struct Ticket {
        #[s1(key = "dept")]
        team: Team,
        #[s1(
            noul = "Mentions a competitor",
            true_means = "Names a competing product",
            false_means = "No competitor named"
        )]
        competitor: bool,
    }

    let q = Ticket::questions();
    let json = serde_json::to_value(&q).expect("json");
    assert_eq!(json["dept"]["type"], "choice");
    assert_eq!(
        json["dept"]["criteria"]["billing"],
        "Payment or subscription issues"
    );
    assert_eq!(
        json["dept"]["criteria"]["tech"],
        "Bugs or integration problems"
    );
    assert!(json["dept"]["criteria"].get("technical").is_none());
    assert_eq!(json["competitor"]["type"], "noul");
    assert_eq!(
        json["competitor"]["criteria"]["true"],
        "Names a competing product"
    );
    assert_eq!(
        json["competitor"]["criteria"]["false"],
        "No competitor named"
    );
}
