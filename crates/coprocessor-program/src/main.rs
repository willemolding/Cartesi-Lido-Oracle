mod gio;

use std::env;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use alloy_sol_types::SolValue;
use anyhow::Result;
use ethereum_consensus::{phase0::SignedBeaconBlockHeader, types::mainnet::BeaconState};
use futures_util::FutureExt;
use gio::get_preimage;
use io::{derive_report, Input, Manifest, Report};
use ssz_rs::prelude::*;
use tower_cartesi_coprocessor::{listen_http, Request, Response};
use tower_service::Service;

type BoxError = Box<dyn Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

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

    tracing::debug!("Manifest: {:?}", manifest);

    let block = SignedBeaconBlockHeader::deserialize(&get_preimage(manifest.block_hash).await?)?;

    tracing::debug!("Successfully loaded beacon block: {:?}", block);

    let mut state_bytes = Vec::new();
    for chunk_hash in manifest.state_chunk_hashes {
        state_bytes.extend_from_slice(&get_preimage(chunk_hash).await?);
    }
    let state = BeaconState::deserialize(&state_bytes)?;

    tracing::debug!("Successfully loaded beacon state");

    // calculate the block root and ensure it matches the input
    tracing::debug!("Calculating block root and checking against input");
    let block_root = block.hash_tree_root()?;
    assert_eq!(block_root, *input.block_root);

    // calculate the state root and ensure it is in the block
    tracing::debug!("Calculating state root and checking state root in block");
    let state_root = state.hash_tree_root()?;
    assert_eq!(state_root, block.message.state_root);

    // now we can trust the data in the state and use it to make a report
    tracing::debug!("Generating report...");
    let report = derive_report(&state);

    Ok(report)
}
