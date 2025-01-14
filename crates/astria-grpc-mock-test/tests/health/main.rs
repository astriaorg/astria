#![expect(
    clippy::should_panic_without_expect,
    reason = "just make the tests work for now"
)]

mod matcher;
mod mock;
mod response;
mod utils;
