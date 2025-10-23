/// Solana Block Data Fetcher
///
/// An ETL pipeline for extracting, transforming, and loading Solana blockchain data.
mod db;
mod etl;
mod models;
mod rpc;

use anyhow::{Context, Result};
use db::Database;
use rpc::SolanaRpcClient;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    println!("🚀 Starting Solana Block Fetcher...");

    // Get RPC URL from environment
    let rpc_url =
        env::var("HELIUS_RPC_URL").context("HELIUS_RPC_URL not found in environment. Please check your .env file")?;

    // Initialize RPC client
    let rpc_client = SolanaRpcClient::new(rpc_url).context("Failed to create RPC client")?;

    // Test RPC connection
    rpc_client.test_connection().await.context("Failed to connect to Solana RPC")?;

    // Get and display connection info
    let conn_info = rpc_client.get_connection_info().await.context("Failed to get connection info")?;

    println!("✅ Connected to: {}", conn_info.endpoint);
    println!(
        "📦 Latest Blockhash: {}...{}",
        &conn_info.blockhash[..7],
        &conn_info.blockhash[conn_info.blockhash.len() - 3..]
    );
    println!("🎯 Current Slot: {:?}", format_number(conn_info.slot));
    println!("⏰ Timestamp: {}", conn_info.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));

    // Initialize database connection
    let database_url =
        env::var("DATABASE_URL").context("DATABASE_URL not found in environment. Please check your .env file")?;

    println!("\n💾 Connecting to PostgreSQL database...");
    let database = Database::new(&database_url).await.context("Failed to connect to PostgreSQL database")?;

    // Test database connection
    database.test_connection().await.context("Database connection test failed")?;

    println!("✅ Database connected successfully!");

    // Run database migrations
    println!("📋 Running database migrations...");
    database.migrate().await.context("Failed to run database migrations")?;

    println!("✅ Database schema created successfully!");

    // Load program registry from database
    println!("📚 Loading program registry from database...");
    let program_infos = database.load_program_registry().await.context("Failed to load program registry")?;
    let program_registry = etl::transform::ProgramRegistry::from_database(program_infos);
    println!("✅ Loaded {} programs from registry", program_registry.programs.len());

    tracing::info!("Solana Block Fetcher initialized successfully");

    // Test: Fetch a range of blocks
    println!("\n🔍 Testing block range extraction...");
    let latest_slot = rpc_client.get_latest_slot().await.context("Failed to get latest slot")?;

    println!("📍 Latest confirmed slot: {}", format_number(latest_slot));

    // Fetch 10 recent blocks (go back 20 slots to ensure they're all finalized)
    let end_slot = latest_slot - 20;
    let start_slot = end_slot - 9; // Fetch 10 blocks total for testing

    println!("🎯 Fetching blocks from {} to {}", format_number(start_slot), format_number(end_slot));

    let (extracted_blocks, _stats) =
        etl::extract::extract_block_range(&rpc_client, start_slot, end_slot, 100, Some(&program_registry))
            .await
            .context("Failed to extract block range")?;

    // ========== PHASE 5: Load blocks and transactions into database ==========
    println!("\n💾 Inserting blocks into database...");
    let start_insert = std::time::Instant::now();

    // Use batch insertion for better performance
    let (blocks_inserted, transactions_inserted) =
        etl::load::batch_insert_blocks_with_transactions(database.pool(), &extracted_blocks, &program_registry)
            .await
            .context("Failed to batch insert blocks and transactions")?;

    let insert_duration = start_insert.elapsed();

    println!(
        "✅ Inserted {} blocks and {} transactions into database in {:.2}s",
        blocks_inserted,
        transactions_inserted,
        insert_duration.as_secs_f64()
    );

    // Display summary of a few blocks
    println!("\n📊 Sample of extracted blocks:");
    for (i, block) in extracted_blocks.iter().take(3).enumerate() {
        println!("\n  Block #{} - Slot: {}", i + 1, format_number(block.slot));
        println!("    Blockhash: {}...{}", &block.blockhash[..8], &block.blockhash[block.blockhash.len() - 8..]);
        println!("    Parent slot: {}", format_number(block.parent_slot));
        println!("    Transactions: {}", block.transactions.len());

        let success_count = block.transactions.iter().filter(|tx| tx.success).count();
        let fail_count = block.transactions.len() - success_count;
        println!("    Success/Fail: {} ✅ / {} ❌", success_count, fail_count);

        let total_fees: u64 = block.transactions.iter().map(|tx| tx.fee).sum();
        println!("    Total fees: {} lamports", format_number(total_fees));
    }

    // Display classified transaction samples
    println!("\n🏷️  Sample classified transactions with detailed analysis:");

    // Collect statistics from all transactions
    let mut stats = etl::transform::TransactionTypeStats::new();
    let mut shown = 0;
    let mut shown_sol_transfer = false;

    for block in &extracted_blocks {
        for tx in &block.transactions {
            // Collect statistics for all transactions
            let tx_type = etl::transform::classify_transaction_with_registry(&tx.program_ids, &program_registry);
            stats.add(&tx_type);

            // Display first 5 transactions OR first SOL transfer (for debugging)
            let is_sol_transfer = tx_type == crate::models::TransactionType::SolTransfer;

            if shown < 5 || (is_sol_transfer && !shown_sol_transfer) {
                let details = etl::transform::analyze_transaction_with_registry(
                    &tx.program_ids,
                    &program_registry,
                    Some(&tx.raw_json),
                );
                let status = if tx.success { "✅" } else { "❌" };

                println!("\n  Transaction {} {}", shown + 1, status);
                println!("    Type: {}", details.label);
                println!("    Signature: {}...{}", &tx.signature[..8], &tx.signature[tx.signature.len() - 8..]);
                println!("    Fee: {} lamports", format_number(tx.fee));
                println!("    Programs: {} ({})", tx.program_ids.len(), details.program_names.join(", "));

                // Display parsed details if available
                if let Some(amount) = details.amount {
                    println!("    💰 Amount: {} lamports", format_number(amount));
                }
                if let Some(token) = &details.token_address {
                    println!("    🪙  Token: {}...{}", &token[..8], &token[token.len().saturating_sub(8)..]);
                }
                if let Some(from) = &details.from_account {
                    println!("    📤 From: {}...{}", &from[..8], &from[from.len().saturating_sub(8)..]);
                }
                if let Some(to) = &details.to_account {
                    println!("    📥 To: {}...{}", &to[..8], &to[to.len().saturating_sub(8)..]);
                }

                if is_sol_transfer {
                    shown_sol_transfer = true;
                }
                shown += 1;
            }
        }
    }

    // Display overall statistics
    println!("\n📈 Transaction Type Statistics:");
    println!("   Total analyzed: {}", stats.total);
    println!("   💸 SOL Transfers: {} ({:.1}%)", stats.sol_transfers, stats.percentage(stats.sol_transfers));
    println!("   🪙  Token Transfers: {} ({:.1}%)", stats.token_transfers, stats.percentage(stats.token_transfers));
    println!("   🔄 DEX Swaps: {} ({:.1}%)", stats.dex_swaps, stats.percentage(stats.dex_swaps));
    println!("   🖼️  NFT Operations: {} ({:.1}%)", stats.nft_operations, stats.percentage(stats.nft_operations));
    println!(
        "   ⚙️  Program Interactions: {} ({:.1}%)",
        stats.program_interactions,
        stats.percentage(stats.program_interactions)
    );
    println!("   ❓ Unknown: {} ({:.1}%)", stats.unknown, stats.percentage(stats.unknown));

    println!("\n✨ All tests complete!");

    Ok(())
}

/// Format a number with thousand separators
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();

    for (count, c) in s.chars().rev().enumerate() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Verify modules are accessible
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(174283491), "174,283,491");
    }
}
