// SPDX-License-Identifier: MIT or Apache-2.0
pragma solidity ^0.8.21;

// This contract facilitates withdrawals of the native asset from the rollup to the base chain.
// 
// Funds can be withdrawn to either the sequencer or the origin chain via IBC.
contract AstriaWithdrawer {
    // the precision of the asset on the base chain.
    //
    // the amount transferred on the base chain will be divided by 10 ^ (18 - BASE_CHAIN_ASSET_PRECISION).
    //
    // for example, if base chain asset is precision is 6, the divisor would be 10^12.
    uint32 public immutable BASE_CHAIN_ASSET_PRECISION;

    // the divisor used to convert the rollup asset amount to the base chain denomination
    //
    // set to 10^ASSET_WITHDRAWAL_DECIMALS on contract creation
    uint256 private immutable DIVISOR;

    constructor(uint32 _baseChainAssetPrecision) {
        if (_baseChainAssetPrecision > 18) {
            revert("AstriaWithdrawer: base chain asset precision must be less than or equal to 18");
        }
        BASE_CHAIN_ASSET_PRECISION = _baseChainAssetPrecision;
        DIVISOR = 10 ** (18 - _baseChainAssetPrecision);
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
        require(amount / DIVISOR > 0, "AstriaWithdrawer: insufficient value, must be greater than 10 ** (18 - BASE_CHAIN_ASSET_PRECISION)");
        _;
    }
    
    function withdrawToSequencer(address destinationChainAddress) external payable sufficientValue(msg.value) {
        emit SequencerWithdrawal(msg.sender, msg.value, destinationChainAddress);
    }

    function withdrawToIbcChain(string calldata destinationChainAddress, string calldata memo) external payable sufficientValue(msg.value) {
        emit Ics20Withdrawal(msg.sender, msg.value, destinationChainAddress, memo);
    }
}
