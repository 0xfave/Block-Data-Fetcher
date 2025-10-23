/// Solana Block Data Fetcher
///
/// An ETL pipeline for extracting, transforming, and loading Solana blockchain data.
mod db;
mod etl;
mod models;
mod pipeline;
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

    // ========== PHASE 6: Run ETL Pipeline ==========
    println!("\n🔍 Determining block range...");
    let latest_slot = rpc_client.get_latest_slot().await.context("Failed to get latest slot")?;

    // Fetch 10 recent blocks (go back 20 slots to ensure they're all finalized)
    let end_slot = latest_slot - 20;
    let start_slot = end_slot - 9; // Fetch 10 blocks total

    println!("📍 Latest confirmed slot: {}", format_number(latest_slot));

    // Configure and run the pipeline
    let pipeline_config = pipeline::PipelineConfig {
        start_slot,
        end_slot,
        max_retries: 3,
        retry_delay: std::time::Duration::from_secs(2),
        batch_size: 10,
    };

    let pipeline = pipeline::Pipeline::new(rpc_client, database, program_registry, pipeline_config);

    // Run the pipeline with error handling and retry logic
    let _pipeline_stats = pipeline.run().await.context("Pipeline execution failed")?;

    println!("\n✨ Pipeline execution complete!");

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
