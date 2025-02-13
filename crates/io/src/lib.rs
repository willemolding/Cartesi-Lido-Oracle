use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};

use anyhow::Result;

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
