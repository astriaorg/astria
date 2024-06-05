// SPDX-License-Identifier: MIT or Apache-2.0
pragma solidity ^0.8.21;

// This contract facilitates withdrawals of the native asset from the rollup to the base chain.
// 
// Funds can be withdrawn to either the sequencer or the origin chain via IBC.
contract AstriaWithdrawer {
    // the number of decimal places more the asset has on the rollup versus the base chain.
    //
    // the amount transferred on the base chain will be divided by 10^ASSET_WITHDRAWAL_DECIMALS.
    //
    // for example, if the rollup specifies the asset has 18 decimal places and the base chain specifies 6,
    // the ASSET_WITHDRAWAL_DECIMALS would be 12.
    uint32 public immutable ASSET_WITHDRAWAL_DECIMALS;

    // the divisor used to convert the rollup asset amount to the base chain denomination
    //
    // set to 10^ASSET_WITHDRAWAL_DECIMALS on contract creation
    uint256 private immutable DIVISOR;

    constructor(uint32 assetWithdrawalDecimals) {
        ASSET_WITHDRAWAL_DECIMALS = assetWithdrawalDecimals;
        DIVISOR = 10 ** assetWithdrawalDecimals;
    }

    // emitted when a withdrawal to the sequencer is initiated
    //
    // the `sender` is the evm address that initiated the withdrawal
    // the `destinationChainAddress` is the address on the sequencer the funds will be sent to
    event SequencerWithdrawal(address indexed sender, uint256 indexed amount, address destinationChainAddress);

    // emitted when a withdrawal to the origin chain is initiated.
    // the withdrawal is sent to the origin chain via IBC from the sequencer using the denomination trace.
    //
    // the `sender` is the evm address that initiated the withdrawal
    // the `destinationChainAddress` is the address on the origin chain the funds will be sent to
    // the `memo` is an optional field that will be used as the ICS20 packet memo
    event Ics20Withdrawal(address indexed sender, uint256 indexed amount, string destinationChainAddress, string memo);

    modifier sufficientValue(uint256 amount) {
        require(amount / DIVISOR > 0, "AstriaWithdrawer: insufficient value, must be greater than 10**ASSET_WITHDRAWAL_DECIMALS");
        _;
    }
    
    function withdrawToSequencer(address destinationChainAddress) external payable sufficientValue(msg.value) {
        emit SequencerWithdrawal(msg.sender, msg.value, destinationChainAddress);
    }

    function withdrawToIbcChain(string calldata destinationChainAddress, string calldata memo) external payable sufficientValue(msg.value) {
        emit Ics20Withdrawal(msg.sender, msg.value, destinationChainAddress, memo);
    }
}
