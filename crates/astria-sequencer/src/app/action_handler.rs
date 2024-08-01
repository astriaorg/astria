use cnidarium::StateWrite;

/// This trait is a verbatim copy of [`cnidarium_component::ActionHandler`].
///
/// It's duplicated here because all actions are foreign types, forbidding
/// the the implementation of [`cnidarium_component::ActionHandler`] for these
/// types due to Rust orphan rules.
#[async_trait::async_trait]
pub(crate) trait ActionHandler {
    type CheckStatelessContext: Clone + Send + Sync + 'static;
    async fn check_stateless(&self, context: Self::CheckStatelessContext) -> anyhow::Result<()>;

    // Commenting out for the time being as CI flags this as not being used. Leaving this in
    // for reference as this is copied from cnidarium_component.
    // ```
    // async fn check_historical<S: StateRead + 'static>(&self, _state: Arc<S>) -> anyhow::Result<()> {
    //     Ok(())
    // }
    // ```

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> anyhow::Result<()>;
}
