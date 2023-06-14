pub mod execution {
    #![allow(unreachable_pub)]
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/astria.execution.v1.rs"));
    }
}

pub mod primitive {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/astria.primitive.v1.rs"));
    }
}

pub mod sequencer {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/astria.sequencer.v1.rs"));
    }
}

mod primitive_impls {
    //! Implementations of foreign traits for foreign types to
    //! deal with orphan rules.

    use crate::primitive::v1::Uint128;
    impl From<u128> for Uint128 {
        fn from(primitive: u128) -> Self {
            let lo = primitive as u64;
            let hi = (primitive >> 64) as u64;
            Self {
                lo,
                hi,
            }
        }
    }

    impl From<Uint128> for u128 {
        fn from(pb: Uint128) -> u128 {
            let lo = pb.lo as u128;
            let hi = (pb.hi as u128) << 64;
            hi + lo
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::primitive::v1::Uint128;
        #[track_caller]
        fn u128_roundtrip_check(expected: u128) {
            let pb: Uint128 = expected.into();
            let actual: u128 = pb.into();
            assert_eq!(expected, actual);
        }
        #[test]
        fn u128_roundtrips_work() {
            u128_roundtrip_check(0u128);
            u128_roundtrip_check(1u128);
            u128_roundtrip_check(u64::MAX as u128);
            u128_roundtrip_check(u64::MAX as u128 + 1u128);
            u128_roundtrip_check(1u128 << 127);
            u128_roundtrip_check((1u128 << 127) + (1u128 << 63));
            u128_roundtrip_check(u128::MAX);
        }
    }
}
