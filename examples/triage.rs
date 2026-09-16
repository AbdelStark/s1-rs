//! Support-ticket triage against `s1-test::FakeClient` (no network).

use s1::{Choice, Policy, Questions, S1, Score, Verdict};
use s1_test::{FakeClient, Script};

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ticket = "Help! My payouts have been failing for 3 days.";

    let fake = FakeClient::new().on::<Triage>(|_state| {
        Script::new()
            .choice(Department::Billing, 0.91)
            .score::<Frustration>(1.2)
            .noul("urgent", 0.97)
    });

    let s1 = S1::new(fake);
    let t = s1.ask::<Triage>(&ticket).await?;

    match t.department.gate(Policy::act(0.85).review(0.6)) {
        Verdict::Act(Department::Billing) => println!("billing queue"),
        Verdict::Act(Department::Technical) => println!("eng queue"),
        Verdict::Act(Department::Sales) => println!("sales queue"),
        Verdict::Review(guess) => println!("human review, hint={guess:?}"),
        Verdict::Escalate => println!("escalate to LLM"),
    }

    println!(
        "frustration nearest={:?} expected={:.2}",
        t.frustration.nearest(),
        t.frustration.expected()
    );
    println!("urgent p={:.2}", t.urgent.p());
    Ok(())
}
