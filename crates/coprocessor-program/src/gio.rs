use std::env;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct GIORequest {
    domain: u16,
    id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GIOResponse {
    pub response_code: u16,
    pub response: String,
}

/// Retrieve the preimage for a given hash if it has already been populated
/// Attempting to retrieve a preimage that has not posted will halt execution
pub async fn get_preimage(hash: [u8; 32]) -> Result<Vec<u8>> {
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;
    let client = reqwest::Client::new();

    let request = GIORequest {
        domain: 0x2c,
        id: hex::encode(hash),
    };

    let res = client
        .get(format!("{}/gio", server_addr))
        .json(&request)
        .send()
        .await?;

    let response: GIOResponse = res.json().await?;
    Ok(hex::decode(response.response)?)
}
