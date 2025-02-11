use std::env;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::FutureExt;
use tower_cartesi::{listen_http, Request, Response};
use tower_service::Service;

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
                println!(
                    "Received advance state request {:?} \npayload {:?}:",
                    metadata, payload
                );

                let mut response = Response::empty_accept();
                response.add_notice(&Vec::new());

                async { Ok(response) }.boxed()
            }
            _ => {
                tracing::error!("Received unexpected request: {:?}", req);
                async { Ok(Response::empty_reject()) }.boxed()
            }
        }
    }
}
