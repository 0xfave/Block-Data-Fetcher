/// RPC Client Module
///
/// This module handles all interactions with the Solana blockchain via RPC.
/// It wraps the Solana client and provides convenient methods for fetching
/// block and transaction data from Helius RPC endpoints.
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use solana_client::{rpc_client::RpcClient, rpc_config::RpcBlockConfig};
use solana_transaction_status::{TransactionDetails, UiConfirmedBlock, UiTransactionEncoding};

use crate::models::ConnectionInfo;

pub struct SolanaRpcClient {
    client: RpcClient,
    endpoint: String,
}

impl SolanaRpcClient {
    /// Create a new RPC client connected to the specified endpoint
    pub fn new(endpoint: String) -> Result<Self> {
        let client = RpcClient::new(endpoint.clone());

        Ok(Self { client, endpoint })
    }

    /// Get a reference to the underlying RPC client
    #[allow(dead_code)]
    pub fn client(&self) -> &RpcClient {
        &self.client
    }

    /// Get the endpoint URL this client is connected to
    #[allow(dead_code)]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Get connection information for display
    pub async fn get_connection_info(&self) -> Result<ConnectionInfo> {
        // Get latest blockhash
        let latest_blockhash = self.client.get_latest_blockhash().context("Failed to get latest blockhash")?;

        // Get current slot
        let slot = self.client.get_slot().context("Failed to get current slot")?;

        // Get block time for the current slot
        let block_time = self.client.get_block_time(slot).context("Failed to get block time")?;

        // Convert Unix timestamp to DateTime
        let timestamp = DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(Utc::now);

        Ok(ConnectionInfo { endpoint: self.endpoint.clone(), blockhash: latest_blockhash.to_string(), slot, timestamp })
    }

    /// Test the RPC connection
    pub async fn test_connection(&self) -> Result<()> {
        self.client.get_version().context("Failed to connect to RPC endpoint")?;
        Ok(())
    }

    /// Fetch a single block by slot number
    pub async fn fetch_block(&self, slot: u64) -> Result<UiConfirmedBlock> {
        tracing::debug!("Fetching block at slot {}", slot);

        let block = self
            .client
            .get_block_with_config(
                slot,
                RpcBlockConfig {
                    encoding: Some(UiTransactionEncoding::JsonParsed),
                    transaction_details: Some(TransactionDetails::Full),
                    rewards: Some(true),
                    commitment: None,
                    max_supported_transaction_version: Some(0),
                },
            )
            .context(format!("Failed to fetch block at slot {}", slot))?;

        tracing::info!("Successfully fetched block at slot {}", slot);
        Ok(block)
    }

    /// Get the latest confirmed slot
    pub async fn get_latest_slot(&self) -> Result<u64> {
        let slot = self.client.get_slot().context("Failed to get latest slot")?;

        Ok(slot)
    }
}
