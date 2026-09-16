//! Proc macros for `s1`: `Choice`, `Score`, and `Questions`.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;

mod attr;
mod choice;
mod questions;
mod rename;
mod score;
mod util;

/// Derive a System One Choice question from a fieldless enum.
///
/// ```ignore
/// #[derive(Choice)]
/// #[s1(instructions = "Which team should handle this ticket?")]
/// enum Department {
///     #[s1("Payment issues")]
///     Billing,
///     #[s1(label = "tech", desc = "Bugs")]
///     Technical,
/// }
/// ```
#[proc_macro_derive(Choice, attributes(s1))]
pub fn derive_choice(input: TokenStream) -> TokenStream {
    choice::derive(input)
}

/// Derive a System One Score question from a fieldless enum.
///
/// Declaration order is the wire order of `criteria` (level index 0..n-1).
#[proc_macro_derive(Score, attributes(s1))]
pub fn derive_score(input: TokenStream) -> TokenStream {
    score::derive(input)
}

/// Derive a question set from a struct of Choice, Score, and Noul fields.
///
/// ```ignore
/// #[derive(Questions)]
/// struct Triage {
///     department: Department,
///     frustration: Frustration,
///     #[s1(noul = "The message conveys urgency")]
///     urgent: bool,
/// }
/// ```
#[proc_macro_derive(Questions, attributes(s1))]
pub fn derive_questions(input: TokenStream) -> TokenStream {
    questions::derive(input)
}
