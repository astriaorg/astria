// re-export so that they need not be imported by downstream users.
// hidden because they shouldn't be imported.
#[doc(hidden)]
pub use const_format::{
    concatcp as __concatcp,
    map_ascii_case as __map_ascii_case,
    Case as __Case,
};

/// Declare a collection of `const` string slices, using the declaring crate's name as a
/// prefix and the variable name as a suffix.
///
/// This macro essentially performs these declarations:
/// ```text
/// METRIC_NAME_1 := ${CARGO_CRATE_NAME}_metric_name_1;
/// METRIC_NAME_2 := ${CARGO_CRATE_NAME}_metric_name_2;
/// METRIC_NAME_3 := ${CARGO_CRATE_NAME}_metric_name_3;
///
/// METRICS_NAMES := [METRIC_NAME_1, METRIC_NAME_2, METRIC_NAME_3];
/// ```
///
/// The purpose of this macro is to avoid accidental typos, ensuring that the
/// metric name matches the const variable name, and to provide a collection of all metric names.
///
/// # Examples
/// ```
/// use astria_telemetry::metric_names;
/// metric_names!(pub const ALL_METRICS: EXAMPLE_COUNTER, EXAMPLE_GAUGE);
/// // Note that these examples have `astria_telemetry` as a prefix because
/// // this doctest is part of this crate.
/// // In your case, your own crate's `CARGO_CRATE_NAME` will be the prefix.
/// assert_eq!(EXAMPLE_COUNTER, "astria_telemetry_example_counter");
/// assert_eq!(EXAMPLE_GAUGE, "astria_telemetry_example_gauge");
/// assert_eq!(ALL_METRICS, [EXAMPLE_COUNTER, EXAMPLE_GAUGE]);
/// ```
#[macro_export]
macro_rules! metric_names {
    ($vis:vis const $collection_name:ident: $($name:ident),* $(,)?) => {
        $(
            $crate::__metric_name_internal!($vis[$name][::core::stringify!($name)]);
        )*
        $vis const $collection_name: [&str; $crate::__count!($($name)*)] = [
            $(
                $name,
            )*
        ];
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

#[doc(hidden)]
#[macro_export]
macro_rules! __count {
    () => (0_usize);
    ( $x:tt $($xs:tt)* ) => (1_usize + $crate::__count!($($xs)*));
}

#[cfg(test)]
mod tests {
    mod inner {
        metric_names!(const PRIVATE_METRICS: EXAMPLE_COUNTER, EXAMPLE_GAUGE, EXAMPLE_HISTOGRAM);
        metric_names!(pub(super) const PUBLIC_METRICS: PUB_COUNTER, PUB_GAUGE, PUB_HISTOGRAM);

        #[test]
        fn gives_expected_const_and_value() {
            assert_eq!("astria_telemetry_example_counter", EXAMPLE_COUNTER);
            assert_eq!("astria_telemetry_example_gauge", EXAMPLE_GAUGE);
            assert_eq!("astria_telemetry_example_histogram", EXAMPLE_HISTOGRAM);
            assert_eq!(
                PRIVATE_METRICS,
                [EXAMPLE_COUNTER, EXAMPLE_GAUGE, EXAMPLE_HISTOGRAM]
            );
        }
    }

    #[test]
    fn should_respect_pub_visibility() {
        assert_eq!("astria_telemetry_pub_counter", inner::PUB_COUNTER);
        assert_eq!("astria_telemetry_pub_gauge", inner::PUB_GAUGE);
        assert_eq!("astria_telemetry_pub_histogram", inner::PUB_HISTOGRAM);
        assert_eq!(
            inner::PUBLIC_METRICS,
            [inner::PUB_COUNTER, inner::PUB_GAUGE, inner::PUB_HISTOGRAM]
        );
    }

    #[test]
    fn should_allow_trailing_comma() {
        metric_names!(const TRAILING_COMMA: A,);
        assert_eq!("astria_telemetry_a", A);
        assert_eq!(TRAILING_COMMA, [A]);
    }
}
