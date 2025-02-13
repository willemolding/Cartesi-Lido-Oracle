//! The orchestrator is responsible for coordinating the production of the oracle reports
//!
//! This involves:
//! - Retrieving the beacon chain data via RPC and building inputs
//! - Uploading preimage data to operators
//! - Calling the contract to initiate the coprocessor execution

mod beacon_client;

use anyhow::Result;
use beacon_client::BeaconClient;
use clap::Parser;
use ethereum_consensus::{phase0::SignedBeaconBlockHeader, types::mainnet::BeaconState};
use sha3::{Digest, Keccak256};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use url::Url;

const CHUNK_SIZE: usize = 1024 * 256;
const KECCACK_HASH_TYPE: u8 = 2;

#[derive(Parser, Debug)]
struct Args {
    /// Ethereum beacon node HTTP RPC endpoint.
    #[clap(long, env)]
    beacon_rpc_url: Url,

    /// Ethereum Node endpoint.
    // #[clap(long, env)]
    // eth_rpc_url: Url,

    /// Beacon slot to generate oracle report for
    #[clap(long)]
    slot: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let Args {
        beacon_rpc_url,
        slot,
        ..
    } = Args::parse();

    let beacon_client = BeaconClient::new_with_cache(beacon_rpc_url, "./beacon-cache")?;
    let block_header = beacon_client.get_block_header(slot).await?;
    let beacon_state = beacon_client.get_beacon_state(slot).await?;

    let inputs = build_inputs::<CHUNK_SIZE>(block_header, beacon_state);

    tracing::info!("Inputs: {:?}", inputs);

    // upload the chunks, block data and manifest to the operators to use in the preimage oracle
    // upload_to_operator(eth_rpc_url, &inputs).await?;

    // call the contract to initiate the coprocessor execution

    Ok(())
}

#[derive(Debug)]
struct Inputs {
    manifest: io::Manifest,
    block_data: Vec<u8>,
    state_chunks: Vec<Vec<u8>>,
}

fn build_inputs<const CHUNK_SIZE: usize>(
    beacon_block: SignedBeaconBlockHeader,
    beacon_state: BeaconState,
) -> Inputs {
    let block_data = serde_cbor::to_vec(&beacon_block).unwrap();
    let beacon_state_data = serde_cbor::to_vec(&beacon_state).unwrap();
    let state_chunks: Vec<_> = beacon_state_data
        .chunks(CHUNK_SIZE)
        .map(|c| c.to_vec())
        .collect();

    let manifest = io::Manifest {
        block_hash: keccak(&block_data),
        state_chunk_hashes: state_chunks.iter().map(|c| keccak(&c)).collect(),
    };

    Inputs {
        block_data,
        state_chunks,
        manifest,
    }
}

fn keccak(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.into()
}

async fn upload_to_operator(base_url: Url, inputs: &Inputs) -> Result<()> {
    let client = reqwest::Client::new();

    // upload all the preimages
    let preimages: Vec<(u8, Vec<u8>, Vec<u8>)> = vec![
        (
            KECCACK_HASH_TYPE,
            keccak(&inputs.manifest.to_bytes()?).to_vec(),
            inputs.manifest.to_bytes()?,
        ),
        (
            KECCACK_HASH_TYPE,
            inputs.manifest.block_hash.to_vec(),
            inputs.block_data.clone(),
        ),
    ]
    .into_iter()
    .chain(
        inputs
            .state_chunks
            .iter()
            .map(|c| (KECCACK_HASH_TYPE, keccak(c).to_vec(), c.clone())),
    )
    .collect();

    let res = client
        .post(base_url.join("/upload_preimages/")?)
        .body(serde_cbor::to_vec(&preimages)?)
        .send()
        .await?;

    if res.status() != 200 {
        return Err(anyhow::anyhow!("Failed to upload preimages"));
    }

    // sanity check the preimages are uploaded correctly
    let check_body: Vec<(u8, Vec<u8>)> = preimages
        .into_iter()
        .map(|(hash_type, hash, _)| (hash_type, hash))
        .collect();
    let res = client
        .get(base_url.join("/check_preimages_status")?)
        .body(serde_cbor::to_vec(&check_body)?)
        .send()
        .await?;

    tracing::info!("Preimage check status: {:?}", res.status());

    Ok(())
}
