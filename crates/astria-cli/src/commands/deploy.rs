use color_eyre::eyre;

use crate::cli::{
    DeployCelestiaArgs,
    DeployRollupArgs,
    DeploySequencerArgs,
};

pub(crate) fn deploy_celestia_local(args: DeployCelestiaArgs) -> eyre::Result<()> {
    println!("deploy local celestia, args: {:?}", args);
    Ok(())
}

pub(crate) fn deploy_rollup_local(args: DeployRollupArgs) -> eyre::Result<()> {
    print!("deploy local rollup, args: {:?}", args);
    Ok(())
}

pub(crate) fn deploy_rollup_remote(args: DeployRollupArgs) -> eyre::Result<()> {
    print!("deploy remote rollup, args: {:?}", args);
    Ok(())
}

pub(crate) fn deploy_sequencer_local(args: DeploySequencerArgs) -> eyre::Result<()> {
    print!("deploy local sequencer, args: {:?}", args);
    Ok(())
}

#[cfg(test)]
mod test {}
