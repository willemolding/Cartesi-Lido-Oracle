// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {CartesiLidoOracle} from "../src/CartesiLidoOracle.sol";

contract CartesiLidoOracleScript is Script {
    CartesiLidoOracle public oracle;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        oracle = new CartesiLidoOracle(address(0), bytes32(0));

        vm.stopBroadcast();
    }
}
