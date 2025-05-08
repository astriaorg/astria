fn main() {}

mod fn_generic_mentioned {
    use std::fmt::Write;

    fn test<W: Write>(mut out: W) {
        out.write_str("Hello, world!").unwrap();
    }
}

mod fn_generic_where_clause {
    use std::fmt::Write;

    fn test<W>(mut out: W) where W: Write {
        out.write_str("Hello, world!").unwrap();
    }
}