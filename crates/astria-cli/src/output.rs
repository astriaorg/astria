use std::fmt::Display;

use erased_serde::Serialize;

pub(super) struct Output(Box<dyn Display>);

impl Output {
    fn new<T: Display + 'static>(val: T) -> Self {
        Self(Box::new(val))
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

struct Json(Box<dyn Serialize>);
impl Json {
    fn new<T: Serialize + 'static>(val: T) -> Self {
        Self(Box::new(val))
    }
}

impl Display for Json {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // FIXME: Get rid of the unwrap here.
        serde_json::to_value(&self.0).unwrap().fmt(f)
    }
}

pub(super) trait IntoOutput {
    fn into_output(self) -> Output;
}

impl IntoOutput for Json {
    fn into_output(self) -> Output {
        Output::new(self)
    }
}

impl IntoOutput for Output {
    fn into_output(self) -> Output {
        self
    }
}

impl<T: Serialize + 'static> IntoOutput for T {
    fn into_output(self) -> Output {
        Json::new(self).into_output()
    }
}
