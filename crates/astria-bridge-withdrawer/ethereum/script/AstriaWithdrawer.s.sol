// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {AstriaWithdrawer} from "../src/AstriaWithdrawer.sol";

contract AstriaWithdrawerScript is Script {
    function setUp() public {}

    function deploy() public {
        uint32 baseChainAssetPrecision = uint32(vm.envUint("BASE_CHAIN_ASSET_PRECISION"));
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);
        new AstriaWithdrawer(baseChainAssetPrecision);
        vm.stopBroadcast();
    }

    function withdrawToSequencer() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        address contractAddress = vm.envAddress("ASTRIA_WITHDRAWER");
        AstriaWithdrawer astriaWithdrawer = AstriaWithdrawer(contractAddress);

        address destinationChainAddress = vm.envAddress("SEQUENCER_DESTINATION_CHAIN_ADDRESS");
        uint256 amount = vm.envUint("AMOUNT");
        astriaWithdrawer.withdrawToSequencer{value: amount}(destinationChainAddress);

        vm.stopBroadcast();
    }

    function withdrawToIbcChain() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        address contractAddress = vm.envAddress("ASTRIA_WITHDRAWER");
        AstriaWithdrawer astriaWithdrawer = AstriaWithdrawer(contractAddress);

        string memory destinationChainAddress = vm.envString("ORIGIN_DESTINATION_CHAIN_ADDRESS");
        uint256 amount = vm.envUint("AMOUNT");
        astriaWithdrawer.withdrawToIbcChain{value: amount}(destinationChainAddress, "");

        vm.stopBroadcast();
    }
}
