/// CLI Module
///
/// Command-line interface configuration using clap.
use clap::Parser;

/// Solana Block Data Fetcher - ETL Pipeline
///
/// Extract, transform, and load Solana blockchain data into PostgreSQL
#[derive(Parser, Debug)]
#[command(name = "block-data-fetcher")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Starting slot number (defaults to latest - 30)
    #[arg(short = 's', long, value_name = "SLOT")]
    pub start_slot: Option<u64>,

    /// Ending slot number (defaults to latest - 20)
    #[arg(short = 'e', long, value_name = "SLOT")]
    pub end_slot: Option<u64>,

    /// Number of blocks to fetch (alternative to specifying end_slot)
    #[arg(short = 'n', long, value_name = "COUNT", conflicts_with = "end_slot")]
    pub num_blocks: Option<u64>,

    /// RPC endpoint URL (overrides HELIUS_RPC_URL env var)
    #[arg(short = 'r', long, value_name = "URL")]
    pub rpc_url: Option<String>,

    /// Database connection URL (overrides DATABASE_URL env var)
    #[arg(short = 'd', long, value_name = "URL")]
    pub database_url: Option<String>,

    /// Batch size for processing blocks
    #[arg(short = 'b', long, value_name = "SIZE", default_value = "10")]
    pub batch_size: usize,

    /// Maximum number of retry attempts
    #[arg(long, value_name = "COUNT", default_value = "3")]
    pub max_retries: usize,

    /// Retry delay in seconds
    #[arg(long, value_name = "SECONDS", default_value = "2")]
    pub retry_delay: u64,

    /// Fetch blocks continuously (keep processing latest blocks)
    #[arg(short = 'c', long)]
    pub continuous: bool,

    /// Interval between continuous fetches in seconds
    #[arg(long, value_name = "SECONDS", default_value = "10")]
    pub interval: u64,
}

impl Cli {
    /// Validate CLI arguments
    pub fn validate(&self) -> anyhow::Result<()> {
        if let (Some(start), Some(end)) = (self.start_slot, self.end_slot) {
            if start > end {
                anyhow::bail!("Start slot ({}) must be less than or equal to end slot ({})", start, end);
            }
        }

        if self.batch_size == 0 {
            anyhow::bail!("Batch size must be greater than 0");
        }

        if self.max_retries == 0 {
            anyhow::bail!("Max retries must be greater than 0");
        }

        Ok(())
    }

    /// Calculate end slot based on start slot and num_blocks
    pub fn calculate_end_slot(&self, start_slot: u64) -> u64 {
        if let Some(num) = self.num_blocks {
            start_slot + num - 1
        } else if let Some(end) = self.end_slot {
            end
        } else {
            start_slot + 9 // Default to 10 blocks
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_end_slot() {
        let cli = Cli {
            start_slot: Some(1000),
            end_slot: None,
            num_blocks: Some(5),
            rpc_url: None,
            database_url: None,
            batch_size: 10,
            max_retries: 3,
            retry_delay: 2,
            continuous: false,
            interval: 10,
        };

        assert_eq!(cli.calculate_end_slot(1000), 1004);
    }

    #[test]
    fn test_validation() {
        let cli = Cli {
            start_slot: Some(1000),
            end_slot: Some(900),
            num_blocks: None,
            rpc_url: None,
            database_url: None,
            batch_size: 10,
            max_retries: 3,
            retry_delay: 2,
            continuous: false,
            interval: 10,
        };

        assert!(cli.validate().is_err());
    }
}
