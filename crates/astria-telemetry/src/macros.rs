// re-export so that they need not be imported by downstream users.
// hidden because they shouldn't be imported.
#[doc(hidden)]
pub use const_format::{
    concatcp as __concatcp,
    map_ascii_case as __map_ascii_case,
    Case as __Case,
};

/// Declare a `const` string slice, using the declaring crate's name as a
/// prefix and the variable name as a suffix.
///
/// This macro essentially performs this declaration:
/// ```text
/// METRIC_NAME := ${CARGO_CRATE_NAME}_metric_name;
/// ```
///
/// The purpose of this macro is to avoid accidental typos, ensuring that the
/// metric name matches the const variable name.
///
/// # Examples
/// ```
/// use astria_telemetry::metric_name;
/// metric_name!(pub const EXAMPLE_COUNTER);
/// // Note that this example has `astria_telemetry` a a prefix because
/// // this doctest is part of this crate.
/// // In your case, use your crate's `CARGO_CRATE_NAME` as prefix.
/// assert_eq!(EXAMPLE_COUNTER, "astria_telemetry_example_counter");
/// ```
#[macro_export]
macro_rules! metric_name {
    ($vis:vis const $($tt:tt)*) => {
        $crate::__metric_name_internal!(
            $vis [$($tt)*] [::core::stringify!($($tt)*)]
        );
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __metric_name_internal {
    ($vis:vis [$name:ident][$suffix:expr]) => {
        $vis const $name: &str = $crate::macros::__concatcp!(
            ::core::env!("CARGO_CRATE_NAME"),
            "_",
            $crate::macros::__map_ascii_case!($crate::macros::__Case::Lower, $suffix),
        );
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn gives_expected_const_and_value() {
        crate::metric_name!(const EXAMPLE_CONST);
        assert_eq!("astria_telemetry_example_const", EXAMPLE_CONST);
    }
}
