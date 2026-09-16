//! Typed answers: [`ChoiceAnswer`], [`ScoreAnswer`], [`NoulAnswer`].

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::decode::{DecodeError, SUM_EPS, in_range, wire_type_name};
use crate::policy::{NoulPolicy, Policy, Verdict};
use crate::traits::{ChoiceQuestion, QuestionSet, ScoreQuestion};
use crate::wire::{WireAnswer, WireResponse};

/// Token usage reported by the backend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Prompt / input tokens.
    #[serde(default)]
    pub input_tokens: u64,
    /// Completion / output tokens.
    #[serde(default)]
    pub output_tokens: u64,
}

/// Per-request metadata attached to a generated answers struct.
#[derive(Clone, Debug, PartialEq)]
pub struct AnswerMeta {
    /// Model that produced the answers.
    pub model: String,
    /// Token usage, when present.
    pub usage: Option<Usage>,
    /// Backend request id, when present.
    pub request_id: Option<String>,
    /// Count of missing probability labels and renormalizations applied while decoding.
    pub normalization_events: u32,
    raw: Option<serde_json::Value>,
}

impl AnswerMeta {
    /// Build metadata from a response, retaining a JSON copy for audit.
    pub fn from_response(resp: &WireResponse) -> Self {
        Self {
            model: resp.model.clone(),
            usage: resp.usage.clone(),
            request_id: resp.request_id.clone(),
            normalization_events: 0,
            raw: serde_json::to_value(resp).ok(),
        }
    }

    /// Raw JSON of the [`WireResponse`], when serialization succeeded.
    pub fn raw(&self) -> Option<&serde_json::Value> {
        self.raw.as_ref()
    }
}

/// Answers struct generated for question set `Q`.
pub type Answers<Q> = <Q as QuestionSet>::Answers;

/// A decoded Choice answer.
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceAnswer<T: ChoiceQuestion> {
    value: T,
    dist: Vec<f64>,
    confidence: f64,
}

impl<T: ChoiceQuestion> ChoiceAnswer<T> {
    /// Chosen variant (the wire `choice` label).
    pub fn value(&self) -> T {
        self.value
    }

    /// Probability assigned to `v`. Missing labels decode as `0.0`.
    pub fn prob(&self, v: T) -> f64 {
        self.dist.get(v.index()).copied().unwrap_or(0.0)
    }

    /// Distribution in variant-declaration order.
    pub fn distribution(&self) -> impl Iterator<Item = (T, f64)> + '_ {
        T::VARIANTS.iter().copied().zip(self.dist.iter().copied())
    }

    /// API confidence in `[0, 1]`.
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Top-1 probability minus top-2 probability.
    pub fn margin(&self) -> f64 {
        let mut top1 = 0.0;
        let mut top2 = 0.0;
        for &p in &self.dist {
            if p > top1 {
                top2 = top1;
                top1 = p;
            } else if p > top2 {
                top2 = p;
            }
        }
        top1 - top2
    }

    /// Shannon entropy of the distribution, in nats.
    pub fn entropy(&self) -> f64 {
        self.dist
            .iter()
            .copied()
            .filter(|p| *p > 0.0)
            .map(|p| -p * p.ln())
            .sum()
    }

    /// Gate this answer with a [`Policy`].
    pub fn gate(&self, policy: impl std::borrow::Borrow<Policy<T>>) -> Verdict<T> {
        let policy = policy.borrow();
        let signal = match policy.signal() {
            crate::Signal::Confidence => self.confidence,
            crate::Signal::TopProb => self.prob(self.value),
            crate::Signal::Margin => self.margin(),
        };
        policy.verdict(signal, self.value)
    }

    /// Construct an answer from a chosen variant, a distribution, and confidence.
    ///
    /// Intended for tests and fakes. Does not validate that probabilities sum to 1.
    pub fn from_distribution(
        value: T,
        dist: impl IntoIterator<Item = (T, f64)>,
        confidence: f64,
    ) -> Self {
        let mut values = vec![0.0; T::VARIANTS.len()];
        for (variant, p) in dist {
            if let Some(slot) = values.get_mut(variant.index()) {
                *slot = p;
            }
        }
        Self {
            value,
            dist: values,
            confidence,
        }
    }

    /// Decode from a [`WireResponse`].
    pub fn decode(
        key: &str,
        resp: &WireResponse,
        meta: &mut AnswerMeta,
    ) -> Result<Self, DecodeError> {
        let answer = resp
            .answers
            .get(key)
            .ok_or_else(|| DecodeError::MissingAnswer {
                key: key.to_string(),
            })?;
        match answer {
            WireAnswer::Choice {
                choice,
                probabilities,
                confidence,
            } => Self::from_wire(key, choice, probabilities, *confidence, meta),
            other => Err(DecodeError::wrong_type(
                key,
                "choice",
                wire_type_name(other),
            )),
        }
    }

    fn from_wire(
        key: &str,
        choice: &str,
        probabilities: &IndexMap<String, f64>,
        confidence: f64,
        meta: &mut AnswerMeta,
    ) -> Result<Self, DecodeError> {
        let value = T::from_label(choice).ok_or_else(|| DecodeError::UnknownLabel {
            key: key.to_string(),
            label: choice.to_string(),
        })?;

        let n = T::VARIANTS.len();
        let mut dist = vec![0.0; n];
        for (i, variant) in T::VARIANTS.iter().enumerate() {
            match probabilities.get(variant.label()) {
                Some(p) => dist[i] = *p,
                None => {
                    dist[i] = 0.0;
                    meta.normalization_events = meta.normalization_events.saturating_add(1);
                }
            }
        }
        for label in probabilities.keys() {
            if T::from_label(label).is_none() {
                meta.normalization_events = meta.normalization_events.saturating_add(1);
            }
        }
        let sum: f64 = dist.iter().sum();
        if (sum - 1.0).abs() > SUM_EPS && sum > SUM_EPS {
            for p in &mut dist {
                *p /= sum;
            }
            meta.normalization_events = meta.normalization_events.saturating_add(1);
        }

        Ok(Self {
            value,
            dist,
            confidence,
        })
    }
}

/// A decoded Score answer.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreAnswer<T: ScoreQuestion> {
    expected: f64,
    dist: Option<Vec<f64>>,
    confidence: f64,
    _ty: PhantomData<T>,
}

impl<T: ScoreQuestion> ScoreAnswer<T> {
    /// Probability-weighted score (the wire `score` field).
    pub fn expected(&self) -> f64 {
        self.expected
    }

    /// Nearest level: `round(expected)` clamped to `[0, n-1]`.
    pub fn nearest(&self) -> T {
        let max_idx = T::LEVELS.len().saturating_sub(1);
        let idx = self.expected.round().clamp(0.0, max_idx as f64) as usize;
        match T::from_index(idx) {
            Some(v) => v,
            None => match T::LEVELS.first().copied() {
                Some(v) => v,
                None => panic!("ScoreQuestion has no levels"),
            },
        }
    }

    /// `P(level >= L)` when a per-level distribution is present.
    pub fn at_least(&self, level: T) -> Option<f64> {
        let dist = self.dist.as_ref()?;
        let start = level.index();
        Some(
            dist.iter()
                .enumerate()
                .filter(|(i, _)| *i >= start)
                .map(|(_, p)| *p)
                .sum(),
        )
    }

    /// API confidence in `[0, 1]`.
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Per-level distribution in declaration order, when provided.
    pub fn distribution(&self) -> Option<impl Iterator<Item = (T, f64)> + '_> {
        let dist = self.dist.as_ref()?;
        Some(T::LEVELS.iter().copied().zip(dist.iter().copied()))
    }

    /// Gate this answer with a [`Policy`]. Uses [`Self::nearest`] as the value.
    pub fn gate(&self, policy: impl std::borrow::Borrow<Policy<T>>) -> Verdict<T> {
        let policy = policy.borrow();
        let value = self.nearest();
        let signal = match policy.signal() {
            crate::Signal::Confidence => self.confidence,
            crate::Signal::TopProb => self
                .dist
                .as_ref()
                .map(|d| max_f64(d))
                .unwrap_or(self.confidence),
            crate::Signal::Margin => self
                .dist
                .as_ref()
                .map(|d| margin_of(d))
                .unwrap_or(self.confidence),
        };
        policy.verdict(signal, value)
    }

    /// Construct an answer from an expected score and confidence.
    pub fn from_expected(expected: f64, confidence: f64) -> Self {
        Self {
            expected,
            dist: None,
            confidence,
            _ty: PhantomData,
        }
    }

    /// Decode from a [`WireResponse`].
    pub fn decode(
        key: &str,
        resp: &WireResponse,
        meta: &mut AnswerMeta,
    ) -> Result<Self, DecodeError> {
        let answer = resp
            .answers
            .get(key)
            .ok_or_else(|| DecodeError::MissingAnswer {
                key: key.to_string(),
            })?;
        match answer {
            WireAnswer::Score {
                score,
                probabilities,
                confidence,
                ..
            } => Self::from_wire(key, *score, probabilities.as_ref(), *confidence, meta),
            other => Err(DecodeError::wrong_type(key, "score", wire_type_name(other))),
        }
    }

    fn from_wire(
        key: &str,
        score: f64,
        probabilities: Option<&IndexMap<String, f64>>,
        confidence: f64,
        meta: &mut AnswerMeta,
    ) -> Result<Self, DecodeError> {
        let n = T::LEVELS.len();
        let max = n.saturating_sub(1) as f64;
        if !in_range(score, 0.0, max) {
            return Err(DecodeError::OutOfRange {
                key: key.to_string(),
                value: score,
                min: 0.0,
                max,
            });
        }

        let dist = match probabilities {
            None => None,
            Some(probs) => {
                let mut d = vec![0.0; n];
                for (i, slot) in d.iter_mut().enumerate() {
                    let k = i.to_string();
                    match probs.get(&k) {
                        Some(p) => *slot = *p,
                        None => {
                            *slot = 0.0;
                            meta.normalization_events = meta.normalization_events.saturating_add(1);
                        }
                    }
                }
                let sum: f64 = d.iter().sum();
                if (sum - 1.0).abs() > SUM_EPS && sum > SUM_EPS {
                    for p in &mut d {
                        *p /= sum;
                    }
                    meta.normalization_events = meta.normalization_events.saturating_add(1);
                }
                Some(d)
            }
        };

        Ok(Self {
            expected: score,
            dist,
            confidence,
            _ty: PhantomData,
        })
    }
}

/// A decoded Noul answer.
#[derive(Clone, Debug, PartialEq)]
pub struct NoulAnswer {
    p: f64,
}

impl NoulAnswer {
    /// Probability the proposition is true, in `[0, 1]`.
    pub fn p(&self) -> f64 {
        self.p
    }

    /// `true` when `p >= 0.5`.
    pub fn is_likely(&self) -> bool {
        self.p >= 0.5
    }

    /// Gate this answer with a [`NoulPolicy`].
    pub fn gate(&self, policy: impl std::borrow::Borrow<NoulPolicy>) -> Verdict<bool> {
        policy.borrow().verdict(self.p)
    }

    /// Construct a noul answer. Returns [`DecodeError::OutOfRange`] if `p` is outside `[0, 1]`.
    pub fn from_p(p: f64) -> Result<Self, DecodeError> {
        Self::checked("noul", p)
    }

    /// Decode from a [`WireResponse`].
    pub fn decode(
        key: &str,
        resp: &WireResponse,
        _meta: &mut AnswerMeta,
    ) -> Result<Self, DecodeError> {
        let answer = resp
            .answers
            .get(key)
            .ok_or_else(|| DecodeError::MissingAnswer {
                key: key.to_string(),
            })?;
        match answer {
            WireAnswer::Noul { noul } => Self::checked(key, *noul),
            other => Err(DecodeError::wrong_type(key, "noul", wire_type_name(other))),
        }
    }

    fn checked(key: &str, p: f64) -> Result<Self, DecodeError> {
        if !in_range(p, 0.0, 1.0) {
            return Err(DecodeError::OutOfRange {
                key: key.to_string(),
                value: p,
                min: 0.0,
                max: 1.0,
            });
        }
        Ok(Self { p })
    }
}

fn max_f64(xs: &[f64]) -> f64 {
    let mut m = f64::NEG_INFINITY;
    for &x in xs {
        if x > m {
            m = x;
        }
    }
    if m.is_finite() { m } else { 0.0 }
}

fn margin_of(xs: &[f64]) -> f64 {
    let mut top1 = 0.0;
    let mut top2 = 0.0;
    for &p in xs {
        if p > top1 {
            top2 = top1;
            top1 = p;
        } else if p > top2 {
            top2 = p;
        }
    }
    top1 - top2
}
