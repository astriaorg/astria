// SPDX-License-Identifier: MIT or Apache-2.0
pragma solidity ^0.8.21;

interface IAstriaMintableERC20 {
    function mint(address _to, uint256 _amount) external;
    function withdrawToSequencer(uint256 _amount, address _destinationChainAddress) external;
    function withdrawToIbcChain(uint256 _amount, string calldata _destinationChainAddress, string calldata _memo) external;
}
