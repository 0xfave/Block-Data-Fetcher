/// Pipeline Module
///
/// Orchestrates the complete ETL pipeline: Extract → Transform → Load
/// with proper error handling, retry logic, and statistics tracking.
use crate::db::Database;
use crate::{
    etl::{extract::ExtractedBlock, transform::ProgramRegistry},
    rpc::SolanaRpcClient,
};
use anyhow::Result;
use std::time::{Duration, Instant};

/// Pipeline execution statistics
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    pub blocks_attempted: usize,
    pub blocks_succeeded: usize,
    pub blocks_failed: usize,
    pub transactions_processed: usize,
    pub transactions_inserted: usize,
    pub elapsed_time: Duration,
    pub errors: Vec<PipelineError>,
}

impl PipelineStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn success_rate(&self) -> f64 {
        if self.blocks_attempted == 0 {
            0.0
        } else {
            (self.blocks_succeeded as f64 / self.blocks_attempted as f64) * 100.0
        }
    }

    pub fn blocks_per_second(&self) -> f64 {
        let secs = self.elapsed_time.as_secs_f64();
        if secs == 0.0 {
            0.0
        } else {
            self.blocks_succeeded as f64 / secs
        }
    }

    pub fn transactions_per_second(&self) -> f64 {
        let secs = self.elapsed_time.as_secs_f64();
        if secs == 0.0 {
            0.0
        } else {
            self.transactions_inserted as f64 / secs
        }
    }
}

/// Pipeline error with context
#[derive(Debug, Clone)]
pub struct PipelineError {
    pub stage: PipelineStage,
    pub slot: Option<u64>,
    pub message: String,
    #[allow(dead_code)]
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum PipelineStage {
    Extract,
    Transform,
    Load,
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineStage::Extract => write!(f, "Extract"),
            PipelineStage::Transform => write!(f, "Transform"),
            PipelineStage::Load => write!(f, "Load"),
        }
    }
}

/// Configuration for pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub start_slot: u64,
    pub end_slot: u64,
    pub max_retries: usize,
    pub retry_delay: Duration,
    pub batch_size: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self { start_slot: 0, end_slot: 0, max_retries: 3, retry_delay: Duration::from_secs(2), batch_size: 10 }
    }
}

/// Main ETL Pipeline
pub struct Pipeline {
    rpc_client: SolanaRpcClient,
    database: Database,
    program_registry: ProgramRegistry,
    config: PipelineConfig,
}

impl Pipeline {
    /// Create a new pipeline instance
    pub fn new(
        rpc_client: SolanaRpcClient,
        database: Database,
        program_registry: ProgramRegistry,
        config: PipelineConfig,
    ) -> Self {
        Self { rpc_client, database, program_registry, config }
    }

    /// Run the complete pipeline for the configured slot range
    pub async fn run(&self) -> Result<PipelineStats> {
        let start_time = Instant::now();
        let mut stats = PipelineStats::new();

        tracing::info!("Starting pipeline for slots {} to {}", self.config.start_slot, self.config.end_slot);

        println!("\n🚀 Starting ETL Pipeline...");
        println!("   📍 Slot range: {} to {}", self.config.start_slot, self.config.end_slot);
        println!("   🔄 Max retries: {}", self.config.max_retries);
        println!("   📦 Batch size: {}", self.config.batch_size);

        // Process blocks in batches
        let total_slots = self.config.end_slot - self.config.start_slot + 1;
        let mut current_slot = self.config.start_slot;

        while current_slot <= self.config.end_slot {
            let batch_end = std::cmp::min(current_slot + self.config.batch_size as u64 - 1, self.config.end_slot);

            match self.process_batch(current_slot, batch_end, &mut stats).await {
                Ok(_) => {
                    let progress = ((stats.blocks_succeeded as f64 / total_slots as f64) * 100.0) as usize;
                    println!(
                        "   📊 Progress: {}/{} blocks ({}%) | ✅ {} succeeded | ❌ {} failed",
                        stats.blocks_attempted, total_slots, progress, stats.blocks_succeeded, stats.blocks_failed
                    );
                }
                Err(e) => {
                    tracing::error!("Batch processing failed for slots {}-{}: {}", current_slot, batch_end, e);
                    stats.errors.push(PipelineError {
                        stage: PipelineStage::Extract,
                        slot: Some(current_slot),
                        message: format!("Batch failed: {}", e),
                        retryable: true,
                    });
                }
            }

            current_slot = batch_end + 1;
        }

        stats.elapsed_time = start_time.elapsed();

        println!("\n✅ Pipeline complete!");
        self.print_final_stats(&stats);

        Ok(stats)
    }

    /// Process a batch of blocks
    async fn process_batch(&self, start_slot: u64, end_slot: u64, stats: &mut PipelineStats) -> Result<()> {
        // Extract: Fetch blocks from RPC
        let blocks = self.extract_blocks(start_slot, end_slot, stats).await?;

        if blocks.is_empty() {
            return Ok(());
        }

        // Transform: Classification happens during load (already implemented)
        // No explicit transform step needed as it's integrated

        // Load: Batch insert into database
        self.load_blocks(&blocks, stats).await?;

        Ok(())
    }

    /// Extract blocks with retry logic
    async fn extract_blocks(
        &self,
        start_slot: u64,
        end_slot: u64,
        stats: &mut PipelineStats,
    ) -> Result<Vec<ExtractedBlock>> {
        let mut retry_count = 0;

        loop {
            stats.blocks_attempted += (end_slot - start_slot + 1) as usize;

            match crate::etl::extract::extract_block_range(
                &self.rpc_client,
                start_slot,
                end_slot,
                100,
                Some(&self.program_registry),
            )
            .await
            {
                Ok((blocks, _extract_stats)) => {
                    stats.blocks_succeeded += blocks.len();

                    // Count transactions
                    let tx_count: usize = blocks.iter().map(|b| b.transactions.len()).sum();
                    stats.transactions_processed += tx_count;

                    return Ok(blocks);
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count >= self.config.max_retries {
                        stats.blocks_failed += (end_slot - start_slot + 1) as usize;
                        stats.errors.push(PipelineError {
                            stage: PipelineStage::Extract,
                            slot: Some(start_slot),
                            message: format!("Max retries exceeded: {}", e),
                            retryable: false,
                        });
                        return Err(e.context(format!(
                            "Failed to extract blocks {}-{} after {} retries",
                            start_slot, end_slot, retry_count
                        )));
                    }

                    tracing::warn!(
                        "Extract failed for slots {}-{}, retrying ({}/{}): {}",
                        start_slot,
                        end_slot,
                        retry_count,
                        self.config.max_retries,
                        e
                    );

                    tokio::time::sleep(self.config.retry_delay * retry_count as u32).await;
                }
            }
        }
    }

    /// Load blocks into database with retry logic
    async fn load_blocks(&self, blocks: &[ExtractedBlock], stats: &mut PipelineStats) -> Result<()> {
        let mut retry_count = 0;

        loop {
            match crate::etl::load::batch_insert_blocks_with_transactions(
                self.database.pool(),
                blocks,
                &self.program_registry,
            )
            .await
            {
                Ok((blocks_inserted, txs_inserted)) => {
                    stats.transactions_inserted += txs_inserted;
                    tracing::info!("Loaded {} blocks with {} transactions", blocks_inserted, txs_inserted);
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;

                    if retry_count >= self.config.max_retries {
                        stats.errors.push(PipelineError {
                            stage: PipelineStage::Load,
                            slot: blocks.first().map(|b| b.slot),
                            message: format!("Max retries exceeded: {}", e),
                            retryable: false,
                        });
                        return Err(e.context(format!("Failed to load blocks after {} retries", retry_count)));
                    }

                    tracing::warn!("Load failed, retrying ({}/{}): {}", retry_count, self.config.max_retries, e);

                    tokio::time::sleep(self.config.retry_delay * retry_count as u32).await;
                }
            }
        }
    }

    /// Print final statistics
    fn print_final_stats(&self, stats: &PipelineStats) {
        println!("\n📊 Pipeline Statistics:");
        println!("   ⏱️  Total time: {:.2}s", stats.elapsed_time.as_secs_f64());
        println!(
            "   📦 Blocks: {} attempted, {} succeeded, {} failed",
            stats.blocks_attempted, stats.blocks_succeeded, stats.blocks_failed
        );
        println!("   ✅ Success rate: {:.1}%", stats.success_rate());
        println!("   📝 Transactions processed: {}", stats.transactions_processed);
        println!("   💾 Transactions inserted: {}", stats.transactions_inserted);
        println!("   ⚡ Speed: {:.2} blocks/sec", stats.blocks_per_second());
        println!("   ⚡ Throughput: {:.0} txs/sec", stats.transactions_per_second());

        if !stats.errors.is_empty() {
            println!("\n❌ Errors encountered: {}", stats.errors.len());
            for (i, error) in stats.errors.iter().take(5).enumerate() {
                println!("   {}. [{}] Slot {:?}: {}", i + 1, error.stage, error.slot, error.message);
            }
            if stats.errors.len() > 5 {
                println!("   ... and {} more errors", stats.errors.len() - 5);
            }
        }
    }
}
