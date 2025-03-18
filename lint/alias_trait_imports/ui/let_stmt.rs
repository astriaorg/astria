fn main() {}

fn let_stmt_should_fail() {
    use std::fmt::Write;

    let _ = {
        let mut out_string = String::new();
        out_string.write_str("Hello, world!");
    };
}

fn let_stmt_mentioned() {
    use std::fmt::Write;

    let _ = {
        let mut out_string = String::new();
        Write::write_str(&mut out_string, "Hello, world!").unwrap();
    };
}