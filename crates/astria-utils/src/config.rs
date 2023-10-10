use figment::{
    providers::Env as FigmentEnv,
    Figment,
};
use serde::{
    Deserialize,
    Serialize,
};

pub trait AstriaConfig<'a>: Serialize + Deserialize<'a> {
    const PREFIX: & 'static str;

    fn from_environment(env_prefix: &str) -> Result<Self, figment::Error> {
        Figment::new()
            .merge(FigmentEnv::prefixed("RUST_").split("_").only(&["log"]))
            .merge(FigmentEnv::prefixed(env_prefix))
            .extract()
    }
}

pub fn get_config<'a, T>(env_prefix: &str) -> Result<T, figment::Error> where T: AstriaConfig<'a> {
    T::from_environment(env_prefix)
}