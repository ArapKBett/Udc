use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::str::FromStr;

// USDC Mint Address
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

#[derive(Debug, Serialize, Deserialize)]
pub struct UsdcTransfer {
    pub date: DateTime<Utc>,
    pub amount: f64,
    pub direction: String, // "in" or "out"
    pub transaction_id: String,
    pub other_party: String,
}

pub async fn get_usdc_transfers(wallet_address: &str) -> Result<Vec<UsdcTransfer>> {
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    let wallet_pubkey = Pubkey::from_str(wallet_address)?;
    let usdc_mint_pubkey = Pubkey::from_str(USDC_MINT)?;

    // Get current slot to calculate time range
    let current_slot = client.get_slot()?;
    let slot_time = client.get_block_time(current_slot)?;
    let one_day_ago = slot_time - 86400; // 24 hours in seconds

    // Get all signatures for the wallet
    let signatures = client.get_signatures_for_address(
        &wallet_pubkey,
        Some(serde_json::json!({
            "limit": 1000,
            "before": current_slot,
        })),
    )?;

    let mut transfers = Vec::new();

    for sig in signatures {
        if sig.block_time.is_none() || sig.block_time.unwrap() < one_day_ago {
            continue;
        }

        let transaction = client.get_transaction(
            &sig.signature.parse()?,
            solana_transaction_status::UiTransactionEncoding::Json,
        )?;

        if let Some(meta) = transaction.meta {
            if let Some(pre_token_balances) = meta.pre_token_balances {
                if let Some(post_token_balances) = meta.post_token_balances {
                    for (pre, post) in pre_token_balances.iter().zip(post_token_balances.iter()) {
                        if pre.mint == USDC_MINT || post.mint == USDC_MINT {
                            let pre_owner = &pre.owner;
                            let post_owner = &post.owner;
                            let pre_amount = pre.ui_token_amount.ui_amount.unwrap_or(0.0);
                            let post_amount = post.ui_token_amount.ui_amount.unwrap_or(0.0);

                            if pre_owner == wallet_address && post_owner != wallet_address {
                                // USDC sent out
                                transfers.push(UsdcTransfer {
                                    date: DateTime::from_timestamp(sig.block_time.unwrap(), 0).unwrap(),
                                    amount: pre_amount - post_amount,
                                    direction: "out".to_string(),
                                    transaction_id: sig.signature.clone(),
                                    other_party: post_owner.clone(),
                                });
                            } else if pre_owner != wallet_address && post_owner == wallet_address {
                                // USDC received
                                transfers.push(UsdcTransfer {
                                    date: DateTime::from_timestamp(sig.block_time.unwrap(), 0).unwrap(),
                                    amount: post_amount - pre_amount,
                                    direction: "in".to_string(),
                                    transaction_id: sig.signature.clone(),
                                    other_party: pre_owner.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(transfers)
          }
