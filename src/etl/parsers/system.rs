/// System Program Instruction Parser
///
/// Parses instructions from the Solana System Program (11111111111111111111111111111111).
/// Handles transfer, createAccount, and other system-level operations.
///
/// Parse System Program instructions for SOL transfers
///
/// Extracts transfer details from JsonParsed instruction data:
/// - Amount in lamports
/// - Source account (from)
/// - Destination account (to)
///
/// Returns None if the instruction is not a transfer (e.g., advanceNonce, createAccount).
pub fn parse_system_transfer(
    instruction: &serde_json::Value,
    _account_keys: &[String],
) -> Option<(u64, String, String)> {
    // Debug: Print the instruction structure
    if std::env::var("DEBUG_TX").is_ok() {
        eprintln!("=== Parsing System Transfer ===");
        eprintln!("Instruction: {}", serde_json::to_string_pretty(instruction).unwrap_or_default());
    }

    // System Program transfer instruction structure with JsonParsed encoding:
    // - parsed/info/lamports contains the amount
    // - parsed/info/source is the "from" account
    // - parsed/info/destination is the "to" account
    // - parsed/type should be "transfer"

    if let Some(parsed) = instruction.get("parsed") {
        // Check if this is a transfer instruction
        if let Some(inst_type) = parsed.get("type").and_then(|t| t.as_str()) {
            if inst_type != "transfer" {
                return None; // Not a transfer, maybe advanceNonce or other system instruction
            }
        }

        if let Some(info) = parsed.get("info") {
            let amount = info.get("lamports").and_then(|v| v.as_u64())?;
            let from_account = info.get("source").and_then(|v| v.as_str())?.to_string();
            let to_account = info.get("destination").and_then(|v| v.as_str())?.to_string();

            if std::env::var("DEBUG_TX").is_ok() {
                eprintln!("✅ Parsed: amount={}, from={}, to={}", amount, from_account, to_account);
            }

            return Some((amount, from_account, to_account));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_system_transfer() {
        let instruction = serde_json::json!({
            "parsed": {
                "type": "transfer",
                "info": {
                    "lamports": 1000,
                    "source": "FY27ZyvXPv7vpGJkE788JHEXo",
                    "destination": "HFqU5x63Z2bU7gRe"
                }
            },
            "program": "system",
            "programId": "11111111111111111111111111111111"
        });

        let result = parse_system_transfer(&instruction, &[]);
        assert!(result.is_some());

        let (amount, from, to) = result.unwrap();
        assert_eq!(amount, 1000);
        assert_eq!(from, "FY27ZyvXPv7vpGJkE788JHEXo");
        assert_eq!(to, "HFqU5x63Z2bU7gRe");
    }

    #[test]
    fn test_parse_system_transfer_non_transfer() {
        let instruction = serde_json::json!({
            "parsed": {
                "type": "advanceNonce",
                "info": {
                    "nonceAccount": "NEzguywY1gfVMbi16AprMLKyrprsLQHXKTP5eGbddEf"
                }
            },
            "program": "system"
        });

        let result = parse_system_transfer(&instruction, &[]);
        assert!(result.is_none());
    }
}
