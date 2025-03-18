// edition:2021

fn main() {
}

mod type_cast_as_trait {
    use std::fmt::Write;

    trait MyTrait {
        async fn write_hello(&mut self);
    }

    impl MyTrait for String {
        async fn write_hello(&mut self) {
            <Self as Write>::write_str(self, "Hello, world!").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}