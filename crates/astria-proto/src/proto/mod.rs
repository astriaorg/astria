#[path = "tonic"]
/// Files generated using [`prost`] and [`tonic`] via [`buf`] and its
/// [`neoeinstein-prost`] and [`neoeinstein-tonic`] plugins.
///
/// [`prost`]:
/// [`tonic`]:
/// [`buf`]: https://buf.build
/// [`neoeinstein-prost`]: https://buf.build/community/neoeinstein-prost
/// [`neoeinstein-tonic`]: https://buf.build/community/neoeinstein-tonic
pub mod tonic {
    #[path = ""]
    pub mod execution {
        #[path = "astria.execution.v1.rs"]
        pub mod v1;
    }

    #[path = ""]
    pub mod primitive {
        #[path = "astria.primitive.v1.rs"]
        pub mod v1;
    }

    #[path = ""]
    pub mod sequencer {
        #[path = "astria.sequencer.v1.rs"]
        pub mod v1;
    }
}

// mod primitive_impls {
//     //! Implementations of foreign traits for foreign types to
//     //! deal with orphan rules.

//     use crate::primitive::v1::Uint128;
//     impl From<u128> for Uint128 {
//         fn from(primitive: u128) -> Self {
//             let [
//                 h0,
//                 h1,
//                 h2,
//                 h3,
//                 h4,
//                 h5,
//                 h6,
//                 h7,
//                 l0,
//                 l1,
//                 l2,
//                 l3,
//                 l4,
//                 l5,
//                 l6,
//                 l7,
//             ] = primitive.to_be_bytes();
//             let lo = u64::from_be_bytes([l0, l1, l2, l3, l4, l5, l6, l7]);
//             let hi = u64::from_be_bytes([h0, h1, h2, h3, h4, h5, h6, h7]);
//             Self {
//                 lo,
//                 hi,
//             }
//         }
//     }

//     impl From<Uint128> for u128 {
//         fn from(pb: Uint128) -> u128 {
//             let [l0, l1, l2, l3, l4, l5, l6, l7] = pb.lo.to_be_bytes();
//             let [h0, h1, h2, h3, h4, h5, h6, h7] = pb.hi.to_be_bytes();
//             u128::from_be_bytes([
//                 h0, h1, h2, h3, h4, h5, h6, h7, l0, l1, l2, l3, l4, l5, l6, l7,
//             ])
//         }
//     }

//     #[cfg(test)]
//     mod tests {
//         use crate::primitive::v1::Uint128;
//         #[track_caller]
//         fn u128_roundtrip_check(expected: u128) {
//             let pb: Uint128 = expected.into();
//             let actual: u128 = pb.into();
//             assert_eq!(expected, actual);
//         }
//         #[test]
//         fn u128_roundtrips_work() {
//             u128_roundtrip_check(0u128);
//             u128_roundtrip_check(1u128);
//             u128_roundtrip_check(u128::from(u64::MAX));
//             u128_roundtrip_check(u128::from(u64::MAX) + 1u128);
//             u128_roundtrip_check(1u128 << 127);
//             u128_roundtrip_check((1u128 << 127) + (1u128 << 63));
//             u128_roundtrip_check(u128::MAX);
//         }
//     }
// }
