# s1-rs: PRD

**Typed decisions for Rust. Turn Rust enums and structs into System One questions, and get compile-time-checked, confidence-gated answers back.**

| Field | Value |
|---|---|
| Owner | Abdel Bakhta (@AbdelStark) |
| Status | Draft v1.0, 16 Sep 2026 |
| Repo | `github.com/AbdelStark/s1-rs` (Cargo workspace) |
| Crates | `s1`, `s1-derive`, `s1-test`, `s1-cli` (names free on crates.io as of 16 Sep 2026) |
| Target | v0.1 in 3 weeks, v0.2 (production-hardening) in 6 weeks |
| Companion | [`SPEC.md`](./SPEC.md) |
| Transport | [`typesafe-rs`](../typesafe-rs/PRD.md) (default), `typesafe-ai` (feature) |
| Downstream consumers | [`reflex`](../reflex/PRD.md), [`sieve`](../sieve/PRD.md) |

---

## 0. Relationship to typesafe-rs

`s1-rs` is the typed layer; [`typesafe-rs`](../typesafe-rs/PRD.md) is the transport SDK underneath it. They ship as separate crates so each stays focused:

- **typesafe-rs** owns HTTP, retries, errors, configuration, middleware, and backends (including LLM backends and Cascade).
- **s1-rs** owns the type system: derive macros, typed answers, gating policies, and test helpers built on those types.

`s1-rs` depends on `typesafe-rs` by default and also supports the community crate `typesafe-ai` (by Joey, `Twister915`, published 16 Sep 2026) behind the `backend-typesafe-ai` feature. The typed layer never forces a choice of client, which keeps the Rust ecosystem positive-sum.

## 1. Problem

Today every System One integration in every language is stringly typed:

```rust
let req = Request::new(ticket)
    .with_question("department", Question::choice("Which team?", criteria_map));
let dept = resp.answer("department").and_then(|a| a.choice()); // Option<&str>
match dept { Some("billing") => ..., Some("technical") => ..., _ => ... } // typo = silent bug
```

Question keys, option labels, and score levels are strings. Rename a variant and nothing fails to compile. Add an option to the criteria and no `match` warns you. Thresholds for acting on confidence are ad hoc `if` statements scattered through code.

TypeSafe's thesis is **"decisions, not strings"** and **"more like code."** The wire format delivers typed outputs; the developer experience still hands you strings. Rust's type system can close that last mile completely.

## 2. Vision

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

## 3. Why this project

- **It is the literal embodiment of the company name.** A founder sees the vision snippet above and understands the value in five seconds.
- **It is the foundation for Reflex and Sieve.** Both flagship projects dogfood it in production, so the SDK is hardened by real load, not examples.
- **It completes the Rust story.** `typesafe-rs` makes System One reliable and fast from Rust; `s1-rs` makes it type-safe. Together they are the most complete System One developer experience in any language.
- **It creates a pattern TypeSafe can port.** Pydantic-model and Zod-schema equivalents for Python and TypeScript follow naturally. You are proposing the canonical DX, not just a crate.

## 4. Goals

| # | Goal | Done when |
|---|---|---|
| G1 | Derive macros map enums and structs to Choice, Score, Noul questions | `#[derive(Choice)]`, `#[derive(Score)]`, `#[derive(Questions)]` generate wire-exact JSON verified against the API reference |
| G2 | Typed answers with full probability access | `ChoiceAnswer<T>` exposes `value()`, `prob(T)`, `distribution()`, `confidence()`; `ScoreAnswer<T>` exposes expected score and level distribution; `NoulAnswer` exposes `p()` |
| G3 | Confidence gating as a first-class type | `Policy` and `Verdict<T>`; per-variant thresholds; risk-weighted policies |
| G4 | Unknown-label safety | A label the enum does not know never panics; surfaces as `Error::UnknownLabel` with raw response retained |
| G5 | Deterministic testing without the network | `s1-test`: record and replay cassettes, scripted fake client, property-test helpers |
| G6 | Throughput helpers for pipelines | `ask_many` with bounded concurrency, token-bucket rate limiting, backpressure, per-item errors |
| G7 | Observability | `tracing` spans and OpenTelemetry-compatible metrics: latency, retries, confidence histograms, verdict counts |
| G8 | Shell-first CLI | `s1 ask` for scripts and prototyping; JSON Lines in and out |

## 5. Non-goals

- No HTTP client code. Transport, retries, errors, and backends come from `typesafe-rs` (or `typesafe-ai` via feature).
- No LLM fallback implementation inside the crate (a trait hook only).
- No WASM target in v0.1 (evaluate for v0.3).
- No hosted service.

## 6. Users

| User | Job to be done |
|---|---|
| Rust backend engineers | Add semantic branching to services without stringly typed glue |
| Latency-critical systems (games, trading, edge, networking) | Embed decisions in hot paths with predictable overhead |
| Agent and infra tool builders | Build guardrails and routers (Reflex is the reference) |
| Data pipeline engineers | Run typed decisions over streams (Sieve is the reference) |
| TypeSafe team | A DX pattern to replicate in official SDKs |

## 7. Collaboration plan

1. Share the vision snippet publicly as a design issue before v0.1 and invite comments from TypeSafe engineers and Rust users.
2. Tell Joey that `s1-rs` supports `typesafe-ai` as a backend; ask whether a small accessor for per-level Score probabilities could land upstream.
3. Ask TypeSafe (Erik, and `evinism` who maintains the official TypeScript SDK) whether they want the typed-decision pattern ported to official SDKs (Pydantic-style for Python, Zod-style for TypeScript), and whether they object to the `s1` crate prefix. Rename before v0.1 if they object.

## 8. Success criteria

**Technical:** zero `unwrap` in library code; wire-format golden tests pass against recorded live responses; overhead of the typed layer under 50 microseconds per call excluding network (benchmarked).
**Adoption:** Reflex and Sieve run on it in production; at least 3 external repos depend on it within 6 weeks.
**Signal:** TypeSafe acknowledges or ports the pattern; `typesafe-ai` users adopt the typed layer through the feature flag.

## 9. Milestones

| Week | Deliverable |
|---|---|
| 0 | Design doc as a GitHub issue; starts once `typesafe-rs` core types are stable (week 1 of typesafe-rs) |
| 1 | `s1-derive` Choice, Score, Noul with golden JSON tests; `Answers<T>` decoding |
| 2 | `Policy`, `Verdict`; `s1-test` fake client and cassettes; examples |
| 3 | v0.1 release: docs.rs complete, 3 examples, announcement post |
| 4 to 5 | `ask_many`, rate limiting, tracing and metrics, `s1-cli` |
| 6 | v0.2: hardened by Reflex and Sieve production use; benchmarks published |

## 10. Risks

| Risk | Mitigation |
|---|---|
| Two transport backends double test surface | Backend adapter trait is tiny (encode request, decode response); golden tests run against both |
| TypeSafe changes the wire format | Golden tests against live responses in CI (weekly, with a sandbox key); version the derive output |
| API does not return per-level probabilities for Score in all cases | Model as `Option`; document; request field from TypeSafe |
| Brand concerns with naming | Ask before publishing; prefix `s1` avoids the company name |
| Coupled release cadence with typesafe-rs | Depend on a semver range; typed layer only touches stable wire types |
| Proc-macro complexity hurts compile times | Keep macros thin, generate trait impls calling runtime helpers; measure compile-time impact |
| Early-access key sharing in CI | Use recorded cassettes in public CI; live tests only on a private runner |

## 11. Open questions

- Maximum number of questions and maximum Choice cardinality per request (launch post says 255 options).
- Are question-key strings counted as input tokens? (Docs say keys are not sent to the model.)
- Does Score always return per-level `probabilities` alongside `score`, `legend`, `confidence`?
- Is there a model-version pinning mechanism beyond `jev-latest`?
