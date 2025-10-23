/// Data Models Module
///
/// This module defines the core data structures used throughout the application.
/// These models represent Solana blockchain data (blocks, transactions, instructions)
/// and their database representations.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a Solana block
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub slot: i64,
    pub blockhash: String,
    pub parent_slot: Option<i64>,
    pub block_time: Option<DateTime<Utc>>,
    pub block_height: Option<i64>,
}

/// Represents a Solana transaction with classification
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub signature: String,
    pub block_slot: i64,
    pub transaction_index: i32,
    pub success: bool,
    pub fee: i64,
    pub transaction_type: Option<String>,
    pub raw_data: serde_json::Value,
}

/// Represents a transaction instruction
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub transaction_signature: String,
    pub instruction_index: i32,
    pub program_id: String,
    pub program_name: Option<String>,
    pub instruction_type: Option<String>,
    pub accounts: Vec<String>,
}

/// Types of transactions we can classify
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    SolTransfer,
    SplTokenTransfer,
    NftMint,
    NftTransfer,
    DexSwap,
    ProgramInteraction,
    Unknown,
}

impl TransactionType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::SolTransfer => "SOL Transfer",
            Self::SplTokenTransfer => "SPL Token Transfer",
            Self::NftMint => "NFT Mint",
            Self::NftTransfer => "NFT Transfer",
            Self::DexSwap => "DEX Swap",
            Self::ProgramInteraction => "Program Interaction",
            Self::Unknown => "Unknown",
        }
    }
}

/// Connection status information displayed at startup
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub endpoint: String,
    pub blockhash: String,
    pub slot: u64,
    pub timestamp: DateTime<Utc>,
}
