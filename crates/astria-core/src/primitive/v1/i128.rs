//! Transformations of compiled protobuf types to other types.

use crate::generated::astria::primitive::v1::Int128;
impl From<i128> for Int128 {
    fn from(primitive: i128) -> Self {
        let [h0, h1, h2, h3, h4, h5, h6, h7, l0, l1, l2, l3, l4, l5, l6, l7] =
            primitive.to_be_bytes();
        let lo = u64::from_be_bytes([l0, l1, l2, l3, l4, l5, l6, l7]);
        let hi = u64::from_be_bytes([h0, h1, h2, h3, h4, h5, h6, h7]);
        Self {
            lo,
            hi,
        }
    }
}

impl From<Int128> for i128 {
    fn from(pb: Int128) -> i128 {
        let [l0, l1, l2, l3, l4, l5, l6, l7] = pb.lo.to_be_bytes();
        let [h0, h1, h2, h3, h4, h5, h6, h7] = pb.hi.to_be_bytes();
        i128::from_be_bytes([
            h0, h1, h2, h3, h4, h5, h6, h7, l0, l1, l2, l3, l4, l5, l6, l7,
        ])
    }
}

impl<'a> From<&'a i128> for Int128 {
    fn from(primitive: &'a i128) -> Self {
        (*primitive).into()
    }
}

#[cfg(test)]
mod tests {
    use super::Int128;

    #[track_caller]
    fn i128_roundtrip_check(expected: i128) {
        let pb: Int128 = expected.into();
        let actual: i128 = pb.into();
        assert_eq!(expected, actual);
    }

    #[test]
    fn i128_roundtrips_work() {
        i128_roundtrip_check(0i128);
        i128_roundtrip_check(1i128);
        i128_roundtrip_check(i128::from(u64::MAX));
        i128_roundtrip_check(i128::from(u64::MAX) + 1i128);
        i128_roundtrip_check(1i128 << 127);
        i128_roundtrip_check((1i128 << 127) + (1i128 << 63));
        i128_roundtrip_check(i128::MAX);
        i128_roundtrip_check(i128::MIN);
        i128_roundtrip_check(-1i128);
        i128_roundtrip_check(-i128::from(u64::MAX));
        i128_roundtrip_check(-i128::from(u64::MAX) - 1i128);
    }
}
