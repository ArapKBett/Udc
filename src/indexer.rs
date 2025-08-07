use anyhow::{Result, anyhow};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use solana_sdk::signature::Signature;
use solana_transaction_status::{UiTransactionEncoding, UiTransactionTokenBalance, option_serializer::OptionSerializer};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, TimeZone};
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

    // Get current slot to calculate time range
    let current_slot = client.get_slot()?;

    // `get_block_time` returns Result<Option<i64>>
    let slot_time_opt = client.get_block_time(current_slot)?;
    let slot_time = slot_time_opt.ok_or_else(|| anyhow!("Failed to get current slot time"))?;

    let one_day_ago = slot_time - 86400; // 24 hours in seconds

    // Get all signatures for the wallet
    let signatures = client.get_signatures_for_address(&wallet_pubkey)?;

    let mut transfers = Vec::new();

    for sig_info in signatures {
        let block_time = match sig_info.block_time {
            Some(t) if t >= one_day_ago => t,
            _ => continue, // skip if no block time or too old
        };

        // Convert signature string to `Signature` type
        let signature = Signature::from_str(&sig_info.signature)?;

        let transaction = client.get_transaction(&signature, UiTransactionEncoding::Json)?;

        if let Some(meta) = transaction.transaction.meta {
            // Provide empty vector references to fix match arm type mismatches
            let empty_vec: &Vec<UiTransactionTokenBalance> = &vec![];

            let pre_balances = match &meta.pre_token_balances {
                OptionSerializer::Some(vec) => vec,
                _ => empty_vec,
            };

            let post_balances = match &meta.post_token_balances {
                OptionSerializer::Some(vec) => vec,
                _ => empty_vec,
            };

            for pre in pre_balances {
                if pre.mint != USDC_MINT {
                    continue;
                }

                for post in post_balances {
                    if post.mint != USDC_MINT {
                        continue;
                    }

                    // Extract owners safely wrapped in OptionSerializer
                    let pre_owner = match &pre.owner {
                        OptionSerializer::Some(owner) => owner,
                        _ => "",
                    };

                    let post_owner = match &post.owner {
                        OptionSerializer::Some(owner) => owner,
                        _ => "",
                    };

                    let pre_amount = pre.ui_token_amount.ui_amount.unwrap_or(0.0);
                    let post_amount = post.ui_token_amount.ui_amount.unwrap_or(0.0);

                    // Create DateTime<Utc> from unix timestamp (seconds)
                    let dt = Utc.timestamp_opt(block_time, 0)
                        .single()
                        .ok_or_else(|| anyhow!("Invalid timestamp"))?;

                    if pre_owner == wallet_address && post_owner != wallet_address {
                        // USDC sent out
                        transfers.push(UsdcTransfer {
                            date: dt,
                            amount: pre_amount - post_amount,
                            direction: "out".to_string(),
                            transaction_id: sig_info.signature.clone(),
                            other_party: post_owner.to_string(),
                        });
                    } else if pre_owner != wallet_address && post_owner == wallet_address {
                        // USDC received
                        transfers.push(UsdcTransfer {
                            date: dt,
                            amount: post_amount - pre_amount,
                            direction: "in".to_string(),
                            transaction_id: sig_info.signature.clone(),
                            other_party: pre_owner.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(transfers)
}
