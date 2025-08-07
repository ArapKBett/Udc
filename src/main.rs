mod indexer;
mod server;

use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let wallet_address = env::args()
        .nth(1)
        .unwrap_or_else(|| "7cMEhpt9y3inBNVv8fNnuaEbx7hKHZnLvR1KWKKxuDDU".to_string());

    // Run indexer to get USDC transfers
    let transfers = indexer::get_usdc_transfers(&wallet_address).await?;
    
    // Start web server to display results
    server::start_server(transfers).await?;

    Ok(())
}
