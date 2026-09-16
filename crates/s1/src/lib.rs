//! Typed System One decisions for Rust.
//!
//! Turn enums and structs into Choice, Score, and Noul questions, and get
//! compile-time-checked, confidence-gated answers back.
//!
//! Transport lives in a [`DecisionBackend`]. v0.1 tests and examples use
//! `s1-test::FakeClient`; HTTP belongs in `typesafe-rs`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod answer;
mod backend;
mod decode;
mod policy;
mod traits;
mod wire;

pub use answer::{AnswerMeta, Answers, ChoiceAnswer, NoulAnswer, ScoreAnswer, Usage};
pub use backend::{DEFAULT_MODEL, DecisionBackend, S1, S1Error};
pub use decode::DecodeError;
pub use policy::{NoulPolicy, Policy, PolicyError, Signal, Verdict};
pub use s1_derive::{Choice, Questions, Score};
pub use traits::{
    ChoiceQuestion, Description, Instructions, MAX_CHOICE_OPTIONS, MIN_CHOICE_OPTIONS,
    MIN_SCORE_LEVELS, QuestionField, QuestionSet, ScoreQuestion,
};
pub use wire::{
    Entry, NoulCriteria, Question, WireAnswer, WireQuestions, WireRequest, WireResponse,
};

/// Common imports for application code.
pub mod prelude {
    pub use crate::{
        Answers, Choice, ChoiceAnswer, ChoiceQuestion, DecisionBackend, NoulAnswer, NoulPolicy,
        Policy, QuestionSet, Questions, S1, Score, ScoreAnswer, ScoreQuestion, Verdict,
    };
}
