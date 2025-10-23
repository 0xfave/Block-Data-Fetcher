-- Create transactions table
-- Stores individual transaction data with classification

CREATE TABLE transactions (
    -- Primary identifier
    id BIGSERIAL PRIMARY KEY,
    signature VARCHAR(88) NOT NULL UNIQUE,
    
    -- Block relationship
    block_slot BIGINT NOT NULL REFERENCES blocks(slot) ON DELETE CASCADE,
    transaction_index INTEGER NOT NULL,
    
    -- Transaction metadata
    success BOOLEAN NOT NULL,
    fee BIGINT NOT NULL,
    
    -- Classification (our ETL logic)
    transaction_type VARCHAR(50),
    transaction_label VARCHAR(100), -- Human-readable label
    
    -- Accounts involved
    signer VARCHAR(44), -- Primary signer/fee payer
    num_accounts INTEGER,
    
    -- Raw data for reference
    raw_data JSONB,
    
    -- Processing metadata
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Ensure uniqueness per block
    UNIQUE(block_slot, transaction_index)
);

-- Indexes for common queries
CREATE INDEX idx_transactions_block_slot ON transactions(block_slot);
CREATE INDEX idx_transactions_signature ON transactions(signature);
CREATE INDEX idx_transactions_type ON transactions(transaction_type);
CREATE INDEX idx_transactions_signer ON transactions(signer);
CREATE INDEX idx_transactions_success ON transactions(success);
CREATE INDEX idx_transactions_raw_data ON transactions USING GIN(raw_data);

-- Add comments
COMMENT ON TABLE transactions IS 'Stores Solana transactions with classification';
COMMENT ON COLUMN transactions.signature IS 'Base58-encoded transaction signature (unique identifier)';
COMMENT ON COLUMN transactions.transaction_type IS 'Classified transaction type (e.g., SOL Transfer, Token Transfer)';
COMMENT ON COLUMN transactions.raw_data IS 'Full transaction data in JSON format';
