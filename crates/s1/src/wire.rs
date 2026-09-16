//! Owned mirrors of the TypeSafe System One JSON request and response.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;

use crate::traits::{ChoiceQuestion, Description, Instructions, ScoreQuestion};

/// Ordered map of question id → question, matching the wire `questions` object.
pub type WireQuestions = IndexMap<String, Question>;

/// A System One evaluation request body (plus the model the backend should use).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireRequest {
    /// Content to evaluate: string, object, or array.
    pub state: Value,
    /// Model name. Defaults to `jev-latest` when built by [`crate::S1`].
    pub model: String,
    /// Questions keyed by caller-chosen ids.
    pub questions: WireQuestions,
}

/// A System One evaluation response body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireResponse {
    /// Model that produced the answers.
    pub model: String,
    /// One answer per question, keyed by the same ids as the request.
    pub answers: IndexMap<String, WireAnswer>,
    /// Token usage, when the backend reports it.
    #[serde(default)]
    pub usage: Option<crate::Usage>,
    /// Request correlation id (header `x-typesafe-request-id` on HTTP backends).
    #[serde(default)]
    pub request_id: Option<String>,
}

/// A typed question as sent on the wire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Question {
    /// Yes/no probability.
    Noul {
        /// What to evaluate.
        instructions: Entry,
        /// Optional descriptions of yes and no.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        criteria: Option<NoulCriteria>,
    },
    /// Pick one labelled option.
    Choice {
        /// What to decide.
        instructions: Entry,
        /// Option label → description (insertion order is wire order).
        criteria: IndexMap<String, Entry>,
    },
    /// Ordered rubric levels; index is the score.
    Score {
        /// What to rate.
        instructions: Entry,
        /// Level descriptions in declaration order.
        criteria: Vec<Entry>,
    },
}

/// Optional yes/no descriptions for a Noul question.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct NoulCriteria {
    /// What a value near 1 means.
    #[serde(rename = "true", default, skip_serializing_if = "Option::is_none")]
    pub when_true: Option<Entry>,
    /// What a value near 0 means.
    #[serde(rename = "false", default, skip_serializing_if = "Option::is_none")]
    pub when_false: Option<Entry>,
}

/// A typed answer as returned on the wire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WireAnswer {
    /// Noul probability in `[0, 1]`.
    Noul {
        /// Probability the answer is yes.
        noul: f64,
    },
    /// Chosen label plus the full distribution.
    Choice {
        /// Highest-probability option label.
        choice: String,
        /// Probability of every option (should sum to 1).
        probabilities: IndexMap<String, f64>,
        /// Model confidence in `[0, 1]`.
        confidence: f64,
    },
    /// Expected score across ordered levels.
    Score {
        /// Probability-weighted level index; may fall between levels.
        score: f64,
        /// Level index (string key) → description.
        #[serde(default)]
        legend: IndexMap<String, Value>,
        /// Per-level probabilities when the backend provides them.
        #[serde(default)]
        probabilities: Option<IndexMap<String, f64>>,
        /// Model confidence in `[0, 1]`.
        #[serde(default)]
        confidence: f64,
    },
    /// Unknown `type` tag, retained for forward compatibility.
    #[serde(other)]
    Unknown,
}

/// An `instructions` or criteria value: string, JSON, or null.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    /// Plain text.
    Text(Cow<'static, str>),
    /// Structured JSON (object, array, number, or bool).
    Json(Value),
    /// Explicit null (allowed by TypeSafe for empty descriptions).
    Null,
}

impl Serialize for Entry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Entry::Text(s) => serializer.serialize_str(s),
            Entry::Json(v) => v.serialize(serializer),
            Entry::Null => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Ok(Entry::from(value))
    }
}

impl From<Value> for Entry {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => Entry::Null,
            Value::String(s) => Entry::Text(Cow::Owned(s)),
            other => Entry::Json(other),
        }
    }
}

impl From<&'static str> for Entry {
    fn from(value: &'static str) -> Self {
        Entry::Text(Cow::Borrowed(value))
    }
}

impl From<String> for Entry {
    fn from(value: String) -> Self {
        Entry::Text(Cow::Owned(value))
    }
}

impl From<Instructions> for Entry {
    fn from(value: Instructions) -> Self {
        value.to_entry()
    }
}

impl From<Description> for Entry {
    fn from(value: Description) -> Self {
        value.to_entry()
    }
}

impl Question {
    /// Build a Noul question, omitting `criteria` when both sides are unset.
    pub fn noul(
        instructions: impl Into<Entry>,
        when_true: Option<Entry>,
        when_false: Option<Entry>,
    ) -> Self {
        let criteria = match (when_true, when_false) {
            (None, None) => None,
            (t, f) => Some(NoulCriteria {
                when_true: t,
                when_false: f,
            }),
        };
        Question::Noul {
            instructions: instructions.into(),
            criteria,
        }
    }

    /// Build a Choice question from a [`ChoiceQuestion`] type.
    pub fn from_choice<T: ChoiceQuestion>() -> Self {
        let mut criteria = IndexMap::with_capacity(T::VARIANTS.len());
        for &variant in T::VARIANTS {
            criteria.insert(
                variant.label().to_string(),
                variant.description().to_entry(),
            );
        }
        Question::Choice {
            instructions: T::INSTRUCTIONS.to_entry(),
            criteria,
        }
    }

    /// Build a Score question from a [`ScoreQuestion`] type.
    pub fn from_score<T: ScoreQuestion>() -> Self {
        let criteria = T::LEVELS
            .iter()
            .copied()
            .map(|level| level.level_description().to_entry())
            .collect();
        Question::Score {
            instructions: T::INSTRUCTIONS.to_entry(),
            criteria,
        }
    }
}
