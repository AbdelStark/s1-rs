# Implementation notes (v0.1)

## Trait graph

```
ChoiceQuestion  ─┐
                 ├─► QuestionField  ─►  field of QuestionSet
ScoreQuestion   ─┘
bool + #[s1(noul)]  ─►  Noul question (handled in the Questions derive)

QuestionSet::questions() -> WireQuestions
QuestionSet::decode(&WireResponse) -> Result<Self::Answers, DecodeError>
Answers<Q> = <Q as QuestionSet>::Answers

DecisionBackend::evaluate(WireRequest) -> Future<Result<WireResponse, Error>>
S1<C: DecisionBackend>::ask<Q: QuestionSet>(state) -> Result<Q::Answers, S1Error<C::Error>>
```

`WireRequest` / `WireResponse` / `Question` / `WireAnswer` are owned by `s1`. They mirror the documented TypeSafe JSON (`state`, `model`, `questions` / `model`, `answers`, `usage`, `request_id`). Insertion order is preserved with `IndexMap`.

## Derive output

`#[derive(Choice)]` emits:

- `impl ChoiceQuestion` (`INSTRUCTIONS`, `VARIANTS`, `label`, `description`, `from_label`, `index`)
- `impl QuestionField` with `Answer = ChoiceAnswer<Self>`, delegating wire build to `Question::from_choice::<Self>()` and decode to `ChoiceAnswer::decode`
- `Copy` / `Clone` / `PartialEq` / `Eq` / `Debug` unless the user already derived them

`#[derive(Score)]` is the same shape with `ScoreQuestion` (`LEVELS`, `level_description`, `index`, `from_index`) and `Ord` / `PartialOrd` from declaration order.

`#[derive(Questions)]` emits `{Name}Answers` with one typed field per question plus `meta: AnswerMeta`, and `impl QuestionSet`. Noul fields become `NoulAnswer`. `Option<T>` still sends the question and treats a missing answer as `None`.

Wire fragments:

```json
{ "type": "choice", "instructions": "...", "criteria": { "label": "desc" } }
{ "type": "score",  "instructions": "...", "criteria": ["desc0", "desc1"] }
{ "type": "noul",   "instructions": "..." }
```

## Decoding

- Choice: `choice` must map via `from_label` or `DecodeError::UnknownLabel { key, label }`. Missing probability labels become 0 and increment `meta.normalization_events`. The distribution is renormalized if its sum differs from 1 by more than `1e-6`.
- Score: `score` must lie in `[0, n-1]` (tolerance `1e-6`) or `DecodeError::OutOfRange`.
- Noul: `noul` must lie in `[0, 1]`.
- Missing required answer: `DecodeError::MissingAnswer { key }`.
- `AnswerMeta::raw()` keeps a JSON copy of the `WireResponse` for audit.

## Gating

```
Policy::act(0.85).review(0.6)     // panics on invariant violation; try_* returns Result
signal >= act     → Verdict::Act(value)
signal >= review  → Verdict::Review(value)
else              → Verdict::Escalate
```

Invariants: `act >= review`; thresholds in `[0, 1]`. Per-variant overrides via `for_variant`. Signal is `Confidence` (default), `TopProb`, or `Margin`.

`NoulPolicy::new(yes_act, no_act, review_lo, review_hi)`:

- `p >= yes_act` → `Act(true)`
- `p <= no_act` → `Act(false)`
- inside the review band → `Review(p >= 0.5)`
- else → `Escalate`

`ChoiceAnswer::gate` / `NoulAnswer::gate` / `ScoreAnswer::gate` accept `&Policy` or a value (so the vision snippet's `gate(Policy::act(0.85).review(0.6))` compiles).

## Backends

v0.1 ships `s1-test::FakeClient` only. `backend-typesafe-rs` waits on typesafe-rs v0.1 (see `ROADMAP.md`).
