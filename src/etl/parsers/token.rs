/// SPL Token Program Instruction Parser
///
/// Parses instructions from the SPL Token Program and Token-2022 Program.
/// Handles token transfers, mints, burns, and other token operations.
///
/// Parse SPL Token Program instructions for token transfers
///
/// Extracts transfer details from JsonParsed instruction data:
/// - Amount (in token base units)
/// - Mint address (token contract address)
/// - Source token account
/// - Destination token account
///
/// Supports both `transfer` and `transferChecked` instruction types.
/// Returns None if the instruction is not a transfer.
pub fn parse_token_transfer(
    instruction: &serde_json::Value,
    _account_keys: &[String],
) -> Option<(u64, String, String, String)> {
    // Token Program transfer instruction structure with JsonParsed encoding:
    // - parsed/info/amount contains the transfer amount (as string)
    // - parsed/info/source is the source token account
    // - parsed/info/destination is the destination token account
    // - parsed/info/authority is the signer
    // - parsed/type should be "transfer" or "transferChecked"

    if let Some(parsed) = instruction.get("parsed") {
        // Check if this is a transfer instruction
        if let Some(inst_type) = parsed.get("type").and_then(|t| t.as_str()) {
            if inst_type != "transfer" && inst_type != "transferChecked" {
                return None; // Not a transfer
            }
        }

        if let Some(info) = parsed.get("info") {
            // Get the amount (as string in JSON, parse to u64)
            let amount = if let Some(amount_str) = info.get("amount").and_then(|v| v.as_str()) {
                amount_str.parse::<u64>().ok()?
            } else if let Some(amount_str) =
                info.get("tokenAmount").and_then(|ta| ta.get("amount")).and_then(|v| v.as_str())
            {
                amount_str.parse::<u64>().ok()?
            } else {
                info.get("amount").and_then(|v| v.as_u64())?
            };

            // Get source and destination from parsed info
            let source = info.get("source").and_then(|v| v.as_str())?.to_string();
            let destination = info.get("destination").and_then(|v| v.as_str())?.to_string();

            // Try to get the mint address (token address)
            let mint = info.get("mint").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();

            return Some((amount, mint, source, destination));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token_transfer() {
        let instruction = serde_json::json!({
            "parsed": {
                "type": "transfer",
                "info": {
                    "amount": "1000000",
                    "source": "TokenAccount1111111111111111111111111",
                    "destination": "TokenAccount2222222222222222222222222",
                    "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                    "authority": "Authority1111111111111111111111111111"
                }
            },
            "program": "spl-token",
            "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        });

        let result = parse_token_transfer(&instruction, &[]);
        assert!(result.is_some());

        let (amount, mint, source, dest) = result.unwrap();
        assert_eq!(amount, 1000000);
        assert_eq!(mint, "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        assert_eq!(source, "TokenAccount1111111111111111111111111");
        assert_eq!(dest, "TokenAccount2222222222222222222222222");
    }

    #[test]
    fn test_parse_token_transfer_checked() {
        let instruction = serde_json::json!({
            "parsed": {
                "type": "transferChecked",
                "info": {
                    "tokenAmount": {
                        "amount": "5000",
                        "decimals": 6
                    },
                    "source": "Source111111111111111111111111111111",
                    "destination": "Dest111111111111111111111111111111111",
                    "mint": "MintAddress11111111111111111111111111"
                }
            },
            "program": "spl-token"
        });

        let result = parse_token_transfer(&instruction, &[]);
        assert!(result.is_some());

        let (amount, mint, _, _) = result.unwrap();
        assert_eq!(amount, 5000);
        assert_eq!(mint, "MintAddress11111111111111111111111111");
    }

    #[test]
    fn test_parse_token_transfer_non_transfer() {
        let instruction = serde_json::json!({
            "parsed": {
                "type": "mintTo",
                "info": {
                    "amount": "1000",
                    "mint": "MintAddress11111111111111111111111111"
                }
            },
            "program": "spl-token"
        });

        let result = parse_token_transfer(&instruction, &[]);
        assert!(result.is_none());
    }
}
