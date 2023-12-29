use tracing::info;

#[derive(Clone, Copy, Debug)]
struct Wrapped(&'static str);

fn main() {
    let field = Wrapped("wrapped");
    info!(field = tracing::field::debug(field), "using the function");
    info!(field = ?field, "using the sigil");
    info!(?field, "using shorthand with sigil");
}
