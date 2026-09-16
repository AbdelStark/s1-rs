//! Decode errors for turning a [`crate::WireResponse`] into typed answers.

use thiserror::Error;

/// Failure to turn a wire answer into a typed field.
#[derive(Debug, Error, Clone, PartialEq)]
#[non_exhaustive]
pub enum DecodeError {
    /// The Choice label is not a variant of the target enum.
    #[error("unknown choice label {label:?} for question {key:?}")]
    UnknownLabel {
        /// Question id.
        key: String,
        /// Label returned by the model.
        label: String,
    },
    /// A required question id was missing from `answers`.
    #[error("missing answer for question {key:?}")]
    MissingAnswer {
        /// Question id.
        key: String,
    },
    /// A Noul value outside `[0, 1]` or a Score outside `[0, n-1]`.
    #[error("value {value} for question {key:?} is outside [{min}, {max}]")]
    OutOfRange {
        /// Question id.
        key: String,
        /// Offending number.
        value: f64,
        /// Inclusive lower bound.
        min: f64,
        /// Inclusive upper bound.
        max: f64,
    },
    /// The answer `type` did not match the field (e.g. noul where a choice was expected).
    #[error("answer for question {key:?} had type {actual}, expected {expected}")]
    WrongType {
        /// Question id.
        key: String,
        /// Expected wire type name.
        expected: &'static str,
        /// Actual wire type name.
        actual: &'static str,
    },
}

impl DecodeError {
    pub(crate) fn wrong_type(key: &str, expected: &'static str, actual: &'static str) -> Self {
        DecodeError::WrongType {
            key: key.to_string(),
            expected,
            actual,
        }
    }
}

/// Absolute tolerance used when checking unit-interval and score-range bounds.
pub(crate) const RANGE_EPS: f64 = 1e-6;
/// Absolute tolerance used when deciding whether a distribution needs renormalization.
pub(crate) const SUM_EPS: f64 = 1e-6;

pub(crate) fn in_range(value: f64, min: f64, max: f64) -> bool {
    value >= min - RANGE_EPS && value <= max + RANGE_EPS
}

pub(crate) fn wire_type_name(answer: &crate::WireAnswer) -> &'static str {
    match answer {
        crate::WireAnswer::Noul { .. } => "noul",
        crate::WireAnswer::Choice { .. } => "choice",
        crate::WireAnswer::Score { .. } => "score",
        crate::WireAnswer::Unknown => "unknown",
    }
}
