# Changelog

## 0.1.0 — 2026-09-16

Initial MVP.

- `#[derive(Choice)]`, `#[derive(Score)]`, `#[derive(Questions)]`
- Typed `ChoiceAnswer`, `ScoreAnswer`, `NoulAnswer`
- `Policy` / `NoulPolicy` / `Verdict::{Act, Review, Escalate}`
- `S1<C: DecisionBackend>::ask`
- `s1-test::FakeClient`
- Golden JSON tests and trybuild UI tests
- Examples: `triage`, `moderation`
