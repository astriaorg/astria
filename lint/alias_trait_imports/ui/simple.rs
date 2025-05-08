fn main() {}

fn simple_test_should_fail() {
    use std::fmt::Write;

    let mut out_string = String::new();
    out_string.write_str("Hello, world!");
}

fn simple_test_mentioned() {
    use std::fmt::Write;
    
    let mut out_string = String::new();
    Write::write_str(&mut out_string, "Hello, world!").unwrap();
}