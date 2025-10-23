/// Extract Module
///
/// Handles fetching data from the Solana blockchain via RPC and parsing transaction details.
use anyhow::{Context, Result};
use solana_transaction_status::{EncodedTransactionWithStatusMeta, UiConfirmedBlock};
use std::time::Duration;
use tokio::time::sleep;

/// Extracted transaction data from a block
#[derive(Debug, Clone)]
pub struct ExtractedTransaction {
    pub signature: String,
    pub success: bool,
    pub fee: u64,
    #[allow(dead_code)]
    pub num_accounts: usize,
    #[allow(dead_code)]
    pub num_instructions: usize,
    pub program_ids: Vec<String>,    // Program IDs involved in the transaction
    pub raw_json: serde_json::Value, // Full transaction JSON for detailed parsing
}

/// Extracted block data with parsed transactions
#[derive(Debug, Clone)]
pub struct ExtractedBlock {
    pub slot: u64,
    pub blockhash: String,
    pub parent_slot: u64,
    #[allow(dead_code)]
    pub block_time: Option<i64>,
    #[allow(dead_code)]
    pub block_height: Option<u64>,
    pub transactions: Vec<ExtractedTransaction>,
}

/// Statistics for a range extraction
#[derive(Debug, Clone)]
pub struct ExtractionStats {
    pub blocks_fetched: u64,
    pub blocks_failed: u64,
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub total_fees: u64,
    // Transaction type counts
    pub sol_transfers: u64,
    pub spl_token_transfers: u64,
    pub dex_swaps: u64,
    pub nft_operations: u64,
    pub program_interactions: u64,
    pub unknown_transactions: u64,
}

/// Parse transactions from a block
pub fn parse_transactions_from_block(block: &UiConfirmedBlock) -> Result<Vec<ExtractedTransaction>> {
    let transactions = block.transactions.as_ref().context("Block has no transactions")?;

    let mut extracted_transactions = Vec::new();

    for (index, tx) in transactions.iter().enumerate() {
        match parse_single_transaction(tx, index) {
            Ok(extracted) => extracted_transactions.push(extracted),
            Err(e) => {
                tracing::warn!("Failed to parse transaction at index {}: {}", index, e);
                continue;
            }
        }
    }

    Ok(extracted_transactions)
}

/// Parse a single transaction - simplified for Solana SDK v2.0
fn parse_single_transaction(tx: &EncodedTransactionWithStatusMeta, _index: usize) -> Result<ExtractedTransaction> {
    // Extract meta information to determine success and fee
    let meta = tx.meta.as_ref().context("Transaction has no metadata")?;

    // Determine success (if err is None, transaction succeeded)
    let success = meta.err.is_none();

    // Extract fee
    let fee = meta.fee;

    // For now, we'll extract the signature from the encoded transaction as a JSON string
    // The actual structure varies based on encoding (Json, Base58, Base64)
    let tx_json = serde_json::to_value(&tx.transaction).context("Failed to serialize transaction to JSON")?;

    // Try to extract signature from the JSON structure
    let signature = if let Some(sigs) = tx_json.get("signatures").and_then(|s| s.as_array()) {
        sigs.first().and_then(|s| s.as_str()).unwrap_or("unknown").to_string()
    } else {
        "unknown".to_string()
    };

    // Count accounts from the message structure
    let num_accounts = if let Some(message) = tx_json.get("message") {
        message.get("accountKeys").and_then(|a| a.as_array()).map(|a| a.len()).unwrap_or(0)
    } else {
        0
    };

    // Count instructions from the message structure
    let num_instructions = if let Some(message) = tx_json.get("message") {
        message.get("instructions").and_then(|i| i.as_array()).map(|i| i.len()).unwrap_or(0)
    } else {
        0
    };

    // Extract program IDs from instructions
    let program_ids = extract_program_ids(&tx_json);

    Ok(ExtractedTransaction { signature, success, fee, num_accounts, num_instructions, program_ids, raw_json: tx_json })
}

/// Extract program IDs from transaction JSON
fn extract_program_ids(tx_json: &serde_json::Value) -> Vec<String> {
    let mut program_ids = Vec::new();

    if let Some(message) = tx_json.get("message") {
        // Get account keys (which include program accounts)
        let account_keys = message.get("accountKeys").and_then(|a| a.as_array());

        // Get instructions
        if let Some(instructions) = message.get("instructions").and_then(|i| i.as_array()) {
            for instruction in instructions {
                // For JsonParsed encoding, check for programId field first
                if let Some(program_id) = instruction.get("programId").and_then(|p| p.as_str()) {
                    if !program_ids.contains(&program_id.to_string()) {
                        program_ids.push(program_id.to_string());
                    }
                }
                // For parsed instructions, also check the program field (program name like "system", "spl-token")
                else if let Some(program) = instruction.get("program").and_then(|p| p.as_str()) {
                    // This is a program name, not ID - we'll skip it for now since we have programId
                    let program_id = program.to_string();
                    if !program_ids.contains(&program_id) {
                        program_ids.push(program_id);
                    }
                }
                // For compiled instructions, use programIdIndex to look up in accountKeys
                else if let Some(program_idx) = instruction.get("programIdIndex").and_then(|i| i.as_u64()) {
                    if let Some(keys) = account_keys {
                        if let Some(key) = keys.get(program_idx as usize) {
                            if let Some(pubkey) = key.get("pubkey").and_then(|p| p.as_str()) {
                                let program_id = pubkey.to_string();
                                if !program_ids.contains(&program_id) {
                                    program_ids.push(program_id);
                                }
                            } else if let Some(pubkey_str) = key.as_str() {
                                let program_id = pubkey_str.to_string();
                                if !program_ids.contains(&program_id) {
                                    program_ids.push(program_id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    program_ids
}

/// Extract a single block with parsed transactions
pub async fn extract_block(rpc_client: &crate::rpc::SolanaRpcClient, slot: u64) -> Result<ExtractedBlock> {
    // Fetch the block from RPC
    let block = rpc_client.fetch_block(slot).await.context(format!("Failed to fetch block at slot {}", slot))?;

    // Parse transactions
    let transactions = parse_transactions_from_block(&block)?;

    // Extract block metadata
    let extracted_block = ExtractedBlock {
        slot,
        blockhash: block.blockhash,
        parent_slot: block.parent_slot,
        block_time: block.block_time,
        block_height: block.block_height,
        transactions,
    };

    Ok(extracted_block)
}

/// Extract a range of blocks with rate limiting and progress tracking
pub async fn extract_block_range(
    rpc_client: &crate::rpc::SolanaRpcClient,
    start_slot: u64,
    end_slot: u64,
    rate_limit_ms: u64,
    registry: Option<&crate::etl::transform::ProgramRegistry>,
) -> Result<(Vec<ExtractedBlock>, ExtractionStats)> {
    if start_slot > end_slot {
        anyhow::bail!("Start slot {} is greater than end slot {}", start_slot, end_slot);
    }

    let total_blocks = end_slot - start_slot + 1;
    println!("\n🔄 Starting block range extraction...");
    println!("   Start slot: {}", format_number(start_slot));
    println!("   End slot: {}", format_number(end_slot));
    println!("   Total blocks: {}", format_number(total_blocks));
    println!("   Rate limit: {}ms between requests", rate_limit_ms);

    let mut extracted_blocks = Vec::new();
    let mut stats = ExtractionStats {
        blocks_fetched: 0,
        blocks_failed: 0,
        total_transactions: 0,
        successful_transactions: 0,
        failed_transactions: 0,
        total_fees: 0,
        sol_transfers: 0,
        spl_token_transfers: 0,
        dex_swaps: 0,
        nft_operations: 0,
        program_interactions: 0,
        unknown_transactions: 0,
    };

    let start_time = std::time::Instant::now();

    for slot in start_slot..=end_slot {
        // Progress indicator
        let progress = stats.blocks_fetched + stats.blocks_failed + 1;
        if progress.is_multiple_of(10) || progress == total_blocks {
            let elapsed = start_time.elapsed().as_secs_f64();
            let blocks_per_sec = progress as f64 / elapsed;
            let eta_secs = ((total_blocks - progress) as f64 / blocks_per_sec).ceil() as u64;

            println!(
                "   📊 Progress: {}/{} blocks ({:.1}%) | {:.2} blocks/sec | ETA: {}s",
                progress,
                total_blocks,
                (progress as f64 / total_blocks as f64) * 100.0,
                blocks_per_sec,
                eta_secs
            );
        }

        // Fetch and parse block
        match extract_block(rpc_client, slot).await {
            Ok(block) => {
                // Update statistics
                stats.blocks_fetched += 1;
                stats.total_transactions += block.transactions.len() as u64;

                for tx in &block.transactions {
                    if tx.success {
                        stats.successful_transactions += 1;
                    } else {
                        stats.failed_transactions += 1;
                    }
                    stats.total_fees += tx.fee;

                    // Classify transaction using registry if available, otherwise use legacy method
                    let tx_type = if let Some(reg) = registry {
                        crate::etl::transform::classify_transaction_with_registry(&tx.program_ids, reg)
                    } else {
                        crate::etl::transform::classify_transaction(&tx.program_ids)
                    };

                    match tx_type {
                        crate::models::TransactionType::SolTransfer => stats.sol_transfers += 1,
                        crate::models::TransactionType::SplTokenTransfer => stats.spl_token_transfers += 1,
                        crate::models::TransactionType::DexSwap => stats.dex_swaps += 1,
                        crate::models::TransactionType::NftMint | crate::models::TransactionType::NftTransfer => {
                            stats.nft_operations += 1
                        }
                        crate::models::TransactionType::ProgramInteraction => stats.program_interactions += 1,
                        crate::models::TransactionType::Unknown => stats.unknown_transactions += 1,
                    }
                }

                extracted_blocks.push(block);
            }
            Err(e) => {
                stats.blocks_failed += 1;
                tracing::warn!("Failed to extract block at slot {}: {}", slot, e);
            }
        }

        // Rate limiting (skip on last block)
        if slot < end_slot && rate_limit_ms > 0 {
            sleep(Duration::from_millis(rate_limit_ms)).await;
        }
    }

    let total_time = start_time.elapsed().as_secs_f64();
    let avg_blocks_per_sec = stats.blocks_fetched as f64 / total_time;

    println!("\n✅ Block range extraction complete!");
    println!("   ⏱️  Total time: {:.2}s", total_time);
    println!("   📦 Blocks fetched: {}", format_number(stats.blocks_fetched));
    println!("   ❌ Blocks failed: {}", format_number(stats.blocks_failed));
    println!("   ⚡ Average speed: {:.2} blocks/sec", avg_blocks_per_sec);
    println!("   📝 Total transactions: {}", format_number(stats.total_transactions));
    println!("   ✅ Successful: {}", format_number(stats.successful_transactions));
    println!("   ❌ Failed: {}", format_number(stats.failed_transactions));
    println!("   💰 Total fees: {} SOL", (stats.total_fees as f64 / 1_000_000_000.0));

    // Transaction type breakdown
    println!("\n📊 Transaction Classification:");
    println!("   💸 SOL Transfers: {}", format_number(stats.sol_transfers));
    println!("   🪙  Token Transfers: {}", format_number(stats.spl_token_transfers));
    println!("   🔄 DEX Swaps: {}", format_number(stats.dex_swaps));
    println!("   🖼️  NFT Operations: {}", format_number(stats.nft_operations));
    println!("   ⚙️  Program Interactions: {}", format_number(stats.program_interactions));
    println!("   ❓ Unknown: {}", format_number(stats.unknown_transactions));

    Ok((extracted_blocks, stats))
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
