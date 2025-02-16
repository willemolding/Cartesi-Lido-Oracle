//! The orchestrator is responsible for coordinating the production of the oracle reports
//!
//! This involves:
//! - Retrieving the beacon chain data via RPC and building inputs
//! - Uploading preimage data to operators
//! - Calling the contract to initiate the coprocessor execution

mod beacon_client;

use std::str::FromStr;

use alloy::{
    network::EthereumWallet,
    primitives::{Address, B256, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use anyhow::Result;
use beacon_client::BeaconClient;
use clap::Parser;
use ethereum_consensus::{phase0::SignedBeaconBlockHeader, types::mainnet::BeaconState};
use sha3::{Digest, Keccak256};
use ssz_rs::prelude::*;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use url::Url;

const CHUNK_SIZE: usize = 1024 * 256;
const KECCACK_HASH_TYPE: u8 = 2;

alloy::sol!(
    #[sol(rpc)]
    CartesiLidoOracle,
    "../../contracts/out/CartesiLidoOracle.sol/CartesiLidoOracle.json"
);

#[derive(Parser, Debug)]
struct Args {
    /// Ethereum beacon node HTTP RPC endpoint.
    #[clap(long, env)]
    beacon_rpc_url: Url,

    /// Coprocessor operator url
    #[clap(long, env)]
    operator_url: Url,

    /// Ethereum Node endpoint.
    #[clap(long, env)]
    eth_rpc_url: Url,

    /// Ethereum private key.
    #[clap(long, env)]
    eth_private_key: String,

    /// Ethereum contract address.
    #[clap(long, env)]
    contract_address: Address,

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
        operator_url,
        eth_rpc_url,
        eth_private_key,
        contract_address,
        ..
    } = Args::parse();

    tracing::info!("Fetching beacon block and state for slot {}", slot);
    let beacon_client = BeaconClient::new_with_cache(beacon_rpc_url, "./beacon-cache")?;
    let block_header = beacon_client.get_block_header(slot).await?;
    let beacon_state = beacon_client.get_beacon_state(slot).await?;

    let block_root = block_header.hash_tree_root()?.to_vec();

    tracing::info!("building inputs...");
    let inputs = build_inputs::<CHUNK_SIZE>(block_header, beacon_state);

    // upload the chunks, block data and manifest to the operators to use in the preimage oracle
    tracing::info!("Uploading to operator");
    upload_to_operator(operator_url, &inputs).await?;

    // call the contract to initiate the coprocessor execution
    let signer = PrivateKeySigner::from_str(eth_private_key.as_str())?;
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new().wallet(wallet).on_http(eth_rpc_url);
    let contract = CartesiLidoOracle::new(contract_address, provider);

    let tx_hash = contract
        .generateReportUntrusted(
            U256::from(slot),
            B256::from_slice(&block_root),
            inputs.get_manifest_hash().into(),
        )
        .send()
        .await?
        .watch()
        .await?;

    tracing::info!("Report generation initiated with tx hash: {:?}", tx_hash);

    Ok(())
}

#[derive(Debug)]
struct Inputs {
    manifest: io::Manifest,
    block_data: Vec<u8>,
    state_chunks: Vec<Vec<u8>>,
}

impl Inputs {
    fn get_manifest_hash(&self) -> [u8; 32] {
        keccak(&self.manifest.to_bytes().unwrap())
    }
}

fn build_inputs<const CHUNK_SIZE: usize>(
    beacon_block: SignedBeaconBlockHeader,
    beacon_state: BeaconState,
) -> Inputs {
    let mut block_data = Vec::new();
    beacon_block.serialize(&mut block_data).unwrap();
    let mut beacon_state_data = Vec::new();
    beacon_state.serialize(&mut beacon_state_data).unwrap();
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
            inputs.get_manifest_hash().to_vec(),
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

    // TODO: This should be batched but I am using an older version of the operator that doesn't support it
    for preimage in preimages.clone() {
        let res = client
            .post(base_url.join("/upload_preimages/")?)
            .body(serde_cbor::to_vec(&preimage)?)
            .send()
            .await?;

        if res.status() != 200 {
            let err = res.text().await?;
            tracing::debug!("Preimage upload response: {:?}", err);
            return Err(anyhow::anyhow!("Failed to upload preimages"));
        }
    }

    // sanity check the preimages are uploaded correctly
    let check_body: Vec<(u8, Vec<u8>)> = preimages
        .into_iter()
        .map(|(hash_type, hash, _)| (hash_type, hash))
        .collect();

    // TODO: Same here - batch once the operator supports it
    for check in check_body {
        let res = client
            .post(base_url.join("/check_preimages_status/")?)
            .body(serde_cbor::to_vec(&check)?)
            .send()
            .await?;

        let status = res.status();
        let text = res.text().await?;
        tracing::debug!("Preimage check response: {:?}", &text);

        if status != 200 {
            return Err(anyhow::anyhow!("Check showed preimages not uploaded"));
        }
    }

    Ok(())
}
