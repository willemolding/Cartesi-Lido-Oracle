use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    block_root: [u8; 32],
    manifest_hash: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    block_hash: [u8; 32],
    state_chunk_hashes: Vec<[u8; 32]>,
}
