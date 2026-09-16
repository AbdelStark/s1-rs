//! [`DecisionBackend`] for the published crates.io [`typesafe_rs::Client`].

use std::future::Future;

use crate::backend::DecisionBackend;
use crate::wire::{WireAnswer, WireRequest, WireResponse};

impl DecisionBackend for typesafe_rs::Client {
    type Error = typesafe_rs::Error;

    fn evaluate(
        &self,
        req: WireRequest,
    ) -> impl Future<Output = Result<WireResponse, Self::Error>> + Send {
        let this = self.clone();
        async move {
            let ts_req = to_system_one_request(req)?;
            let resp = this
                .system_one_with(&ts_req, typesafe_rs::CallOptions::default())
                .await?;
            Ok(from_system_one_response(resp))
        }
    }
}

fn to_system_one_request(
    req: WireRequest,
) -> Result<typesafe_rs::SystemOneRequest, typesafe_rs::Error> {
    let value = serde_json::to_value(&req)
        .map_err(|e| typesafe_rs::Error::InvalidRequest(e.to_string()))?;
    serde_json::from_value(value).map_err(|e| typesafe_rs::Error::InvalidRequest(e.to_string()))
}

fn from_system_one_response(resp: typesafe_rs::SystemOneResponse) -> WireResponse {
    WireResponse {
        model: resp.model,
        answers: resp
            .answers
            .into_iter()
            .map(|(key, answer)| (key, from_answer(answer)))
            .collect(),
        usage: resp.usage.map(|u| crate::Usage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
        }),
        request_id: resp.meta.request_id,
    }
}

fn from_answer(answer: typesafe_rs::Answer) -> WireAnswer {
    match answer {
        typesafe_rs::Answer::Noul { noul } => WireAnswer::Noul { noul },
        typesafe_rs::Answer::Choice {
            choice,
            probabilities,
            confidence,
        } => WireAnswer::Choice {
            choice,
            probabilities,
            confidence,
        },
        typesafe_rs::Answer::Score {
            score,
            legend,
            probabilities,
            confidence,
        } => WireAnswer::Score {
            score,
            legend,
            probabilities,
            confidence,
        },
        _ => WireAnswer::Unknown,
    }
}
