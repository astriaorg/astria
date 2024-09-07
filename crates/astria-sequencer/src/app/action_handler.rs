use cnidarium::StateWrite;

/// This trait is a verbatim copy of `cnidarium_component::ActionHandler`.
///
/// It's duplicated here because all actions are foreign types, forbidding
/// the implementation of [`cnidarium_component::ActionHandler`][1] for
/// these types due to Rust orphan rules.
///
/// [1]: https://github.com/penumbra-zone/penumbra/blob/14959350abcb8cfbf33f9aedc7463fccfd8e3f9f/crates/cnidarium-component/src/action_handler.rs#L30
#[async_trait::async_trait]
pub(crate) trait ActionHandler {
    // Commenting out for the time being as this is currently not being used. Leaving this in
    // for reference as this is copied from cnidarium_component.
    // ```
    // type CheckStatelessContext: Clone + Send + Sync + 'static;
    // async fn check_stateless(&self, context: Self::CheckStatelessContext) -> anyhow::Result<()>;
    // async fn check_historical<S: StateRead + 'static>(&self, _state: Arc<S>) -> anyhow::Result<()> {
    //     Ok(())
    // }
    // ```

    async fn check_stateless(&self) -> anyhow::Result<()>;

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> anyhow::Result<()>;
}
