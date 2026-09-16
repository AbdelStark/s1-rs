use s1::Choice;

#[derive(Choice)]
#[s1(instructions = "only one")]
enum One {
    #[s1("a")]
    A,
}

fn main() {}
