// SPDX-License-Identifier: MIT or Apache-2.0
pragma solidity ^0.8.21;

import {IAstriaWithdrawer} from "./IAstriaWithdrawer.sol";

// This contract facilitates withdrawals of the native asset from the rollup to the base chain.
// 
// Funds can be withdrawn to either the sequencer or the origin chain via IBC.
contract AstriaWithdrawer is IAstriaWithdrawer {
    constructor(uint32 _assetWithdrawalDecimals) {
        ASSET_WITHDRAWAL_DECIMALS = _assetWithdrawalDecimals;
    }

    function withdrawToSequencer(address _destinationChainAddress) external payable {
        emit SequencerWithdrawal(msg.sender, msg.value, _destinationChainAddress);
    }

    function withdrawToOriginChain(string calldata _destinationChainAddress, string calldata _memo) external payable {
        emit Ics20Withdrawal(msg.sender, msg.value, _destinationChainAddress, _memo);
    }
}
