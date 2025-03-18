fn main() {
    use std::fmt::Write;

    trait MyTrait: Write {
        fn write_hello(&mut self) {
            self.write_str("Hello, world!").unwrap();
        }
    }
}