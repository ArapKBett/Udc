use askama::Template;
use serde::Serialize;
use std::sync::Arc;
use warp::Filter;

#[derive(Serialize, Clone)]
pub struct DisplayTransfer {
    pub date: String,
    pub amount: String,
    pub direction: String,
    pub transaction_id: String,
    pub other_party: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    wallet_address: String,
    transfers: Vec<DisplayTransfer>,
}

pub async fn start_server(transfers: Vec<crate::indexer::UsdcTransfer>) -> anyhow::Result<()> {
    let transfers = Arc::new(transfers);

    // Convert to display-friendly format
    let display_transfers: Vec<DisplayTransfer> = transfers.iter().map(|t| {
        DisplayTransfer {
            date: t.date.format("%Y-%m-%d %H:%M:%S").to_string(),
            amount: format!("{:.2}", t.amount),
            direction: t.direction.clone(),
            transaction_id: t.transaction_id.clone(),
            other_party: t.other_party.clone(),
        }
    }).collect();

    // GET / endpoint
    let index = warp::path::end().map(move || {
        let template = IndexTemplate {
            wallet_address: "7cMEhpt9y3inBNVv8fNnuaEbx7hKHZnLvR1KWKKxuDDU".to_string(),
            transfers: display_transfers.to_vec(),
        };
        warp::reply::html(template.render().unwrap())
    });

    println!("Server running on http://localhost:8080");
    warp::serve(index).run(([0, 0, 0, 0], 8080)).await;

    Ok(())
}