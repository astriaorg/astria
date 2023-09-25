pub mod primitive;
pub mod sequencer;

/// A helper trait to convert from raw decoded protobuf types to idiomatic astria types.
pub trait Protobuf: Sized {
    /// Errors that can occur when transforming from a raw type.
    type Error;
    /// The raw deserialized protobuf type.
    type Raw;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error>;

    fn try_from_raw(raw: Self::Raw) -> Result<Self, Self::Error> {
        Self::try_from_raw_ref(&raw)
    }

    fn to_raw(&self) -> Self::Raw;

    fn into_raw(self) -> Self::Raw {
        Self::to_raw(&self)
    }
}
