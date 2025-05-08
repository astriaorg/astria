use std::fmt::Write;

fn main() {}

fn impl_in_fn_sig() -> impl Write {
    let mut s = String::new();
    s.write_str("Hello, world!").unwrap();
    s
}
