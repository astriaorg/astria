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

    use crate::primitive::v1::U128;
    impl From<u128> for U128 {
        fn from(primitive: u128) -> U128 {
            let lo = primitive as u64;
            let hi = (primitive >> 64) as u64;
            U128 {
                lo,
                hi,
            }
        }
    }

    impl From<U128> for u128 {
        fn from(pb: U128) -> u128 {
            let lo = pb.lo as u128;
            let hi = (pb.hi as u128) << 64;
            hi + lo
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::primitive::v1::U128;
        #[track_caller]
        fn u128_roundtrip_check(expected: u128) {
            let pb: U128 = expected.into();
            let actual: u128 = pb.into();
            assert_eq!(expected, actual);
        }
        #[test]
        fn u128_roundtrips_work() {
            u128_roundtrip_check(0u128);
            u128_roundtrip_check(1u128);
            u128_roundtrip_check(u64::MAX as u128);
            u128_roundtrip_check(u128::MAX);
        }
    }
}
