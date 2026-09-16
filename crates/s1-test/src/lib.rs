//! Test helpers for `s1`. No network: script answers per question set.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use indexmap::IndexMap;
use s1::{
    ChoiceQuestion, DecisionBackend, Question, QuestionSet, ScoreQuestion, Usage, WireAnswer,
    WireRequest, WireResponse,
};
use std::collections::{BTreeSet, HashMap};
use std::future::{Future, ready};
use std::sync::Arc;
use thiserror::Error;

/// Scriptable in-memory [`DecisionBackend`].
#[derive(Clone, Default)]
pub struct FakeClient {
    handlers: Vec<Handler>,
}

#[derive(Clone)]
struct Handler {
    keys: BTreeSet<String>,
    f: Arc<dyn Fn(serde_json::Value) -> Script + Send + Sync>,
}

impl FakeClient {
    /// Empty client. Register handlers with [`Self::on`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run `f` when a request's question keys match `Q::questions()`.
    pub fn on<Q: QuestionSet + 'static>(
        mut self,
        f: impl Fn(serde_json::Value) -> Script + Send + Sync + 'static,
    ) -> Self {
        let keys = Q::questions().keys().cloned().collect();
        self.handlers.push(Handler {
            keys,
            f: Arc::new(f),
        });
        self
    }
}

impl DecisionBackend for FakeClient {
    type Error = FakeError;

    fn evaluate(
        &self,
        req: WireRequest,
    ) -> impl Future<Output = Result<WireResponse, Self::Error>> + Send {
        ready(self.evaluate_sync(req))
    }
}

impl FakeClient {
    fn evaluate_sync(&self, req: WireRequest) -> Result<WireResponse, FakeError> {
        let keys: BTreeSet<String> = req.questions.keys().cloned().collect();
        let handler = self
            .handlers
            .iter()
            .find(|h| h.keys == keys)
            .ok_or_else(|| FakeError::NoHandler(keys.iter().cloned().collect()))?;
        let script = (handler.f)(req.state.clone());
        script.apply(&req)
    }
}

/// Failure from [`FakeClient`].
#[derive(Debug, Error, Clone, PartialEq)]
#[non_exhaustive]
pub enum FakeError {
    /// No `.on::<Q>()` registration matched the request's question keys.
    #[error("no FakeClient script registered for questions {0:?}")]
    NoHandler(Vec<String>),
    /// The script did not provide an answer for this question id.
    #[error("FakeClient script missing answer for {0}")]
    MissingScript(String),
}

/// Answers to emit for one request.
#[derive(Clone, Debug, Default)]
pub struct Script {
    choices: Vec<ScriptedChoice>,
    scores: Vec<ScriptedScore>,
    nouls: HashMap<String, f64>,
}

#[derive(Clone, Debug)]
struct ScriptedChoice {
    key: Option<String>,
    labels: BTreeSet<String>,
    chosen: String,
    probabilities: IndexMap<String, f64>,
    confidence: f64,
}

#[derive(Clone, Debug)]
struct ScriptedScore {
    key: Option<String>,
    n_levels: usize,
    expected: f64,
    legend: IndexMap<String, serde_json::Value>,
    probabilities: IndexMap<String, f64>,
    confidence: f64,
}

impl Script {
    /// Empty script. Chain [`Self::choice`], [`Self::score`], [`Self::noul`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Peak a Choice distribution on `value` with the given confidence/top probability.
    pub fn choice<T: ChoiceQuestion>(self, value: T, confidence: f64) -> Self {
        self.choice_on::<T>(None, value, confidence)
    }

    /// Like [`Self::choice`], bound to a question key.
    pub fn choice_key<T: ChoiceQuestion>(
        self,
        key: impl Into<String>,
        value: T,
        confidence: f64,
    ) -> Self {
        self.choice_on::<T>(Some(key.into()), value, confidence)
    }

    fn choice_on<T: ChoiceQuestion>(
        mut self,
        key: Option<String>,
        value: T,
        confidence: f64,
    ) -> Self {
        let n = T::VARIANTS.len();
        let rest = (1.0 - confidence).max(0.0);
        let others = (n.saturating_sub(1)) as f64;
        let mut probabilities = IndexMap::new();
        let mut labels = BTreeSet::new();
        for &variant in T::VARIANTS {
            let label = variant.label().to_string();
            labels.insert(label.clone());
            let p = if variant == value {
                confidence
            } else if others > 0.0 {
                rest / others
            } else {
                0.0
            };
            probabilities.insert(label, p);
        }
        self.choices.push(ScriptedChoice {
            key,
            labels,
            chosen: value.label().to_string(),
            probabilities,
            confidence,
        });
        self
    }

    /// Emit a Score whose expected value is `expected`.
    pub fn score<T: ScoreQuestion>(self, expected: f64) -> Self {
        self.score_on::<T>(None, expected)
    }

    /// Like [`Self::score`], bound to a question key.
    pub fn score_key<T: ScoreQuestion>(self, key: impl Into<String>, expected: f64) -> Self {
        self.score_on::<T>(Some(key.into()), expected)
    }

    fn score_on<T: ScoreQuestion>(mut self, key: Option<String>, expected: f64) -> Self {
        let n = T::LEVELS.len();
        let mut legend = IndexMap::new();
        for (i, level) in T::LEVELS.iter().enumerate() {
            let desc = match level.level_description().to_entry() {
                s1::Entry::Text(s) => serde_json::Value::String(s.into_owned()),
                s1::Entry::Json(v) => v,
                s1::Entry::Null => serde_json::Value::Null,
            };
            legend.insert(i.to_string(), desc);
        }
        let probabilities = peaked_score_dist(expected, n);
        self.scores.push(ScriptedScore {
            key,
            n_levels: n,
            expected,
            legend,
            probabilities,
            confidence: 0.8,
        });
        self
    }

    /// Emit a Noul probability for `key`.
    pub fn noul(mut self, key: impl Into<String>, p: f64) -> Self {
        self.nouls.insert(key.into(), p);
        self
    }

    fn apply(&self, req: &WireRequest) -> Result<WireResponse, FakeError> {
        let mut answers = IndexMap::new();
        let mut used_choice = vec![false; self.choices.len()];
        let mut used_score = vec![false; self.scores.len()];

        for (key, question) in &req.questions {
            match question {
                Question::Noul { .. } => {
                    let p = self
                        .nouls
                        .get(key)
                        .copied()
                        .ok_or_else(|| FakeError::MissingScript(key.clone()))?;
                    answers.insert(key.clone(), WireAnswer::Noul { noul: p });
                }
                Question::Choice { criteria, .. } => {
                    let crit: BTreeSet<String> = criteria.keys().cloned().collect();
                    let found = self.choices.iter().enumerate().find_map(|(i, c)| {
                        if used_choice[i] {
                            return None;
                        }
                        if c.key.as_ref() == Some(key) || (c.key.is_none() && c.labels == crit) {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    let i = found.ok_or_else(|| FakeError::MissingScript(key.clone()))?;
                    used_choice[i] = true;
                    let c = &self.choices[i];
                    answers.insert(
                        key.clone(),
                        WireAnswer::Choice {
                            choice: c.chosen.clone(),
                            probabilities: c.probabilities.clone(),
                            confidence: c.confidence,
                        },
                    );
                }
                Question::Score { criteria, .. } => {
                    let n = criteria.len();
                    let found = self.scores.iter().enumerate().find_map(|(i, s)| {
                        if used_score[i] {
                            return None;
                        }
                        if s.key.as_ref() == Some(key) || (s.key.is_none() && s.n_levels == n) {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    let i = found.ok_or_else(|| FakeError::MissingScript(key.clone()))?;
                    used_score[i] = true;
                    let s = &self.scores[i];
                    answers.insert(
                        key.clone(),
                        WireAnswer::Score {
                            score: s.expected,
                            legend: s.legend.clone(),
                            probabilities: Some(s.probabilities.clone()),
                            confidence: s.confidence,
                        },
                    );
                }
            }
        }

        Ok(WireResponse {
            model: req.model.clone(),
            answers,
            usage: Some(Usage {
                input_tokens: 0,
                output_tokens: 0,
            }),
            request_id: Some("fake".to_string()),
        })
    }
}

fn peaked_score_dist(expected: f64, n: usize) -> IndexMap<String, f64> {
    let mut probabilities = IndexMap::new();
    if n == 0 {
        return probabilities;
    }
    let max = (n - 1) as f64;
    let x = expected.clamp(0.0, max);
    let lo = x.floor() as usize;
    let hi = x.ceil() as usize;
    let frac = x - lo as f64;
    for i in 0..n {
        let p = if lo == hi {
            if i == lo { 1.0 } else { 0.0 }
        } else if i == lo {
            1.0 - frac
        } else if i == hi {
            frac
        } else {
            0.0
        };
        probabilities.insert(i.to_string(), p);
    }
    probabilities
}
