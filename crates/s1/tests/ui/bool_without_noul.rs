use s1::{Choice, Questions};

#[derive(Choice)]
#[s1(instructions = "pick")]
enum Side {
    #[s1("L")]
    Left,
    #[s1("R")]
    Right,
}

#[derive(Questions)]
struct Bad {
    side: Side,
    flag: bool,
}

fn main() {}
