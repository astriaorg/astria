// SPDX-License-Identifier: MIT or Apache-2.0
pragma solidity ^0.8.21;

abstract contract IAstriaWithdrawer {
    // the precision of the asset on the base chain.
    //
    // the amount transferred on the base chain will be divided by 10 ^ (18 - BASE_CHAIN_ASSET_PRECISION).
    //
    // for example, if base chain asset is precision is 6, the divisor would be 10^12.
    uint32 public immutable BASE_CHAIN_ASSET_PRECISION;

    // emitted when a withdrawal to the sequencer is initiated
    //
    // the `sender` is the evm address that initiated the withdrawal
    // the `destinationChainAddress` is the address on the sequencer the funds will be sent to
    event SequencerWithdrawal(address indexed sender, uint256 indexed amount, address destinationChainAddress);

    // emitted when a withdrawal to the IBC origin chain is initiated.
    // the withdrawal is sent to the origin chain via IBC from the sequencer using the denomination trace.
    //
    // the `sender` is the evm address that initiated the withdrawal
    // the `destinationChainAddress` is the address on the origin chain the funds will be sent to
    // the `memo` is an optional field that will be used as the ICS20 packet memo
    event Ics20Withdrawal(address indexed sender, uint256 indexed amount, string destinationChainAddress, string memo);
}
