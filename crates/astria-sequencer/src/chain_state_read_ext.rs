use anyhow::Result;
use cnidarium::{
    StateRead,
    StateWrite,
};
use cnidarium_component::ChainStateReadExt;

use crate::state_ext::StateReadExt as _;

#[derive(
    wrapper_derive::StateRead, wrapper_derive::StateWrite, wrapper_derive::ChainStateReadExt,
)]
pub(crate) struct StateDeltaWrapper<'a, S: StateRead + StateWrite>(pub(crate) &'a mut S);

#[derive(
    wrapper_derive::StateRead, wrapper_derive::StateWrite, wrapper_derive::ChainStateReadExt,
)]
pub(crate) struct ArcStateDeltaWrapper<'a, S: StateRead + StateWrite>(
    pub(crate) &'a mut std::sync::Arc<S>,
);
