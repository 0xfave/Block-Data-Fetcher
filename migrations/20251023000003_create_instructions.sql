-- Create instructions table
-- Stores detailed instruction breakdown for each transaction

CREATE TABLE instructions (
    -- Primary identifier
    id BIGSERIAL PRIMARY KEY,
    
    -- Transaction relationship
    transaction_signature VARCHAR(88) NOT NULL REFERENCES transactions(signature) ON DELETE CASCADE,
    instruction_index INTEGER NOT NULL,
    
    -- Instruction details
    program_id VARCHAR(44) NOT NULL,
    program_name VARCHAR(100), -- Human-readable program name
    instruction_type VARCHAR(100), -- Specific instruction (Transfer, Swap, etc.)
    
    -- Accounts involved in this instruction
    accounts TEXT[], -- Array of account addresses
    num_accounts INTEGER,
    
    -- Instruction data
    data_hex TEXT, -- Raw instruction data in hex
    data_decoded JSONB, -- Decoded instruction data when possible
    
    -- Processing metadata
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(transaction_signature, instruction_index)
);

-- Indexes for common queries
CREATE INDEX idx_instructions_transaction ON instructions(transaction_signature);
CREATE INDEX idx_instructions_program_id ON instructions(program_id);
CREATE INDEX idx_instructions_program_name ON instructions(program_name);
CREATE INDEX idx_instructions_instruction_type ON instructions(instruction_type);
CREATE INDEX idx_instructions_accounts ON instructions USING GIN(accounts);
CREATE INDEX idx_instructions_data_decoded ON instructions USING GIN(data_decoded);

-- Add comments
COMMENT ON TABLE instructions IS 'Stores detailed instruction breakdown for each transaction';
COMMENT ON COLUMN instructions.program_id IS 'Base58-encoded program public key';
COMMENT ON COLUMN instructions.accounts IS 'Array of account addresses involved in this instruction';
COMMENT ON COLUMN instructions.data_decoded IS 'Decoded instruction data in JSON format';
