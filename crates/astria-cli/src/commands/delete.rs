use color_eyre::eyre;

use crate::cli::DeleteRollupArgs;

pub(crate) fn delete_celestia_local() -> eyre::Result<()> {
    println!("delete local celestia");
    Ok(())
}

pub(crate) fn delete_rollup_local(args: DeleteRollupArgs) -> eyre::Result<()> {
    println!("delete local rollup, args: {:?}", args);
    Ok(())
}

pub(crate) fn delete_rollup_remote(args: DeleteRollupArgs) -> eyre::Result<()> {
    println!("delete remote rollup, args: {:?}", args);
    Ok(())
}

pub(crate) fn delete_sequencer_local() -> eyre::Result<()> {
    println!("delete local sequencer");
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_delete_celestia_local() {
        assert!(delete_celestia_local().is_ok());
    }

    #[test]
    fn test_delete_sequencer_local() {
        assert!(delete_sequencer_local().is_ok());
    }
}
