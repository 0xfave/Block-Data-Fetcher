-- Create program_registry table
-- Reference table for known Solana programs

CREATE TABLE program_registry (
    program_id VARCHAR(44) PRIMARY KEY,
    program_name VARCHAR(100) NOT NULL,
    program_type VARCHAR(50), -- DEX, NFT, Token, System, etc.
    description TEXT,
    website VARCHAR(255),
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Pre-populate with known Solana programs
INSERT INTO program_registry (program_id, program_name, program_type, description) VALUES
    -- System and Core Programs
    ('11111111111111111111111111111111', 'System Program', 'System', 'Native Solana system program for account management and transfers'),
    ('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA', 'Token Program', 'Token', 'SPL Token program for fungible tokens'),
    ('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL', 'Associated Token Program', 'Token', 'Creates associated token accounts'),
    ('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb', 'Token-2022 Program', 'Token', 'SPL Token-2022 program with extensions'),
    
    -- DEX Programs
    ('JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4', 'Jupiter Aggregator v6', 'DEX', 'Jupiter DEX aggregator for best swap rates'),
    ('whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc', 'Orca Whirlpool', 'DEX', 'Orca concentrated liquidity pools'),
    ('9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP', 'Orca v2', 'DEX', 'Orca v2 liquidity pools'),
    ('675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8', 'Raydium AMM v4', 'DEX', 'Raydium automated market maker'),
    ('CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK', 'Raydium CLMM', 'DEX', 'Raydium concentrated liquidity market maker'),
    
    -- NFT Programs
    ('M2mx93ekt1fmXSVkTrUL9xVFHkmME8HTUi5Cyc5aF7K', 'Magic Eden v2', 'NFT', 'Magic Eden NFT marketplace'),
    ('CJsLwbP1iu5DuUikHEJnLfANgKy6stB2uFgvBBHoyxwz', 'Solanart', 'NFT', 'Solanart NFT marketplace'),
    ('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s', 'Metaplex Token Metadata', 'NFT', 'Metaplex token metadata program'),
    ('p1exdMJcjVao65QdewkaZRUnU6VPSXhus9n2GzWfh98', 'Metaplex Auction House', 'NFT', 'Metaplex auction house for NFT sales'),
    
    -- Lending Programs
    ('So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo', 'Solend', 'Lending', 'Solend lending protocol'),
    ('MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD', 'Marginfi', 'Lending', 'Marginfi lending protocol'),
    
    -- Staking Programs
    ('CRaTQLhLmP93f5YeEdoVvfDwHp2FyokBME6MpF9pxLx9', 'Marinade Finance', 'Staking', 'Marinade liquid staking'),
    ('J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn', 'Jito Stake Pool', 'Staking', 'Jito MEV liquid staking'),
    
    -- Other Programs
    ('MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr', 'Memo Program', 'Utility', 'On-chain memo/message program'),
    ('ComputeBudget111111111111111111111111111111', 'Compute Budget Program', 'System', 'Adjust compute unit price and limits');

-- Add comments
COMMENT ON TABLE program_registry IS 'Registry of known Solana programs';
COMMENT ON COLUMN program_registry.program_id IS 'Base58-encoded program public key';
COMMENT ON COLUMN program_registry.program_type IS 'Category: DEX, NFT, Token, System, Lending, etc.';
