// allow: clippy lints that are not ok in production code but acceptable or wanted in tests
#[allow(clippy::missing_panics_doc)]
pub mod helpers;
pub mod soft_only;

use helpers::ROLLUP_ID;
