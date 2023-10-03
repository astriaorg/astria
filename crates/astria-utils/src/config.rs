use figment::{
    providers::Env as FigmentEnv,
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};

pub trait AstriaConfig<'a>: Serialize + Deserialize<'a> {
    fn from_environment(env_prefix: &str) -> Result<Self, figment::Error> {
        Figment::new()
            .merge(FigmentEnv::prefixed("RUST_").split("_").only(&["log"]))
            .merge(FigmentEnv::prefixed(env_prefix))
            .extract()
    }
}
