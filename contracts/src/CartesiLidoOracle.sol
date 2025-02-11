// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {CoprocessorAdapter} from "../lib/coprocessor-base-contract/src/CoprocessorAdapter.sol";
import {ISecondOpinionOracle} from "./ISecondOpinionOracle.sol";
import "./BeaconBlockRoots.sol";

contract CartesiLidoOracle is CoprocessorAdapter, ISecondOpinionOracle {
    /// An oracle report as stored in the contract
    struct Report {
        uint256 clBalanceGwei;
        uint256 withdrawalVaultBalanceWei;
        uint256 totalDepositedValidators;
        uint256 totalExitedValidators;
    }

    /// Genesis timestamp of the chain, required for retrieving the beacon block roots
    uint256 public immutable genesis_block_timestamp;

    /// @notice Oracle reports stored by slot.
    mapping(uint256 => Report) public reports;

    /// @notice Mapping from payload hash to slot of inflight requests.
    mapping(bytes32 => uint256) public inflightRequests;

    constructor(address _taskIssuerAddress, bytes32 _machineHash)
        CoprocessorAdapter(_taskIssuerAddress, _machineHash)
    {}

    /// @notice Generates a report for a given slot. This slot must be within the last 32768 blocks or this will fail
    function generateReport(uint256 slot) external {
        // this will revert if unable to get the block root for this slot
        bytes32 blockRoot = BeaconBlockRoots.findBlockRoot(genesis_block_timestamp, slot);
        bytes memory input = abi.encode(blockRoot);
        bytes32 payloadHash = keccak256(input);
        inflightRequests[payloadHash] = slot;
        callCoprocessor(input);
    }

    /// @notice Callback that is invoked by the coprocessor with the outputs (notice) of the computation
    function handleNotice(bytes32 payloadHash, bytes memory notice) internal override {
        uint256 slot = inflightRequests[payloadHash];
        if (slot == 0) {
            // This is not a valid payload hash
            revert("No inflight request found for payload hash");
        }

        Report memory report = abi.decode(notice, (Report));
        reports[slot] = report;
    }

    function getReport(uint256 slot)
        external
        view
        returns (
            bool success,
            uint256 clBalanceGwei,
            uint256 withdrawalVaultBalanceWei,
            uint256 totalDepositedValidators,
            uint256 totalExitedValidators
        )
    {
        Report memory report = reports[slot];
        return (
            report.clBalanceGwei != 0,
            report.clBalanceGwei,
            report.withdrawalVaultBalanceWei,
            report.totalDepositedValidators,
            report.totalExitedValidators
        );
    }
}
