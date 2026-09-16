//! Content-moderation example against `s1-test::FakeClient` (no network).

use s1::{Choice, NoulPolicy, Policy, Questions, S1, Score, Verdict};
use s1_test::{FakeClient, Script};

#[derive(Choice)]
#[s1(instructions = "What is the appropriate moderation action?")]
enum Action {
    #[s1("Content is fine to publish")]
    Allow,
    #[s1("Content should be hidden pending review")]
    Quarantine,
    #[s1("Content violates policy and must be removed")]
    Remove,
}

#[derive(Score)]
#[s1(instructions = "How severe is the policy concern?")]
enum Severity {
    #[s1("No issue")]
    Clean,
    #[s1("Borderline or mildly concerning")]
    Mild,
    #[s1("Clear and serious violation")]
    Severe,
}

#[derive(Questions)]
#[allow(dead_code)]
struct Moderation {
    action: Action,
    severity: Severity,
    #[s1(noul = "The content is directed at a private individual")]
    personal: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let post = "This is a mildly rude joke about a public figure.";

    let fake = FakeClient::new().on::<Moderation>(|_state| {
        Script::new()
            .choice(Action::Quarantine, 0.72)
            .score::<Severity>(1.1)
            .noul("personal", 0.12)
    });

    let s1 = S1::new(fake);
    let m = s1.ask::<Moderation>(&post).await?;

    match m.action.gate(Policy::act(0.85).review(0.55)) {
        Verdict::Act(Action::Allow) => println!("publish"),
        Verdict::Act(Action::Quarantine) => println!("hide pending review"),
        Verdict::Act(Action::Remove) => println!("remove"),
        Verdict::Review(guess) => println!("moderator queue, hint={guess:?}"),
        Verdict::Escalate => println!("escalate"),
    }

    println!(
        "severity nearest={:?} expected={:.2}",
        m.severity.nearest(),
        m.severity.expected()
    );

    match m.personal.gate(NoulPolicy::new(0.9, 0.15, 0.35, 0.65)) {
        Verdict::Act(true) => println!("flagged as personal"),
        Verdict::Act(false) => println!("not personal"),
        Verdict::Review(_) => println!("review personal-target"),
        Verdict::Escalate => println!("unsure whether personal"),
    }
    Ok(())
}
