pub mod v1;

/// A trait to convert from raw decoded protobuf types to idiomatic astria types.
///
/// The primary use of this trait is to convert to/from foreign types.
pub trait Protobuf: Sized {
    /// Errors that can occur when transforming from a raw type.
    type Error;
    /// The raw deserialized protobuf type.
    type Raw;

    /// Convert from a reference to the raw protobuf type.
    ///
    /// # Errors
    /// Returns [`Self::Error`] as defined by the implementator of this trait.
    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error>;

    /// Convert from the raw protobuf type, dropping it.
    ///
    /// This method provides a default implementation in terms of
    /// [`Self::try_from_raw_ref`].
    ///
    /// # Errors
    /// Returns [`Self::Error`] as defined by the implementator of this trait.
    fn try_from_raw(raw: Self::Raw) -> Result<Self, Self::Error> {
        Self::try_from_raw_ref(&raw)
    }

    /// Convert to the raw protobuf type by reference.
    fn to_raw(&self) -> Self::Raw;

    /// Convert to the raw protobuf type, dropping `self`.
    ///
    /// This method provides a default implementation in terms of
    /// [`Self::to_raw`].
    fn into_raw(self) -> Self::Raw {
        Self::to_raw(&self)
    }
}
