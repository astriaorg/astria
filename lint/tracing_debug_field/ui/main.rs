use tracing::info;

#[derive(Clone, Copy, Debug)]
struct Wrapped(&'static str);

fn main() {
    let val = Wrapped("wrapped");
    info!(field = tracing::field::debug(val), "using the function");
    info!(field = ?val, "using the sigil");
}
