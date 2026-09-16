# s1-rs

**Typed decisions for Rust.** Turn Rust enums and structs into System One questions, and get compile-time-checked, confidence-gated answers back.

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

Add a `Department::Legal` variant and the `match` stops compiling until you handle it. That is what "type-safe AI" should mean.

A complete, network-free version of this snippet lives in [`examples/triage.rs`](./examples/triage.rs) and runs against `s1-test::FakeClient`.

## Crates

| Crate | Role |
|---|---|
| `s1` | Runtime: traits, answers, `Policy` / `Verdict`, `S1<C: DecisionBackend>` |
| `s1-derive` | `#[derive(Choice)]`, `#[derive(Score)]`, `#[derive(Questions)]` |
| `s1-test` | Scriptable `FakeClient` (no network) |

Transport (HTTP, retries, config) belongs in [`typesafe-rs`](https://github.com/AbdelStark/typesafe-rs). This crate does not implement HTTP.

## Try it

```bash
cargo test --workspace
cargo run --example triage
cargo run --example moderation
```

Both examples talk to `FakeClient`. There is no network call.

## License

MIT OR Apache-2.0

- [PRD](./PRD.md)
- [SPEC](./SPEC.md)
- [Implementation notes](./docs/IMPLEMENTATION.md)
- [Roadmap](./ROADMAP.md)
