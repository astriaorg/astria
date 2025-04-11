use std::{
    borrow::Cow,
    fmt::{
        self,
        Display,
        Formatter,
    },
};

/// The human-readable name assigned to a given upgrade change.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ChangeName(Cow<'static, str>);

impl ChangeName {
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0.into_owned()
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Cow::Borrowed(name) => name,
            Cow::Owned(name) => name.as_str(),
        }
    }
}

impl Display for ChangeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&'static str> for ChangeName {
    fn from(name: &'static str) -> Self {
        Self::new(name)
    }
}

impl From<String> for ChangeName {
    fn from(name: String) -> Self {
        Self(Cow::Owned(name))
    }
}
