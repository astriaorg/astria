fn main() {}

fn closure_should_fail() {
    use std::fmt::Write;

    let mut out_string = String::new();
    let _ = move |mut out_string: String| {
        out_string.write_str("Hello, world!").unwrap();
    };
}

fn closure_mentioned() {
    use std::fmt::Write;

    let mut out_string = String::new();
    let _ = move |mut out_string: String| {
        Write::write_str(&mut out_string, "Hello, world!").unwrap();
    };
}
