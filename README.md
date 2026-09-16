# s1

Typed [System One](https://docs.typesafe.ai/api) decisions for Rust.

Turn enums and structs into Choice, Score, and Noul questions. Get compile-time-checked, confidence-gated answers back.

[![CI](https://img.shields.io/github/actions/workflow/status/AbdelStark/s1-rs/ci.yml?branch=main&style=for-the-badge&logo=githubactions&logoColor=white&label=CI)](https://github.com/AbdelStark/s1-rs/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.85+-informational?style=for-the-badge&logo=rust&logoColor=white)](https://blog.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=for-the-badge)](LICENSE)

```rust
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
    #[s1("Calm, just stating facts")] Calm,
    #[s1("Frustrated but civil")]     Annoyed,
    #[s1("Angry, strong language")]   Furious,
}

#[derive(Questions)]
struct Triage {
    department: Department,
    frustration: Frustration,
    #[s1(noul = "The message conveys urgency or time pressure")]
    urgent: bool,
}

let t: Answers<Triage> = s1.ask::<Triage>(&ticket).await?;

match t.department.gate(Policy::act(0.85).review(0.6)) {
    Verdict::Act(Department::Billing) => billing_queue.push(ticket),
    Verdict::Act(Department::Technical) => eng_queue.push(ticket),
    Verdict::Act(Department::Sales) => sales_queue.push(ticket),
    Verdict::Review(guess) => human_queue.push_with_hint(ticket, guess),
    Verdict::Escalate => llm_fallback(ticket).await?,
}
```

Add a `Department::Legal` variant and the `match` stops compiling until you handle it.

A complete, network-free version of this snippet lives in [`examples/triage.rs`](./examples/triage.rs) and runs against `s1-test::FakeClient`.

## Install

The typed layer is this git workspace (`s1`, `s1-derive`, `s1-test`). HTTP transport is the published [`typesafe-rs`](https://crates.io/crates/typesafe-rs) crate.

```toml
[dependencies]
s1 = { git = "https://github.com/AbdelStark/s1-rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

[dev-dependencies]
s1-test = { git = "https://github.com/AbdelStark/s1-rs" }
```

From a checkout of this repo:

```toml
s1 = { path = "crates/s1" }
s1-test = { path = "crates/s1-test" }
```

MSRV is **1.85** (edition 2024).

## Quick start

Script answers in process. No network, no API key.

```rust
use s1::{Choice, Policy, Questions, S1, Score, Verdict};
use s1_test::{FakeClient, Script};

let fake = FakeClient::new().on::<Triage>(|_state| {
    Script::new()
        .choice(Department::Billing, 0.91)
        .score::<Frustration>(1.2)
        .noul("urgent", 0.97)
});

let s1 = S1::new(fake);
let t = s1.ask::<Triage>(&ticket).await?;

assert_eq!(
    t.department.gate(Policy::act(0.85).review(0.6)),
    Verdict::Act(Department::Billing)
);
assert_eq!(t.frustration.nearest(), Frustration::Annoyed);
assert!((t.urgent.p() - 0.97).abs() < 1e-9);
```

Handlers match on the question-key set of `Triage::questions()`. Tests go through `QuestionSet::decode` / `S1::ask`, not a hand-rolled decoder.

## HTTP via `typesafe-rs`

Enable feature `backend-typesafe-rs` so the published [`typesafe_rs::Client`](https://docs.rs/typesafe-rs) implements `DecisionBackend`. This crate does not speak HTTP.

```toml
[dependencies]
s1 = { git = "https://github.com/AbdelStark/s1-rs", features = ["backend-typesafe-rs"] }
typesafe-rs = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use s1::S1;
use typesafe_rs::Client;

let client = Client::from_env()?;
let s1 = S1::new(client);
let t = s1.ask::<Triage>(&ticket).await?;
```

`Client::from_env()` reads `TYPESAFE_API_KEY` (required) and optional `TYPESAFE_BASE_URL` / `TYPESAFE_DEFAULT_MODEL`. Retries, timeouts, and identification headers live in `typesafe-rs`.

Point the same `Client` at a loopback server in tests, or use `FakeClient` and skip HTTP entirely.

## Confidence gating

`Policy` maps a scalar (default: API `confidence`) onto `Verdict::{Act, Review, Escalate}`.

| Signal vs thresholds | Outcome |
|---|---|
| `>= act` | `Verdict::Act(value)` — take the typed answer |
| `>= review` | `Verdict::Review(value)` — keep a guess for a human |
| else | `Verdict::Escalate` — fall back |

```rust
let policy = Policy::act(0.85)
    .review(0.6)
    .for_variant(Department::Billing, 0.9, 0.7);

match t.department.gate(&policy) {
    Verdict::Act(dept) => route(dept),
    Verdict::Review(guess) => review_queue.push(guess),
    Verdict::Escalate => fallback().await?,
}
```

Invariants: `act >= review`; every threshold is in `[0, 1]`. Infallible constructors panic on violation; `try_act` / `try_review` / `try_for_variant` return `PolicyError`.

Noul (yes/no) uses `NoulPolicy`:

```rust
match t.urgent.gate(NoulPolicy::new(0.9, 0.15, 0.35, 0.65)) {
    Verdict::Act(true) => page_oncall(),
    Verdict::Act(false) => (),
    Verdict::Review(_) => human_queue.push(ticket),
    Verdict::Escalate => fallback().await?,
}
```

`ScoreAnswer::nearest()` is the closest level to the expected score; `ScoreAnswer::expected()` is the probability-weighted index.

## Derives

| Derive | On | Wire |
|---|---|---|
| `#[derive(Choice)]` | Fieldless enum, 2..=255 variants | `{ "type": "choice", "criteria": { "label": "desc" } }` |
| `#[derive(Score)]` | Fieldless enum; declaration order is the level index | `{ "type": "score", "criteria": ["desc0", "desc1"] }` |
| `#[derive(Questions)]` | Named struct of Choice, Score, or `bool` + `#[s1(noul = "...")]` | One question per field; generates `{Name}Answers` |

- Default Choice labels are `snake_case`. Override with `#[s1(label = "...")]`.
- Every Choice/Score variant needs `#[s1("description")]` unless `#[s1(allow_empty_descriptions)]`.
- `#[s1(key = "...")]` on a struct field overrides the wire question id.
- `#[s1(answers = "...")]` names the generated answers struct.
- If you do not derive `Copy` / `Eq` / `Debug` (and `Ord` for Score), the macro emits them.

Unknown Choice labels decode as `DecodeError::UnknownLabel`. Out-of-range scores and noul values are `DecodeError::OutOfRange`. Missing answers are `DecodeError::MissingAnswer`.

## Feature flags

| Feature | Default | Notes |
|---|---|---|
| *(none)* | — | Traits, derives, answers, `Policy` / `Verdict`, `S1::ask` |
| `backend-typesafe-rs` | no | `impl DecisionBackend for typesafe_rs::Client` via crates.io [`typesafe-rs`](https://crates.io/crates/typesafe-rs) **0.1** |

## Examples

Both examples use `FakeClient`. There is no network call.

```bash
cargo test --workspace
cargo run --example triage
cargo run --example moderation
```

| Example | What it prints |
|---|---|
| [`triage`](./examples/triage.rs) | Routes a payouts ticket to the billing queue |
| [`moderation`](./examples/moderation.rs) | Sends a borderline post to the moderator queue (`Quarantine`) |

HTTP against a real `typesafe_rs::Client` is covered by `crates/s1/tests/typesafe_rs_backend.rs` (loopback, not the live API):

```bash
cargo test -p s1 --features backend-typesafe-rs --test typesafe_rs_backend
```

## Crates

| Crate | Role |
|---|---|
| `s1` | Runtime: traits, typed answers, `Policy` / `Verdict`, `S1<C: DecisionBackend>` |
| `s1-derive` | `#[derive(Choice)]`, `#[derive(Score)]`, `#[derive(Questions)]` |
| `s1-test` | Scriptable `FakeClient` (no network) |
| [`typesafe-rs`](https://crates.io/crates/typesafe-rs) | HTTP client, retries, errors, configuration (separate crate) |

`#![forbid(unsafe_code)]` on every crate in this workspace.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT](LICENSE) at your option.

See [CHANGELOG](./CHANGELOG.md), [SPEC](./SPEC.md), and [ROADMAP](./ROADMAP.md).
