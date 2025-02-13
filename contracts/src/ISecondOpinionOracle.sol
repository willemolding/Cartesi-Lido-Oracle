// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @title LIP-23 beacon chain oracle interface
/// https://github.com/lidofinance/lido-improvement-proposals/blob/develop/LIPS/lip-23.md
interface ISecondOpinionOracle {
    function getReport(uint256 refSlot)
        external
        view
        returns (
            bool success,
            uint256 clBalanceGwei,
            uint256 withdrawalVaultBalanceWei,
            uint256 totalDepositedValidators,
            uint256 totalExitedValidators
        );
}
