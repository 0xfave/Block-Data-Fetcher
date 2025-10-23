/// ETL Pipeline Module
///
/// This module orchestrates the Extract, Transform, Load pipeline:
/// - Extract: Fetch block and transaction data from Solana RPC
/// - Transform: Parse and classify transactions
/// - Load: Store structured data in PostgreSQL database
pub mod extract;
pub mod load;
pub mod parsers;
pub mod transform;

use crate::{db::Database, rpc::SolanaRpcClient};
use anyhow::Result;

/// Main ETL pipeline coordinator
#[allow(dead_code)]
pub struct EtlPipeline {
    rpc_client: SolanaRpcClient,
    database: Database,
}

#[allow(dead_code)]
impl EtlPipeline {
    /// Create a new ETL pipeline
    pub fn new(rpc_client: SolanaRpcClient, database: Database) -> Self {
        Self { rpc_client, database }
    }

    /// Run the ETL pipeline for a range of slots
    pub async fn run(&self, start_slot: u64, end_slot: u64) -> Result<()> {
        tracing::info!("Starting ETL pipeline for slots {} to {}", start_slot, end_slot);

        // TODO: Implement pipeline stages
        // 1. Extract blocks from RPC
        // 2. Transform transaction data
        // 3. Load into database

        Ok(())
    }
}
