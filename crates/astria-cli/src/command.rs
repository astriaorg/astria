use std::{
    future::Future,
    pin::Pin,
};

use color_eyre::eyre;

use crate::output::{
    IntoOutput,
    Output,
};

pub(super) type RunCommandFut = Pin<Box<dyn Future<Output = eyre::Result<Output>> + Send>>;

pub(super) fn run<T: Command>(command: T) -> RunCommandFut {
    command.run()
}

pub(super) fn run_sync<F, Out>(f: F) -> RunCommandFut
where
    F: FnOnce() -> eyre::Result<Out> + Clone + Send + 'static,
    Out: IntoOutput,
{
    run(move || async { f() })
}

pub(super) trait Command {
    fn run(self) -> RunCommandFut;
}

impl<F, Fut, Out> Command for F
where
    F: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = eyre::Result<Out>> + Send,
    Out: IntoOutput,
{
    fn run(self) -> RunCommandFut {
        Box::pin(async move { self().await.map(IntoOutput::into_output) })
    }
}
