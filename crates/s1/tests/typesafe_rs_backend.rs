//! Drive `S1<typesafe_rs::Client>` through a real HTTP socket — no live API.

#![cfg(feature = "backend-typesafe-rs")]

use s1::{Choice, Policy, Questions, S1, Score, Verdict};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use typesafe_rs::{Client, ClientConfig, RetryPolicy, Url};

#[derive(Choice)]
#[s1(instructions = "Which team should handle this ticket?")]
enum Department {
    #[s1("Payment, invoice, refund, or subscription issues")]
    Billing,
    #[s1("Bugs, errors, or integration problems")]
    Technical,
    #[s1("Pricing or plan questions")]
    Sales,
}

#[derive(Score)]
#[s1(instructions = "How frustrated does the customer appear?")]
enum Frustration {
    #[s1("Calm, just stating facts")]
    Calm,
    #[s1("Frustrated but civil")]
    Annoyed,
    #[s1("Angry, strong language")]
    Furious,
}

#[derive(Questions)]
#[allow(dead_code)]
struct Triage {
    department: Department,
    frustration: Frustration,
    #[s1(noul = "The message conveys urgency or time pressure")]
    urgent: bool,
}

async fn serve_json(body: serde_json::Value) -> Url {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let bytes = serde_json::to_vec(&body).expect("json");
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0_u8; 8192];
        let _ = stream.read(&mut buf).await;
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        );
        stream.write_all(header.as_bytes()).await.expect("hdr");
        stream.write_all(&bytes).await.expect("body");
    });
    Url::parse(&format!("http://{addr}")).expect("url")
}

#[tokio::test]
async fn ask_triage_through_real_client_and_http() {
    let url = serve_json(json!({
        "model": "jev-latest",
        "answers": {
            "department": {
                "type": "choice",
                "choice": "billing",
                "probabilities": {
                    "billing": 0.91,
                    "technical": 0.05,
                    "sales": 0.04
                },
                "confidence": 0.91
            },
            "frustration": {
                "type": "score",
                "score": 1.2,
                "legend": {
                    "0": "Calm, just stating facts",
                    "1": "Frustrated but civil",
                    "2": "Angry, strong language"
                },
                "probabilities": { "0": 1.0e-1, "1": 0.6, "2": 0.3 },
                "confidence": 0.8
            },
            "urgent": { "type": "noul", "noul": 0.97 }
        },
        "usage": { "input_tokens": 10, "output_tokens": 3 }
    }))
    .await;

    let client = Client::new(ClientConfig {
        api_key: Some("test".into()),
        base_url: Some(url),
        default_model: Some("jev-latest".into()),
        retry: RetryPolicy::none(),
        ..ClientConfig::default()
    })
    .expect("client");

    let s1 = S1::new(client);
    let t = s1
        .ask::<Triage>(&"Help! My payouts have been failing for 3 days.")
        .await
        .expect("ask");

    assert_eq!(
        t.department.gate(Policy::act(0.85).review(0.6)),
        Verdict::Act(Department::Billing)
    );
    assert_eq!(t.frustration.nearest(), Frustration::Annoyed);
    assert!((t.urgent.p() - 0.97).abs() < 1e-9);
}
