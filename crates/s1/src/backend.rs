//! [`DecisionBackend`] and the [`S1`] typed client.

use serde::Serialize;
use std::future::Future;

use crate::decode::DecodeError;
use crate::traits::QuestionSet;
use crate::wire::{WireRequest, WireResponse};

/// Default model sent when [`S1::with_model`] is not used.
pub const DEFAULT_MODEL: &str = "jev-latest";

/// Minimal transport the typed layer needs.
///
/// Implemented by `s1-test::FakeClient`. Enable the `backend-typesafe-rs`
/// feature to use the published [`typesafe_rs::Client`](https://docs.rs/typesafe-rs/latest/typesafe_rs/struct.Client.html)
/// as a backend.
pub trait DecisionBackend: Send + Sync {
    /// Backend failure (network, HTTP, fake-script miss, …).
    type Error: std::error::Error + Send + Sync + 'static;

    /// Evaluate `req` and return a wire response.
    fn evaluate(
        &self,
        req: WireRequest,
    ) -> impl Future<Output = Result<WireResponse, Self::Error>> + Send;
}

/// Typed System One client over a [`DecisionBackend`].
#[derive(Clone, Debug)]
pub struct S1<C> {
    client: C,
    model: Option<String>,
}

impl<C> S1<C> {
    /// Wrap a backend.
    pub fn new(client: C) -> Self {
        Self {
            client,
            model: None,
        }
    }

    /// Set the model name (default [`DEFAULT_MODEL`]).
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Model that will be sent on the next request.
    pub fn model(&self) -> &str {
        self.model.as_deref().unwrap_or(DEFAULT_MODEL)
    }

    /// Borrow the inner backend.
    pub fn client(&self) -> &C {
        &self.client
    }
}

impl<C: DecisionBackend> S1<C> {
    /// One state, one question set, one backend call.
    pub async fn ask<Q: QuestionSet>(
        &self,
        state: impl Serialize,
    ) -> Result<Q::Answers, S1Error<C::Error>> {
        let req = WireRequest {
            state: serde_json::to_value(&state)?,
            model: self.model().to_string(),
            questions: Q::questions(),
        };
        let resp = self.client.evaluate(req).await.map_err(S1Error::Backend)?;
        Q::decode(&resp).map_err(S1Error::Decode)
    }
}

/// Error from [`S1::ask`].
#[derive(Debug, thiserror::Error)]
pub enum S1Error<E> {
    /// The backend failed.
    #[error("backend error: {0}")]
    Backend(E),
    /// The response could not be decoded into the question set's answers.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// `state` could not be serialized to JSON.
    #[error("failed to serialize state: {0}")]
    Serialize(#[from] serde_json::Error),
}
