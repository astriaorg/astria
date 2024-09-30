#[expect(clippy::all)]
use tracing::info;

#[expect(
    dead_code,
    reason = "the type and its inner value are used by dylint to test generated code and have no \
              further meaning beyond that"
)]
#[derive(Clone, Copy, Debug)]
struct Wrapped(&'static str);

fn main() {
    let field = Wrapped("wrapped");
    info!(field = tracing::field::debug(field), "using the function");
    info!(field = ?field, "using the sigil");
    info!(?field, "using shorthand with sigil");
}
