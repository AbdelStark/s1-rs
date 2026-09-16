//! Typed [System One](https://docs.typesafe.ai/api) decisions for Rust.
//!
//! Turn enums and structs into Choice, Score, and Noul questions, then
//! [`S1::ask`] and gate on [`Verdict::{Act, Review, Escalate}`](Verdict).
//!
//! Tests and examples use `s1-test::FakeClient` (no network). Enable
//! `backend-typesafe-rs` to talk HTTP via the published
//! [`typesafe-rs`](https://docs.rs/typesafe-rs) crate
//! (`typesafe_rs::Client`). This crate does not implement HTTP.
//!
//! # Quick start
//!
//! ```
//! use s1::{Choice, Policy, Questions, S1, Score, Verdict};
//! use s1_test::{FakeClient, Script};
//!
//! #[derive(Choice)]
//! #[s1(instructions = "Which team should handle this ticket?")]
//! enum Department {
//!     #[s1("Payment, invoice, refund, or subscription issues")]
//!     Billing,
//!     #[s1("Bugs, errors, or integration problems")]
//!     Technical,
//!     #[s1("Pricing or plan questions")]
//!     Sales,
//! }
//!
//! #[derive(Score)]
//! #[s1(instructions = "How frustrated does the customer appear?")]
//! enum Frustration {
//!     #[s1("Calm, just stating facts")]
//!     Calm,
//!     #[s1("Frustrated but civil")]
//!     Annoyed,
//!     #[s1("Angry, strong language")]
//!     Furious,
//! }
//!
//! #[derive(Questions)]
//! #[allow(dead_code)]
//! struct Triage {
//!     department: Department,
//!     frustration: Frustration,
//!     #[s1(noul = "The message conveys urgency or time pressure")]
//!     urgent: bool,
//! }
//!
//! let ticket = "Help! My payouts have been failing for 3 days.";
//! let fake = FakeClient::new().on::<Triage>(|_state| {
//!     Script::new()
//!         .choice(Department::Billing, 0.91)
//!         .score::<Frustration>(1.2)
//!         .noul("urgent", 0.97)
//! });
//! let s1 = S1::new(fake);
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! let t = s1.ask::<Triage>(&ticket).await.unwrap();
//!
//! assert_eq!(
//!     t.department.gate(Policy::act(0.85).review(0.6)),
//!     Verdict::Act(Department::Billing)
//! );
//! assert_eq!(t.frustration.nearest(), Frustration::Annoyed);
//! assert!((t.urgent.p() - 0.97).abs() < 1e-9);
//! # });
//! ```
//!
//! Add a `Department::Legal` variant and every `match` on `Verdict::Act`
//! stops compiling until you handle it.
//!
//! # HTTP via `typesafe-rs`
//!
//! ```ignore
//! use s1::S1;
//! use typesafe_rs::Client;
//!
//! let client = Client::from_env()?;
//! let s1 = S1::new(client);
//! let t = s1.ask::<Triage>(&ticket).await?;
//! ```
//!
//! `Client::from_env()` reads `TYPESAFE_API_KEY`. Retries, timeouts, and
//! identification headers live in `typesafe-rs`.
//!
//! # Confidence gating
//!
//! [`Policy::act`] / [`Policy::review`] map a scalar (default: API
//! `confidence`) onto [`Verdict`]. `>= act` takes the typed value,
//! `>= review` keeps a guess for a human, otherwise escalate.
//!
//! ```
//! use s1::{Choice, ChoiceAnswer, Policy, Verdict};
//!
//! #[derive(Choice)]
//! #[s1(instructions = "Which team?")]
//! enum Department {
//!     #[s1("Payments")]
//!     Billing,
//!     #[s1("Bugs")]
//!     Technical,
//! }
//!
//! let answer = ChoiceAnswer::from_distribution(
//!     Department::Billing,
//!     [(Department::Billing, 0.91), (Department::Technical, 0.09)],
//!     0.91,
//! );
//! assert_eq!(
//!     answer.gate(Policy::act(0.85).review(0.6)),
//!     Verdict::Act(Department::Billing)
//! );
//! ```
//!
//! # Crate features
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | *(none)* | — | Derives, answers, [`Policy`] / [`Verdict`], [`S1::ask`] |
//! | `backend-typesafe-rs` | no | [`DecisionBackend`] for [`typesafe_rs::Client`](https://docs.rs/typesafe-rs/latest/typesafe_rs/struct.Client.html) |
//!
//! MSRV is 1.85 (edition 2024). Dual-licensed MIT OR Apache-2.0.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod answer;
mod backend;
mod decode;
mod policy;
mod traits;
#[cfg(feature = "backend-typesafe-rs")]
mod typesafe_rs;
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
