use color_eyre::eyre;

use crate::cli::{
    RollupConfigCreateArgs,
    RollupConfigDeleteArgs,
    RollupConfigDeployArgs,
    RollupConfigEditArgs,
};

pub(crate) fn create_config(args: RollupConfigCreateArgs) -> eyre::Result<()> {
    println!("Create Rollup Config {:?}", args);
    Ok(())
}
pub(crate) fn edit_config(args: RollupConfigEditArgs) -> eyre::Result<()> {
    println!("Edit Rollup Config {:?}", args);
    Ok(())
}
pub(crate) fn deploy_config(args: RollupConfigDeployArgs) -> eyre::Result<()> {
    println!("Deploy Rollup Config {:?}", args);
    Ok(())
}
pub(crate) fn delete_config(args: RollupConfigDeleteArgs) -> eyre::Result<()> {
    println!("Delete Rollup Config {:?}", args);
    Ok(())
}
