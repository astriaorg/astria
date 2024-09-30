/// A trait to safely convert to `f64`.
pub trait IntoF64 {
    /// Converts `self` to `f64`.
    fn into_f64(self) -> f64;
}

impl IntoF64 for f64 {
    fn into_f64(self) -> f64 {
        self
    }
}

impl IntoF64 for std::time::Duration {
    fn into_f64(self) -> f64 {
        self.as_secs_f64()
    }
}

impl IntoF64 for i8 {
    fn into_f64(self) -> f64 {
        f64::from(self)
    }
}

impl IntoF64 for u8 {
    fn into_f64(self) -> f64 {
        f64::from(self)
    }
}

impl IntoF64 for i16 {
    fn into_f64(self) -> f64 {
        f64::from(self)
    }
}

impl IntoF64 for u16 {
    fn into_f64(self) -> f64 {
        f64::from(self)
    }
}

impl IntoF64 for i32 {
    fn into_f64(self) -> f64 {
        f64::from(self)
    }
}

impl IntoF64 for u32 {
    fn into_f64(self) -> f64 {
        f64::from(self)
    }
}

impl IntoF64 for f32 {
    fn into_f64(self) -> f64 {
        f64::from(self)
    }
}

impl IntoF64 for usize {
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss is unlikely (values too small) but also unimportant in metrics"
    )]
    fn into_f64(self) -> f64 {
        self as f64
    }
}
