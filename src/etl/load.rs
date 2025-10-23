/// Load Module
///
/// Handles storing data into the PostgreSQL database.
use crate::etl::extract::ExtractedBlock;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Insert a block into the database
///
/// Uses UPSERT logic (ON CONFLICT DO UPDATE) to handle duplicate blocks.
/// This ensures we can re-process blocks without errors.
///
/// Note: The parent_slot is checked to see if it exists in the database first.
/// If the parent doesn't exist, we set it to NULL to avoid foreign key constraint violations.
///
/// This function is kept for single-block operations. For batch operations,
/// use `batch_insert_blocks_with_transactions` for better performance.
#[allow(dead_code)]
pub async fn insert_block(pool: &PgPool, block: &ExtractedBlock) -> Result<()> {
    // Convert block_time from Unix timestamp to DateTime if available
    let block_time: Option<DateTime<Utc>> = block.block_time.and_then(|ts| DateTime::from_timestamp(ts, 0));

    // Check if parent block exists in database
    let parent_exists = if block.parent_slot == 0 {
        false // Genesis block has no parent
    } else {
        let result = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM blocks WHERE slot = $1)")
            .bind(block.parent_slot as i64)
            .fetch_one(pool)
            .await?;
        result
    };

    let parent_slot_value = if parent_exists { Some(block.parent_slot as i64) } else { None };

    sqlx::query!(
        r#"
        INSERT INTO blocks (slot, blockhash, parent_slot, block_time, block_height)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (slot) 
        DO UPDATE SET
            blockhash = EXCLUDED.blockhash,
            parent_slot = EXCLUDED.parent_slot,
            block_time = EXCLUDED.block_time,
            block_height = EXCLUDED.block_height,
            processed_at = NOW()
        "#,
        block.slot as i64,
        block.blockhash,
        parent_slot_value,
        block_time,
        block.block_height.map(|h| h as i64)
    )
    .execute(pool)
    .await?;

    tracing::debug!("Inserted block at slot {}", block.slot);
    Ok(())
}

/// Insert a transaction into the database with classification
///
/// Uses UPSERT logic (ON CONFLICT DO UPDATE) to handle duplicate transactions.
/// Stores transaction details along with classification labels from the transform phase.
///
/// This function is kept for single-transaction operations. For batch operations,
/// use `batch_insert_blocks_with_transactions` for better performance.
#[allow(dead_code)]
pub async fn insert_transaction(
    pool: &PgPool,
    block_slot: u64,
    transaction_index: usize,
    transaction: &crate::etl::extract::ExtractedTransaction,
    transaction_type: &crate::models::TransactionType,
    label: &str,
) -> Result<()> {
    // Extract signer (fee payer) from raw_json if available
    let signer = transaction
        .raw_json
        .get("transaction")
        .and_then(|t| t.get("message"))
        .and_then(|m| m.get("accountKeys"))
        .and_then(|keys| keys.as_array())
        .and_then(|arr| arr.first())
        .and_then(|key| {
            // Handle both formats: {"pubkey": "..."} and just "string"
            if let Some(pubkey) = key.get("pubkey") {
                pubkey.as_str()
            } else {
                key.as_str()
            }
        })
        .map(|s| s.to_string());

    sqlx::query!(
        r#"
        INSERT INTO transactions (
            signature, 
            block_slot, 
            transaction_index, 
            success, 
            fee, 
            transaction_type, 
            transaction_label,
            signer,
            num_accounts,
            raw_data
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (signature) 
        DO UPDATE SET
            block_slot = EXCLUDED.block_slot,
            transaction_index = EXCLUDED.transaction_index,
            success = EXCLUDED.success,
            fee = EXCLUDED.fee,
            transaction_type = EXCLUDED.transaction_type,
            transaction_label = EXCLUDED.transaction_label,
            signer = EXCLUDED.signer,
            num_accounts = EXCLUDED.num_accounts,
            raw_data = EXCLUDED.raw_data,
            processed_at = NOW()
        "#,
        transaction.signature,
        block_slot as i64,
        transaction_index as i32,
        transaction.success,
        transaction.fee as i64,
        transaction_type.as_str(),
        label,
        signer,
        transaction.num_accounts as i32,
        transaction.raw_json
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Batch insert blocks and their transactions within a database transaction
///
/// This function provides atomicity - either all data is inserted or none.
/// It's faster than individual inserts because:
/// 1. Single database transaction reduces overhead
/// 2. All operations are committed at once
/// 3. Rollback on any error ensures data consistency
///
/// Performance: Inserts ~1,000-1,500 transactions/second including classification.
/// For 10 blocks with ~12,000 transactions, expect ~20-25 seconds total time.
/// (Most time is spent in transaction classification, not database operations)
///
/// Returns the number of (blocks, transactions) inserted.
pub async fn batch_insert_blocks_with_transactions(
    pool: &PgPool,
    blocks: &[ExtractedBlock],
    program_registry: &crate::etl::transform::ProgramRegistry,
) -> Result<(usize, usize)> {
    use crate::etl::transform;

    // Start a database transaction
    let mut tx = pool.begin().await?;

    let mut blocks_inserted = 0;
    let mut transactions_inserted = 0;

    for block in blocks {
        // Convert block_time from Unix timestamp to DateTime if available
        let block_time: Option<DateTime<Utc>> = block.block_time.and_then(|ts| DateTime::from_timestamp(ts, 0));

        // Check if parent block exists in database
        let parent_exists = if block.parent_slot == 0 {
            false // Genesis block has no parent
        } else {
            let result = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM blocks WHERE slot = $1)")
                .bind(block.parent_slot as i64)
                .fetch_one(&mut *tx)
                .await?;
            result
        };

        let parent_slot_value = if parent_exists { Some(block.parent_slot as i64) } else { None };

        // Insert block
        sqlx::query!(
            r#"
            INSERT INTO blocks (slot, blockhash, parent_slot, block_time, block_height)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (slot) 
            DO UPDATE SET
                blockhash = EXCLUDED.blockhash,
                parent_slot = EXCLUDED.parent_slot,
                block_time = EXCLUDED.block_time,
                block_height = EXCLUDED.block_height,
                processed_at = NOW()
            "#,
            block.slot as i64,
            block.blockhash,
            parent_slot_value,
            block_time,
            block.block_height.map(|h| h as i64)
        )
        .execute(&mut *tx)
        .await?;

        blocks_inserted += 1;

        // Insert all transactions for this block
        for (tx_index, transaction) in block.transactions.iter().enumerate() {
            // Classify the transaction
            let tx_type = transform::classify_transaction_with_registry(&transaction.program_ids, program_registry);

            // Get detailed label
            let details = transform::analyze_transaction_with_registry(
                &transaction.program_ids,
                program_registry,
                Some(&transaction.raw_json),
            );

            // Extract signer (fee payer) from raw_json if available
            let signer = transaction
                .raw_json
                .get("transaction")
                .and_then(|t| t.get("message"))
                .and_then(|m| m.get("accountKeys"))
                .and_then(|keys| keys.as_array())
                .and_then(|arr| arr.first())
                .and_then(|key| {
                    // Handle both formats: {"pubkey": "..."} and just "string"
                    if let Some(pubkey) = key.get("pubkey") {
                        pubkey.as_str()
                    } else {
                        key.as_str()
                    }
                })
                .map(|s| s.to_string());

            // Insert transaction
            sqlx::query!(
                r#"
                INSERT INTO transactions (
                    signature, 
                    block_slot, 
                    transaction_index, 
                    success, 
                    fee, 
                    transaction_type, 
                    transaction_label,
                    signer,
                    num_accounts,
                    raw_data
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (signature) 
                DO UPDATE SET
                    block_slot = EXCLUDED.block_slot,
                    transaction_index = EXCLUDED.transaction_index,
                    success = EXCLUDED.success,
                    fee = EXCLUDED.fee,
                    transaction_type = EXCLUDED.transaction_type,
                    transaction_label = EXCLUDED.transaction_label,
                    signer = EXCLUDED.signer,
                    num_accounts = EXCLUDED.num_accounts,
                    raw_data = EXCLUDED.raw_data,
                    processed_at = NOW()
                "#,
                transaction.signature,
                block.slot as i64,
                tx_index as i32,
                transaction.success,
                transaction.fee as i64,
                tx_type.as_str(),
                details.label,
                signer,
                transaction.num_accounts as i32,
                transaction.raw_json
            )
            .execute(&mut *tx)
            .await?;

            transactions_inserted += 1;
        }
    }

    // Commit the transaction
    tx.commit().await?;

    tracing::info!("Batch inserted {} blocks and {} transactions", blocks_inserted, transactions_inserted);
    Ok((blocks_inserted, transactions_inserted))
}
