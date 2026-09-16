#![allow(dead_code)]

use s1::{Choice, Questions, Score};

#[derive(Choice)]
#[s1(instructions = "Which team should handle this ticket?")]
pub enum Department {
    #[s1("Payment, invoice, refund, or subscription issues")]
    Billing,
    #[s1("Bugs, errors, or integration problems")]
    Technical,
    #[s1("Pricing or plan questions")]
    Sales,
}

#[derive(Score)]
#[s1(instructions = "How frustrated does the customer appear?")]
pub enum Frustration {
    #[s1("Calm, just stating facts")]
    Calm,
    #[s1("Frustrated but civil")]
    Annoyed,
    #[s1("Angry, strong language")]
    Furious,
}

#[derive(Questions)]
pub struct Triage {
    pub department: Department,
    pub frustration: Frustration,
    #[s1(noul = "The message conveys urgency or time pressure")]
    pub urgent: bool,
}
