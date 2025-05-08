fn main() {}

mod struct_impl_should_fail {
    use std::fmt::Write;

    struct Inner {
        out: String,
    }

    impl Inner {
        fn write_hello(&mut self) {
            self.out.write_str("Hello, world!").unwrap();
        }
    }
}

mod struct_impl_mentioned {
    use std::fmt::Write;

    struct Inner {
        out: String,
    }

    impl Inner {
        fn write_hello(&mut self) {
            Write::write_str(&mut self.out, "Hello, world!").unwrap();
        }
    }
}