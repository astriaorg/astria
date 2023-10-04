// use color_eyre::eyre;

use crate::cli::{
    RollupConfigCreateArgs,
    RollupConfigDeleteArgs,
    RollupConfigDeployArgs,
    RollupConfigEditArgs,
};

// pub(crate) fn create_config(args: &RollupConfigCreateArgs) -> eyre::Result<()> {
pub(crate) fn create_config(args: &RollupConfigCreateArgs) {
    println!("Create Rollup Config {args:?}");
    // Ok(())
}
// pub(crate) fn edit_config(args: &RollupConfigEditArgs) -> eyre::Result<()> {
pub(crate) fn edit_config(args: &RollupConfigEditArgs) {
    println!("Edit Rollup Config {args:?}");
    // Ok(())
}
// pub(crate) fn deploy_config(args: &RollupConfigDeployArgs) -> eyre::Result<()> {
pub(crate) fn deploy_config(args: &RollupConfigDeployArgs) {
    println!("Deploy Rollup Config {args:?}");
    // Ok(())
}
// pub(crate) fn delete_config(args: &RollupConfigDeleteArgs) -> eyre::Result<()> {
pub(crate) fn delete_config(args: &RollupConfigDeleteArgs) {
    println!("Delete Rollup Config {args:?}");
    // Ok(())
}
