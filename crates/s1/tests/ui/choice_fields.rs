use s1::Choice;

#[derive(Choice)]
#[s1(instructions = "pick one")]
enum Bad {
    #[s1("has a field")]
    A(u8),
    #[s1("ok")]
    B,
}

fn main() {}
