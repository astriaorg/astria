fn main() {}

mod container {
    use std::fmt::Write;
    use super::other_container::convert;

    enum MyEnum {
        A,
    }

    impl MyEnum {
        fn iter_trait_object(input: &Vec<String>) -> impl Iterator<Item = &'_ dyn Write> {
            input.iter().map(convert)
        }
    }
}

pub mod other_container {
    use std::fmt::Write;

    pub fn convert(input: &String) -> &'_ dyn Write {
        input as &dyn Write
    }
}
