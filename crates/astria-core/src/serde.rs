pub(crate) mod string {
    use serde::Serializer;

    pub(crate) fn hex<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<[u8]>,
        S: Serializer,
    {
        struct Hex<'a>(&'a [u8]);

        impl<'a> std::fmt::Display for Hex<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for byte in self.0 {
                    f.write_fmt(format_args!("{byte:02x}"))?;
                }
                Ok(())
            }
        }

        serializer.collect_str(&Hex(value.as_ref()))
    }
}
