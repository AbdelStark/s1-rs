//! Confidence gating: [`Policy`], [`NoulPolicy`], and [`Verdict`].

use thiserror::Error;

/// Outcome of gating an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict<T> {
    /// Signal is at or above the act threshold; take the typed value.
    Act(T),
    /// Signal is below act but at or above review; keep a guess for a human.
    Review(T),
    /// Signal is below review; escalate (LLM fallback, etc.).
    Escalate,
}

impl<T> Verdict<T> {
    /// Rank used by monotonicity tests: Act (2) > Review (1) > Escalate (0).
    pub fn rank(&self) -> u8 {
        match self {
            Verdict::Act(_) => 2,
            Verdict::Review(_) => 1,
            Verdict::Escalate => 0,
        }
    }
}

/// Which scalar a [`Policy`] compares against thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Signal {
    /// API `confidence` (default).
    #[default]
    Confidence,
    /// Probability of the chosen / top option.
    TopProb,
    /// Top-1 minus top-2 probability.
    Margin,
}

/// Invalid [`Policy`] or [`NoulPolicy`] construction.
#[derive(Debug, Error, Clone, PartialEq)]
#[non_exhaustive]
pub enum PolicyError {
    /// A threshold was not in `[0, 1]`.
    #[error("threshold {0} is outside [0, 1]")]
    OutOfUnitInterval(f64),
    /// Act threshold was strictly below the review threshold.
    #[error("act threshold {act} must be >= review threshold {review}")]
    ActBelowReview {
        /// Act threshold.
        act: f64,
        /// Review threshold.
        review: f64,
    },
    /// Review band lower bound exceeded the upper bound.
    #[error("review band {lo}..{hi} is invalid")]
    InvalidReviewBand {
        /// Lower bound.
        lo: f64,
        /// Upper bound.
        hi: f64,
    },
    /// Noul `yes_act` was strictly below `no_act`.
    #[error("yes_act {yes} must be >= no_act {no}")]
    YesBelowNo {
        /// Yes-act threshold.
        yes: f64,
        /// No-act threshold.
        no: f64,
    },
}

fn check_unit(t: f64) -> Result<f64, PolicyError> {
    if (0.0..=1.0).contains(&t) && t.is_finite() {
        Ok(t)
    } else {
        Err(PolicyError::OutOfUnitInterval(t))
    }
}

/// Confidence policy for Choice and Score answers.
///
/// Invariants: `act >= review`; every threshold is in `[0, 1]`.
///
/// # Panics
///
/// The infallible constructors [`Policy::act`], [`Policy::review`], and
/// [`Policy::for_variant`] panic if an invariant is violated. Use the `try_*`
/// methods when thresholds come from configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct Policy<T> {
    act: f64,
    review: f64,
    per_variant: Vec<(T, f64, f64)>,
    signal: Signal,
}

impl<T: Copy + Eq> Policy<T> {
    /// Start a policy with an act threshold. Review defaults to `0.0`.
    ///
    /// # Panics
    ///
    /// Panics if `threshold` is outside `[0, 1]`.
    pub fn act(threshold: f64) -> Self {
        match Self::try_act(threshold) {
            Ok(p) => p,
            Err(e) => panic!("{e}"),
        }
    }

    /// Fallible [`Policy::act`].
    pub fn try_act(threshold: f64) -> Result<Self, PolicyError> {
        Ok(Self {
            act: check_unit(threshold)?,
            review: 0.0,
            per_variant: Vec::new(),
            signal: Signal::Confidence,
        })
    }

    /// Set the review threshold. Must be `<= act`.
    ///
    /// # Panics
    ///
    /// Panics if `threshold` is outside `[0, 1]` or greater than `act`.
    pub fn review(self, threshold: f64) -> Self {
        match self.try_review(threshold) {
            Ok(p) => p,
            Err(e) => panic!("{e}"),
        }
    }

    /// Fallible [`Policy::review`].
    pub fn try_review(mut self, threshold: f64) -> Result<Self, PolicyError> {
        let review = check_unit(threshold)?;
        if self.act < review {
            return Err(PolicyError::ActBelowReview {
                act: self.act,
                review,
            });
        }
        self.review = review;
        Ok(self)
    }

    /// Override thresholds for a high-risk variant.
    ///
    /// # Panics
    ///
    /// Panics if thresholds are outside `[0, 1]` or `act < review`.
    pub fn for_variant(self, v: T, act: f64, review: f64) -> Self {
        match self.try_for_variant(v, act, review) {
            Ok(p) => p,
            Err(e) => panic!("{e}"),
        }
    }

    /// Fallible [`Policy::for_variant`].
    pub fn try_for_variant(mut self, v: T, act: f64, review: f64) -> Result<Self, PolicyError> {
        let act = check_unit(act)?;
        let review = check_unit(review)?;
        if act < review {
            return Err(PolicyError::ActBelowReview { act, review });
        }
        self.per_variant.push((v, act, review));
        Ok(self)
    }

    /// Select the scalar compared to thresholds.
    pub fn using(mut self, signal: Signal) -> Self {
        self.signal = signal;
        self
    }

    /// Act threshold (global, before per-variant overrides).
    pub fn act_threshold(&self) -> f64 {
        self.act
    }

    /// Review threshold (global, before per-variant overrides).
    pub fn review_threshold(&self) -> f64 {
        self.review
    }

    pub(crate) fn signal(&self) -> Signal {
        self.signal
    }

    fn thresholds_for(&self, value: T) -> (f64, f64) {
        self.per_variant
            .iter()
            .find(|(v, _, _)| *v == value)
            .map(|(_, act, review)| (*act, *review))
            .unwrap_or((self.act, self.review))
    }

    pub(crate) fn verdict(&self, signal: f64, value: T) -> Verdict<T> {
        let (act, review) = self.thresholds_for(value);
        if signal >= act {
            Verdict::Act(value)
        } else if signal >= review {
            Verdict::Review(value)
        } else {
            Verdict::Escalate
        }
    }
}

/// Gating policy for a Noul probability.
///
/// * `p >= yes_act` → [`Verdict::Act`]`(true)`
/// * `p <= no_act` → [`Verdict::Act`]`(false)`
/// * `p` inside `review_band` → [`Verdict::Review`] (guess is `p >= 0.5`)
/// * otherwise → [`Verdict::Escalate`]
#[derive(Clone, Debug, PartialEq)]
pub struct NoulPolicy {
    yes_act: f64,
    no_act: f64,
    review_band: (f64, f64),
}

impl NoulPolicy {
    /// Construct a policy.
    ///
    /// # Panics
    ///
    /// Panics if any value is outside `[0, 1]`, if `yes_act < no_act`, or if
    /// `review_lo > review_hi`.
    pub fn new(yes_act: f64, no_act: f64, review_lo: f64, review_hi: f64) -> Self {
        match Self::try_new(yes_act, no_act, review_lo, review_hi) {
            Ok(p) => p,
            Err(e) => panic!("{e}"),
        }
    }

    /// Fallible [`NoulPolicy::new`].
    pub fn try_new(
        yes_act: f64,
        no_act: f64,
        review_lo: f64,
        review_hi: f64,
    ) -> Result<Self, PolicyError> {
        let yes_act = check_unit(yes_act)?;
        let no_act = check_unit(no_act)?;
        let review_lo = check_unit(review_lo)?;
        let review_hi = check_unit(review_hi)?;
        if yes_act < no_act {
            return Err(PolicyError::YesBelowNo {
                yes: yes_act,
                no: no_act,
            });
        }
        if review_lo > review_hi {
            return Err(PolicyError::InvalidReviewBand {
                lo: review_lo,
                hi: review_hi,
            });
        }
        Ok(Self {
            yes_act,
            no_act,
            review_band: (review_lo, review_hi),
        })
    }

    /// Act-yes threshold.
    pub fn yes_act(&self) -> f64 {
        self.yes_act
    }

    /// Act-no threshold.
    pub fn no_act(&self) -> f64 {
        self.no_act
    }

    /// Inclusive review band.
    pub fn review_band(&self) -> (f64, f64) {
        self.review_band
    }

    pub(crate) fn verdict(&self, p: f64) -> Verdict<bool> {
        if p >= self.yes_act {
            Verdict::Act(true)
        } else if p <= self.no_act {
            Verdict::Act(false)
        } else if p >= self.review_band.0 && p <= self.review_band.1 {
            Verdict::Review(p >= 0.5)
        } else {
            Verdict::Escalate
        }
    }
}
