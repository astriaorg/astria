fn main() {}

fn boxed_trait_object_mentioned() {
    use std::fmt::Write;
    type Writer = Box<dyn Write>;
}

fn fn_returns_boxed_trait_object_mentioned() {
    use std::fmt::Write;

    fn test() -> Box<dyn Write> {
        Box::new(String::new())
    }
}