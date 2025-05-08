fn main() {}

fn macro_expansion_should_fail() {
    use std::fmt::Write;

    macro_rules! write_hello {
        ($out:expr) => {
            $out.write_str("Hello, world!").unwrap();
        };
    }

    let mut out_string = String::new();
    write_hello!(out_string);
}

fn macro_expansion_mentioned() {
    use std::fmt::Write;

    macro_rules! write_hello {
        ($out:expr) => {
            Write::write_str(&mut $out, "Hello, world!").unwrap();
        };
    }

    let mut out_string = String::new();
    write_hello!(out_string);
}