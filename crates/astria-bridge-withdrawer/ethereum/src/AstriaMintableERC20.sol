// SPDX-License-Identifier: MIT or Apache-2.0
pragma solidity ^0.8.21;

import {IAstriaWithdrawer} from "./IAstriaWithdrawer.sol";
import {ERC20} from "lib/openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

contract AstriaMintableERC20 is IAstriaWithdrawer, ERC20 {
    // the `astriaBridgeSenderAddress` built into the astria-geth node
    address public immutable BRIDGE;

    // emitted when tokens are minted from a deposit
    event Mint(address indexed account, uint256 amount);

    modifier onlyBridge() {
        require(msg.sender == BRIDGE, "AstriaMintableERC20: only bridge can mint");
        _;
    }

    constructor(
        address _bridge,
        uint32 _assetWithdrawalDecimals,
        string memory _name,
        string memory _symbol
    ) ERC20(_name, _symbol) {
        BRIDGE = _bridge;
        ASSET_WITHDRAWAL_DECIMALS = _assetWithdrawalDecimals;
    }

    function mint(address _to, uint256 _amount)
        external
        onlyBridge
    {
        _mint(_to, _amount);
        emit Mint(_to, _amount);
    }

    function withdrawToSequencer(uint256 _amount, address _destinationChainAddress)
        external
    {
        _burn(msg.sender, _amount);
        emit SequencerWithdrawal(msg.sender, _amount, _destinationChainAddress);
    }

    function withdrawToIbcChain(uint256 _amount, string calldata _destinationChainAddress, string calldata _memo)
        external
    {
        _burn(msg.sender, _amount);
        emit Ics20Withdrawal(msg.sender, _amount, _destinationChainAddress, _memo);
    }
}
