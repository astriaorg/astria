// SPDX-License-Identifier: MIT or Apache-2.0
pragma solidity ^0.8.21;

contract AstriaWithdrawer {
    event Withdrawal(address indexed sender, uint256 indexed amount, bytes memo);
    
    function withdraw(bytes calldata memo) external payable {
        emit Withdrawal(msg.sender, msg.value, memo);
    }
}
