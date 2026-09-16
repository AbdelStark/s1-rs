use s1::Choice;

#[derive(Choice)]
#[s1(instructions = "pick one")]
enum Dup {
    #[s1(label = "same", desc = "a")]
    A,
    #[s1(label = "same", desc = "b")]
    B,
}

fn main() {}
