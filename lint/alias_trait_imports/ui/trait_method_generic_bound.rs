fn main() {
    use std::fmt::Write;

    trait MyTrait {
        fn write_hello<W: Write>(mut out: W) {
            out.write_str("Hello, world!").unwrap();
        }
    }
}