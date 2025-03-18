fn main() {}

mod struct_generic_mentioned {
    use std::fmt::Write;

    struct Inner<W: Write> {
        out: W,
    }
}

mod struct_generic_where_clause {
    use std::fmt::Write;

    struct Inner<W> where W: Write {
        out: W,
    }
}