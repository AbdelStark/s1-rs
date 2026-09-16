mod common;

use common::{Department, Frustration, Triage};
use indexmap::IndexMap;
use s1::{DecodeError, QuestionSet, Usage, WireAnswer, WireResponse};

fn valid_response() -> WireResponse {
    let mut answers = IndexMap::new();
    answers.insert(
        "department".into(),
        WireAnswer::Choice {
            choice: "billing".into(),
            probabilities: IndexMap::from_iter([
                ("billing".into(), 0.91),
                ("technical".into(), 0.05),
                ("sales".into(), 0.04),
            ]),
            confidence: 0.88,
        },
    );
    answers.insert(
        "frustration".into(),
        WireAnswer::Score {
            score: 1.2,
            legend: IndexMap::from_iter([
                ("0".into(), serde_json::json!("Calm, just stating facts")),
                ("1".into(), serde_json::json!("Frustrated but civil")),
                ("2".into(), serde_json::json!("Angry, strong language")),
            ]),
            probabilities: Some(IndexMap::from_iter([
                ("0".into(), 0.1),
                ("1".into(), 0.6),
                ("2".into(), 0.3),
            ])),
            confidence: 0.7,
        },
    );
    answers.insert("urgent".into(), WireAnswer::Noul { noul: 0.97 });
    WireResponse {
        model: "jev-latest".into(),
        answers,
        usage: Some(Usage {
            input_tokens: 10,
            output_tokens: 3,
        }),
        request_id: Some("req-1".into()),
    }
}

#[test]
fn decode_happy_path_through_question_set() {
    let t = Triage::decode(&valid_response()).expect("decode");
    assert_eq!(t.department.value(), Department::Billing);
    assert!((t.department.prob(Department::Billing) - 0.91).abs() < 1e-9);
    assert!((t.department.confidence() - 0.88).abs() < 1e-9);
    assert_eq!(t.frustration.nearest(), Frustration::Annoyed);
    assert!((t.frustration.expected() - 1.2).abs() < 1e-9);
    assert!((t.urgent.p() - 0.97).abs() < 1e-9);
    assert!(t.urgent.is_likely());
    assert_eq!(t.meta.model, "jev-latest");
    assert!(t.meta.raw().is_some());
}

#[test]
fn unknown_choice_label_is_decode_error() {
    let mut resp = valid_response();
    resp.answers.insert(
        "department".into(),
        WireAnswer::Choice {
            choice: "legal".into(),
            probabilities: IndexMap::from_iter([("legal".into(), 1.0)]),
            confidence: 0.9,
        },
    );
    match Triage::decode(&resp) {
        Err(DecodeError::UnknownLabel { key, label }) => {
            assert_eq!(key, "department");
            assert_eq!(label, "legal");
        }
        other => panic!("expected UnknownLabel, got {other:?}"),
    }
}

#[test]
fn missing_answer_is_decode_error() {
    let mut resp = valid_response();
    resp.answers.shift_remove("urgent");
    match Triage::decode(&resp) {
        Err(DecodeError::MissingAnswer { key }) => assert_eq!(key, "urgent"),
        other => panic!("expected MissingAnswer, got {other:?}"),
    }
}

#[test]
fn noul_out_of_range_is_decode_error() {
    let mut resp = valid_response();
    resp.answers
        .insert("urgent".into(), WireAnswer::Noul { noul: 1.5 });
    match Triage::decode(&resp) {
        Err(DecodeError::OutOfRange {
            key,
            value,
            min,
            max,
        }) => {
            assert_eq!(key, "urgent");
            assert!((value - 1.5).abs() < 1e-12);
            assert_eq!((min, max), (0.0, 1.0));
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
}

#[test]
fn score_out_of_range_is_decode_error() {
    let mut resp = valid_response();
    if let Some(WireAnswer::Score { score, .. }) = resp.answers.get_mut("frustration") {
        *score = 9.0;
    }
    match Triage::decode(&resp) {
        Err(DecodeError::OutOfRange { key, max, .. }) => {
            assert_eq!(key, "frustration");
            assert!((max - 2.0).abs() < 1e-12);
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
}

#[test]
fn missing_probability_labels_become_zero_and_count_events() {
    let mut resp = valid_response();
    resp.answers.insert(
        "department".into(),
        WireAnswer::Choice {
            choice: "billing".into(),
            probabilities: IndexMap::from_iter([
                ("billing".into(), 0.5),
                ("technical".into(), 0.5),
                // sales omitted
            ]),
            confidence: 0.5,
        },
    );
    let t = Triage::decode(&resp).expect("decode");
    assert_eq!(t.department.prob(Department::Sales), 0.0);
    assert!(t.meta.normalization_events >= 1);
}

#[test]
fn optional_choice_allows_missing_answer() {
    #[derive(s1::Choice)]
    #[s1(instructions = "Pick a side")]
    enum Side {
        #[s1("Left")]
        Left,
        #[s1("Right")]
        Right,
    }

    #[derive(s1::Questions)]
    #[allow(dead_code)]
    struct Maybe {
        side: Option<Side>,
    }

    let resp = WireResponse {
        model: "jev-latest".into(),
        answers: IndexMap::new(),
        usage: None,
        request_id: None,
    };
    let a = Maybe::decode(&resp).expect("optional missing is ok");
    assert!(a.side.is_none());
}

#[test]
fn score_nearest_rounds_and_clamps() {
    let t = Triage::decode(&valid_response()).expect("decode");
    assert_eq!(t.frustration.nearest(), Frustration::Annoyed);
    let at_least = t.frustration.at_least(Frustration::Annoyed).expect("dist");
    assert!((at_least - 0.9).abs() < 1e-9);
}
