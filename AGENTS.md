# s1-rs

Community project, private until launch.

Read `PRD.md` and `SPEC.md` before writing code. Implementation happens in this repository.

Do not add TypeSafe job-hunt or campaign material to this repo.

## Crate map

| Crate | Path | What it is |
|---|---|---|
| `s1` | `crates/s1` | Runtime: `ChoiceQuestion` / `ScoreQuestion` / `QuestionSet` / `DecisionBackend`, typed answers, `Policy` / `Verdict`, `S1::ask` |
| `s1-derive` | `crates/s1-derive` | Proc macros. Generate thin trait impls that call helpers in `s1`. |
| `s1-test` | `crates/s1-test` | `FakeClient` + `Script`. Default test backend. |

There is no `s1-cli` in v0.1 (planned for v0.2). There is no HTTP client in this repo.

## Commands

```bash
cargo test --workspace          # no network; uses FakeClient
cargo test -p s1 --features backend-typesafe-rs   # HTTP via crates.io typesafe-rs + loopback stub
cargo run --example triage      # FakeClient, no network
cargo run --example moderation
cargo test -p s1 --test ui      # trybuild compile-fail
```

The workspace `default-members` is `crates/s1`, so `cargo run --example triage` from the repo root selects the `s1` package.

MSRV: 1.85 (edition 2024). Toolchain: stable (`rust-toolchain.toml`).

## How derives work

- `#[derive(Choice)]` on a fieldless enum (2..=255 variants) impls `ChoiceQuestion` + `QuestionField`. Default labels are `snake_case`. Override with `#[s1(label = "...")]`. Every variant needs `#[s1("description")]` unless `allow_empty_descriptions`.
- `#[derive(Score)]` on a fieldless enum. Declaration order is the Score level index. `criteria` on the wire is an array of descriptions.
- `#[derive(Questions)]` on a named struct. Fields are Choice/Score types or `bool` with `#[s1(noul = "...")]`. Optional `key =` overrides the wire question id. Generates `{Name}Answers` (or `#[s1(answers = "...")]`) and `impl QuestionSet`.
- Generated code paths through `::s1::...`. Keep macros thin; decoding and wire building live in `s1`.

If the user does not also derive `Copy` / `Eq` / `Debug` (and `Ord` for Score), the macro emits those impls so the vision snippet compiles as written.

## FakeClient

```rust
let fake = FakeClient::new()
    .on::<Triage>(|state| Script::new()
        .choice(Department::Billing, 0.91)
        .score::<Frustration>(1.2)
        .noul("urgent", 0.97));
let s1 = S1::new(fake);
let t = s1.ask::<Triage>(&ticket).await?;
```

Handlers match on the question-key set of `Q::questions()`. Tests must go through `QuestionSet::decode` / `S1::ask`, not a hand-rolled decoder. Golden JSON tests serialize `T::questions()` from the real derives.

## v0.1 vs v0.2

**v0.1 (this tree):** derives, answers, gating, FakeClient, golden + trybuild tests, two examples.

**Not in v0.1:** `s1-cli`, `ask_many` / pipeline, metrics / OTEL, live cassettes, WASM, `backend-typesafe-ai`.

**`backend-typesafe-rs`:** optional. Depends on crates.io `typesafe-rs` 0.1. `impl DecisionBackend for typesafe_rs::Client`. Integration test `typesafe_rs_backend` talks HTTP on loopback; it does not call the live API.

See `ROADMAP.md`.
