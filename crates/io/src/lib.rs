use alloy_primitives::{B256, U256};
use alloy_sol_types::sol;
use anyhow::Result;
use ethereum_consensus::types::mainnet::BeaconState;
use serde::{Deserialize, Serialize};

sol! {
    /// The input that is passed via the on-chain contract
    /// into the coprocessor
    #[derive(Debug)]
    struct Input {
        bytes32 block_root;
        bytes32 manifest_hash;
    }
}

sol! {
    /// An oracle report as stored in the contract
    /// This is the output of the coprocessor execution
    #[derive(Debug)]
    struct Report {
        uint256 clBalanceGwei;
        uint256 withdrawalVaultBalanceWei;
        uint256 totalDepositedValidators;
        uint256 totalExitedValidators;
    }
}

// mainnet
pub const WITHDRAWAL_CREDENTIALS: B256 = B256::new([
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xb9, 0xd7, 0x93, 0x48,
    0x78, 0xb5, 0xfb, 0x96, 0x10, 0xb3, 0xfe, 0x8a, 0x5e, 0x44, 0x1e, 0x8f, 0xad, 0x7e, 0x29, 0x3f,
]);

// Holesky
// pub const WITHDRAWAL_CREDENTIALS: B256 = B256::new([
//     0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x17, 0x9d, 0xEC,
//     0x45, 0xa3, 0x74, 0x23, 0xEA, 0xD4, 0xFa, 0xD5, 0xfC, 0xb1, 0x36, 0x19, 0x78, 0x72, 0xEA, 0xd9,
// ]);

pub fn derive_report(state: &BeaconState) -> Report {
    // total balance of all Lido validators
    let current_epoch = state.slot() / 32;
    let (cl_balance_gewi, total_deposited, total_exited) = state
        .validators()
        .iter()
        .zip(state.balances().iter())
        .filter(|(v, _)| v.withdrawal_credentials.as_slice() == WITHDRAWAL_CREDENTIALS.as_slice())
        .fold(
            (0u64, 0u64, 0u64),
            |(cl_balance_gwei, total_deposited, total_exited), (validator, bal)| {
                let did_exit = if validator.exit_epoch <= current_epoch {
                    1
                } else {
                    0
                };
                (
                    cl_balance_gwei + bal,
                    total_deposited + 1,
                    total_exited + did_exit,
                )
            },
        );

    Report {
        clBalanceGwei: U256::from(cl_balance_gewi),
        withdrawalVaultBalanceWei: U256::ZERO, // TODO: This requires execute state data so is a problem for another time
        totalDepositedValidators: U256::from(total_deposited),
        totalExitedValidators: U256::from(total_exited),
    }
}

/// THe manifest is the first piece of data loaded into the coprocessor
/// It contains the block hash and the state chunk hashes which can be used to
/// retrieve the content via the preimage oracle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub block_hash: [u8; 32],
    pub state_chunk_hashes: Vec<[u8; 32]>,
}

impl Manifest {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_cbor::from_slice(bytes)?)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_cbor::to_vec(self)?)
    }
}
