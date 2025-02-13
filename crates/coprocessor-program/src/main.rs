mod gio;

use std::env;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use alloy_primitives::B256;
use alloy_sol_types::SolValue;
use anyhow::Result;
use ethereum_consensus::{phase0::SignedBeaconBlockHeader, types::mainnet::BeaconState};
use futures_util::FutureExt;
use gio::get_preimage;
use io::{Input, Manifest, Report};
use ssz_rs::prelude::*;
use tower_cartesi::{listen_http, Request, Response};
use tower_service::Service;

// mainnet
pub const WITHDRAWAL_CREDENTIALS: B256 = B256::new([
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xb9, 0xd7, 0x93, 0x48,
    0x78, 0xb5, 0xfb, 0x96, 0x10, 0xb3, 0xfe, 0x8a, 0x5e, 0x44, 0x1e, 0x8f, 0xad, 0x7e, 0x29, 0x3f,
]);

type BoxError = Box<dyn Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let mut app = LidoOracleApp;
    tracing::info!("Listening on: {}", server_addr);
    listen_http(&mut app, &server_addr).await?;

    Ok(())
}

struct LidoOracleApp;

impl Service<Request> for LidoOracleApp {
    type Response = Response;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req {
            Request::AdvanceState { metadata, payload } => {
                tracing::info!(
                    "Received advance state request {:?} \npayload {:?}:",
                    metadata,
                    payload
                );

                async {
                    let report = run_oracle(payload).await?;

                    tracing::info!("Derived report: {:?}", report);

                    let mut response = Response::empty_accept();
                    response.add_notice(&report.abi_encode());

                    Ok(response)
                }
                .boxed()
            }
            _ => {
                tracing::error!("Received unexpected request: {:?}", req);
                async { Ok(Response::empty_reject()) }.boxed()
            }
        }
    }
}

/// Perform the input processing, GIO requests and derivation of the report from the state
async fn run_oracle(input: Vec<u8>) -> Result<Report> {
    let input = Input::abi_decode(&input, true)?;

    let manifest = Manifest::from_bytes(&get_preimage(*input.manifest_hash).await?)?;
    let block: SignedBeaconBlockHeader =
        serde_cbor::from_slice(&get_preimage(*input.block_root).await?)?;

    let mut state_bytes = Vec::new();
    for chunk_hash in manifest.state_chunk_hashes {
        state_bytes.extend_from_slice(&get_preimage(chunk_hash).await?);
    }
    let state: BeaconState = serde_cbor::from_slice(&state_bytes)?;

    // calculate the block root and ensure it matches the input
    let block_root = block.hash_tree_root()?;
    assert_eq!(block_root, *input.block_root);

    // calculate the state root and ensure it is in the block
    let state_root = state.hash_tree_root()?;
    assert_eq!(state_root, block.message.state_root);

    // now we can trust the data in the state and use it to make a report
    let report = derive_report(state);

    Ok(report)
}

fn derive_report(state: BeaconState) -> Report {
    // total balance of all Lido validators
    let (cl_balance_gewi, total_deposited) = state
        .validators()
        .iter()
        .zip(state.balances().iter())
        .filter(|(v, _)| v.withdrawal_credentials.as_slice() == WITHDRAWAL_CREDENTIALS.as_slice())
        .fold(
            (0u64, 0u64),
            |(cl_balance_gwei, total_deposited), (_, bal)| {
                (cl_balance_gwei + bal, total_deposited + 1)
            },
        );

    Report {
        clBalanceGwei: U256::from(cl_balance_gewi),
        withdrawalVaultBalanceWei: U256::ZERO,
        totalDepositedValidators: U256::from(total_deposited),
        totalExitedValidators: U256::ZERO,
    }
}
