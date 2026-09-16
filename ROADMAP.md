# Roadmap

## v0.1 shipped (MVP)

Typed layer only. No HTTP.

- Cargo workspace, edition 2024, MSRV 1.85: `s1`, `s1-derive`, `s1-test`
- `#[derive(Choice)]`, `#[derive(Score)]`, `#[derive(Questions)]`
- Wire JSON matches TypeSafe Choice / Score / Noul
- `ChoiceAnswer` / `ScoreAnswer` / `NoulAnswer` with `value` / `prob` / `distribution` / `confidence` / `expected` / `nearest` / `p()`
- Unknown Choice label → `DecodeError::UnknownLabel` (no panic)
- `Policy` / `Verdict::{Act, Review, Escalate}` and `NoulPolicy`
- `S1<C: DecisionBackend>::ask`
- `s1-test::FakeClient`
- Golden tests + trybuild UI tests
- Examples: `triage`, `moderation` (FakeClient, no network)
- Dual license MIT OR Apache-2.0
- **`backend-typesafe-rs`** — `impl DecisionBackend for typesafe_rs::Client` (crates.io `typesafe-rs` 0.1). Default `cargo test --workspace` stays FakeClient-only.

## Next

1. **`ask_many` / pipeline** — bounded concurrency over many states (PRD G6). Prefer delegating rate limits to typesafe-rs tower layers.
3. **`s1-cli`** — `s1 ask` / `s1 map` / `s1 gate` / `s1 replay` (PRD G8, SPEC §10).
4. **`backend-typesafe-ai`** — optional adapter for Joey's `typesafe-ai` crate.
5. **Cassettes** — `Cassette::record` / `replay` against a real backend, sanitized.
6. **Observability** — `tracing` spans on `s1.ask`, optional `metrics` histograms (PRD G7).
7. **`#[s1(flatten)]`** and richer structured `instructions_json` / `desc_json` coverage.

`backend-typesafe-ai` and live Jev cassettes are explicitly out of this MVP.
