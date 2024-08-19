// Required to force the benchmark target to actually register the divan benchmark cases.
use astria_sequencer as _;

fn main() {
    // Handle `nextest` querying the benchmark binary for tests.  Currently `divan` is incompatible
    // with `nextest`, so just report no tests available.
    // See https://github.com/nvzqz/divan/issues/43 for further details.
    let args: Vec<_> = std::env::args().collect();
    if args.contains(&"--list".to_string())
        && args.contains(&"--format".to_string())
        && args.contains(&"terse".to_string())
    {
        return;
    }
    // Run registered benchmarks.
    divan::main();
}
