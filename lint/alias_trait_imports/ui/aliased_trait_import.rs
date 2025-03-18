fn main() {
    use std::fmt::Write as WriteTrait;
    
    let mut out_string = String::new();
    WriteTrait::write_str(&mut out_string, "Hello, world!").unwrap();
}
