use tracing::info;

#[derive(Debug)]
struct Wrapped(&'static str);

fn main() {
    let val = Wrapped("wrapped");
    info!(field = tracing::field::debug(val), "using the function");
    info!(field = ?Wrapped("wrapped"), "using the sigil");
}
