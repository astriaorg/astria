# Astria Codebase Guidelines for Claude

## Build & Test Commands
- Build: `cargo build --release`
- Lint: `just lint` (or `just lint rust` for Rust only)
- Format: `just fmt` (or `just fmt rust` for Rust only)
- Run all tests: `cargo test`
- Run specific test: `cargo test <test_name>`
- Run crate tests: `cargo test -p <crate_name>`

## Code Style
- **Imports**: Vertical layout, grouped (std → external → project)
- **Formatting**: 100-char line wrap, Unix newlines
- **Types**: Rust 2021 edition with strong type safety
- **Naming**: `snake_case` functions/vars, `CamelCase` types/traits, `SCREAMING_SNAKE_CASE` constants
- **Error Handling**: `thiserror` for error definitions, `Result` types for propagation

## Project Structure
- Workspace with multiple crates in `crates/`
- Protocol definitions in `proto/`
- Primary components: sequencer, conductor, bridges, relayers
- Configuration managed through environment variables and config files

## Documentation
- Each crate has a README.md explaining its purpose
- Detailed in-code documentation adheres to rustdoc standards
- System specifications in `specs/` directory