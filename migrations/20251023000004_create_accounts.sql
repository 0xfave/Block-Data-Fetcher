-- Create accounts table (optional - for tracking account activity)
-- Track account activity across transactions

CREATE TABLE accounts (
    -- Primary identifier
    address VARCHAR(44) PRIMARY KEY,
    
    -- Activity tracking
    first_seen_slot BIGINT REFERENCES blocks(slot),
    last_seen_slot BIGINT REFERENCES blocks(slot),
    first_seen_at TIMESTAMP WITH TIME ZONE,
    last_seen_at TIMESTAMP WITH TIME ZONE,
    
    -- Statistics
    transaction_count INTEGER DEFAULT 0,
    as_signer_count INTEGER DEFAULT 0,
    as_writable_count INTEGER DEFAULT 0,
    
    -- Account type classification
    account_type VARCHAR(50), -- wallet, program, token_account, etc.
    
    -- Metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_accounts_first_seen_slot ON accounts(first_seen_slot);
CREATE INDEX idx_accounts_last_seen_slot ON accounts(last_seen_slot);
CREATE INDEX idx_accounts_transaction_count ON accounts(transaction_count);
CREATE INDEX idx_accounts_account_type ON accounts(account_type);

-- Add comments
COMMENT ON TABLE accounts IS 'Tracks account activity over time';
COMMENT ON COLUMN accounts.address IS 'Base58-encoded account public key';
COMMENT ON COLUMN accounts.transaction_count IS 'Total number of transactions involving this account';
