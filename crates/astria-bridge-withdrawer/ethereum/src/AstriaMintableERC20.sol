// SPDX-License-Identifier: MIT or Apache-2.0
pragma solidity ^0.8.21;

import {IAstriaMintableERC20} from "./IAstriaMintableERC20.sol";
import {ERC20} from "lib/openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";

contract AstriaMintableERC20 is IAstriaMintableERC20, ERC20 {
    // the `astriaBridgeSenderAddress` built into the astria-geth node
    address public immutable BRIDGE;

    // emitted when tokens are minted from a deposit
    event Mint(address indexed account, uint256 amount);

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
    
    modifier onlyBridge() {
        require(msg.sender == BRIDGE, "AstriaMintableERC20: only bridge can mint");
        _;
    }

    constructor(
        address _bridge,
        string memory _name,
        string memory _symbol
    ) ERC20(_name, _symbol) {
        BRIDGE = _bridge;
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
