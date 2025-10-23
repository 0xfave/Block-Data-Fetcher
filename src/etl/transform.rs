/// Transform Module
///
/// Handles parsing and classification of transaction data.
use crate::models::TransactionType;
use anyhow::Result;
use std::collections::HashMap;

// Import parsers
use super::parsers::{parse_system_transfer, parse_token_transfer};

/// Program registry for transaction classification
#[derive(Debug, Clone)]
pub struct ProgramRegistry {
    /// Map of program_id -> (program_name, program_type)
    pub programs: HashMap<String, (String, String)>,
}

impl ProgramRegistry {
    /// Create a new program registry from database
    pub fn from_database(programs: Vec<crate::db::ProgramInfo>) -> Self {
        let mut registry = HashMap::new();

        for prog in programs {
            registry.insert(
                prog.program_id.clone(),
                (prog.program_name.clone(), prog.program_type.unwrap_or_else(|| "Unknown".to_string())),
            );
        }

        Self { programs: registry }
    }

    /// Get program name by ID
    pub fn get_program_name(&self, program_id: &str) -> Option<&str> {
        self.programs.get(program_id).map(|(name, _)| name.as_str())
    }

    /// Get program type by ID
    pub fn get_program_type(&self, program_id: &str) -> Option<&str> {
        self.programs.get(program_id).map(|(_, ptype)| ptype.as_str())
    }

    /// Check if a program is a DEX
    pub fn is_dex(&self, program_id: &str) -> bool {
        self.get_program_type(program_id).map(|t| t == "DEX").unwrap_or(false)
    }

    /// Check if a program is NFT-related
    pub fn is_nft(&self, program_id: &str) -> bool {
        self.get_program_type(program_id).map(|t| t == "NFT").unwrap_or(false)
    }

    /// Check if a program is Token-related
    pub fn is_token(&self, program_id: &str) -> bool {
        self.get_program_type(program_id).map(|t| t == "Token").unwrap_or(false)
    }

    /// Check if a program is System
    pub fn is_system(&self, program_id: &str) -> bool {
        self.get_program_type(program_id).map(|t| t == "System").unwrap_or(false)
    }
}

// Known Solana program IDs (fallback if database is not available)
const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
#[allow(dead_code)]
const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

// DEX programs
const JUPITER_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const RAYDIUM_CLMM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

// NFT programs
const METAPLEX_TOKEN_METADATA: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
const MAGIC_EDEN_V2: &str = "M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K";

/// Classify a transaction using the program registry
pub fn classify_transaction_with_registry(program_ids: &[String], registry: &ProgramRegistry) -> TransactionType {
    // Check for DEX interactions (swaps) using registry
    if program_ids.iter().any(|id| registry.is_dex(id)) {
        return TransactionType::DexSwap;
    }

    // Check for NFT operations using registry
    if program_ids.iter().any(|id| registry.is_nft(id)) {
        return TransactionType::NftMint; // Generic NFT operation
    }

    // Check for SPL Token transfers using registry
    if program_ids.iter().any(|id| registry.is_token(id)) {
        // System program + Token program often indicates account creation or wrapped SOL
        if program_ids.iter().any(|id| registry.is_system(id)) {
            return TransactionType::SplTokenTransfer;
        }
        return TransactionType::SplTokenTransfer;
    }

    // Check for pure SOL transfers (System Program only)
    if program_ids.len() == 1 && registry.is_system(&program_ids[0]) {
        return TransactionType::SolTransfer;
    }

    // If System Program is involved with other programs
    if program_ids.iter().any(|id| registry.is_system(id)) {
        return TransactionType::ProgramInteraction;
    }

    // Default to Unknown
    TransactionType::Unknown
}

/// Classify a transaction based on its program IDs (without registry - legacy)
pub fn classify_transaction(program_ids: &[String]) -> TransactionType {
    // Check for DEX interactions (swaps)
    if program_ids
        .iter()
        .any(|id| id == JUPITER_V6 || id == ORCA_WHIRLPOOL || id == RAYDIUM_AMM_V4 || id == RAYDIUM_CLMM)
    {
        return TransactionType::DexSwap;
    }

    // Check for NFT operations
    if program_ids.iter().any(|id| id == METAPLEX_TOKEN_METADATA || id == MAGIC_EDEN_V2) {
        // Distinguish between mint and transfer would require instruction parsing
        // For now, we'll just call it NFT-related
        return TransactionType::NftMint; // Generic NFT operation
    }

    // Check for SPL Token transfers
    if program_ids.iter().any(|id| id == TOKEN_PROGRAM || id == TOKEN_2022_PROGRAM) {
        // This could be a token transfer, but we'd need to parse instructions to be sure
        // System program + Token program often indicates a wrapped SOL or token operation
        if program_ids.contains(&SYSTEM_PROGRAM.to_string()) {
            // Could be SOL wrapping or account creation for token
            return TransactionType::SplTokenTransfer;
        }
        return TransactionType::SplTokenTransfer;
    }

    // Check for pure SOL transfers (System Program only or primarily)
    if program_ids.len() == 1 && program_ids[0] == SYSTEM_PROGRAM {
        return TransactionType::SolTransfer;
    }

    // If System Program is involved but with other programs
    if program_ids.contains(&SYSTEM_PROGRAM.to_string()) {
        return TransactionType::ProgramInteraction;
    }

    // Default to Unknown
    TransactionType::Unknown
}

/// Get a human-readable label for a transaction type with program names using registry
#[allow(dead_code)]
pub fn label_transaction_with_registry(
    transaction_type: &TransactionType,
    program_ids: &[String],
    registry: &ProgramRegistry,
) -> String {
    let type_label = transaction_type.as_str();

    // Add program context if available
    let program_context = if !program_ids.is_empty() {
        let program_names: Vec<String> =
            program_ids.iter().filter_map(|id| registry.get_program_name(id).map(|s| s.to_string())).collect();

        if !program_names.is_empty() {
            format!(" ({})", program_names.join(", "))
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    format!("{}{}", type_label, program_context)
}

/// Get a human-readable label for a transaction type with program names (legacy)
#[allow(dead_code)]
pub fn label_transaction(transaction_type: &TransactionType, program_ids: &[String]) -> String {
    let type_label = transaction_type.as_str();

    // Add program context if available
    let program_context = if !program_ids.is_empty() {
        let program_names: Vec<String> = program_ids.iter().filter_map(|id| get_program_name(id)).collect();

        if !program_names.is_empty() {
            format!(" ({})", program_names.join(", "))
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    format!("{}{}", type_label, program_context)
}

/// Get a friendly name for a known program ID
#[allow(dead_code)]
fn get_program_name(program_id: &str) -> Option<String> {
    match program_id {
        SYSTEM_PROGRAM => Some("System".to_string()),
        TOKEN_PROGRAM => Some("Token".to_string()),
        TOKEN_2022_PROGRAM => Some("Token-2022".to_string()),
        ASSOCIATED_TOKEN_PROGRAM => Some("Associated Token".to_string()),
        JUPITER_V6 => Some("Jupiter".to_string()),
        ORCA_WHIRLPOOL => Some("Orca".to_string()),
        RAYDIUM_AMM_V4 => Some("Raydium AMM".to_string()),
        RAYDIUM_CLMM => Some("Raydium CLMM".to_string()),
        METAPLEX_TOKEN_METADATA => Some("Metaplex".to_string()),
        MAGIC_EDEN_V2 => Some("Magic Eden".to_string()),
        _ => None,
    }
}

/// Parse instruction data to extract details
#[allow(dead_code)]
pub fn parse_instruction_data(_data: &[u8]) -> Result<serde_json::Value> {
    // TODO: Implement specific instruction parsing based on program
    // This will decode instruction data for known programs
    Ok(serde_json::json!({}))
}

/// Extract account keys from transaction JSON
fn extract_account_keys(tx_json: &serde_json::Value) -> Vec<String> {
    let mut keys = Vec::new();

    if let Some(message) = tx_json.get("message") {
        if let Some(account_keys) = message.get("accountKeys").and_then(|a| a.as_array()) {
            for key in account_keys {
                if let Some(pubkey) = key.get("pubkey").and_then(|p| p.as_str()) {
                    keys.push(pubkey.to_string());
                } else if let Some(pubkey_str) = key.as_str() {
                    keys.push(pubkey_str.to_string());
                }
            }
        }
    }

    keys
}

/// Extract detailed information from a transaction for better classification
#[derive(Debug, Clone)]
pub struct TransactionDetails {
    #[allow(dead_code)]
    pub tx_type: TransactionType,
    pub label: String,
    #[allow(dead_code)]
    pub amount: Option<u64>,
    #[allow(dead_code)]
    pub token_address: Option<String>,
    #[allow(dead_code)]
    pub from_account: Option<String>,
    #[allow(dead_code)]
    pub to_account: Option<String>,
    #[allow(dead_code)]
    pub program_names: Vec<String>,
}

/// Analyze transaction with detailed extraction
pub fn analyze_transaction_with_registry(
    program_ids: &[String],
    registry: &ProgramRegistry,
    tx_json: Option<&serde_json::Value>,
) -> TransactionDetails {
    let tx_type = classify_transaction_with_registry(program_ids, registry);

    // Collect program names
    let program_names: Vec<String> =
        program_ids.iter().filter_map(|id| registry.get_program_name(id).map(|s| s.to_string())).collect();

    // Create base label
    let label = tx_type.as_str().to_string();
    let full_label =
        if !program_names.is_empty() { format!("{} ({})", label, program_names.join(", ")) } else { label };

    // Try to extract detailed information if we have the transaction JSON
    let mut amount = None;
    let mut token_address = None;
    let mut from_account = None;
    let mut to_account = None;

    if let Some(json) = tx_json {
        // Debug: Print transaction structure for SOL transfers
        if std::env::var("DEBUG_TX").is_ok() && tx_type == TransactionType::SolTransfer {
            eprintln!("\n=== SOL Transfer Transaction ===");
            if let Some(message) = json.get("message") {
                eprintln!("Message keys: {:?}", message.as_object().map(|m| m.keys().collect::<Vec<_>>()));
                if let Some(instructions) = message.get("instructions") {
                    eprintln!("Instructions: {}", serde_json::to_string_pretty(instructions).unwrap_or_default());
                }
            }
        }

        // Extract account keys first
        let account_keys = extract_account_keys(json);

        // Look through instructions to find transfer details
        if let Some(message) = json.get("message") {
            if let Some(instructions) = message.get("instructions").and_then(|i| i.as_array()) {
                for instruction in instructions {
                    // Check for System Program transfers (SOL)
                    if let Some(program) = instruction.get("program").and_then(|p| p.as_str()) {
                        if registry.is_system(program) || program == "system" {
                            if let Some((amt, from, to)) = parse_system_transfer(instruction, &account_keys) {
                                amount = Some(amt);
                                from_account = Some(from);
                                to_account = Some(to);
                                break;
                            }
                        } else if registry.is_token(program) || program == "spl-token" {
                            if let Some((amt, mint, from, to)) = parse_token_transfer(instruction, &account_keys) {
                                amount = Some(amt);
                                token_address = Some(mint);
                                from_account = Some(from);
                                to_account = Some(to);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    TransactionDetails { tx_type, label: full_label, amount, token_address, from_account, to_account, program_names }
}

/// Get statistics about transaction types in a batch
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct TransactionTypeStats {
    pub sol_transfers: usize,
    pub token_transfers: usize,
    pub dex_swaps: usize,
    pub nft_operations: usize,
    pub program_interactions: usize,
    pub unknown: usize,
    pub total: usize,
}

impl TransactionTypeStats {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn add(&mut self, tx_type: &TransactionType) {
        self.total += 1;
        match tx_type {
            TransactionType::SolTransfer => self.sol_transfers += 1,
            TransactionType::SplTokenTransfer => self.token_transfers += 1,
            TransactionType::DexSwap => self.dex_swaps += 1,
            TransactionType::NftMint | TransactionType::NftTransfer => self.nft_operations += 1,
            TransactionType::ProgramInteraction => self.program_interactions += 1,
            TransactionType::Unknown => self.unknown += 1,
        }
    }

    #[allow(dead_code)]
    pub fn percentage(&self, count: usize) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (count as f64 / self.total as f64) * 100.0
        }
    }
}
