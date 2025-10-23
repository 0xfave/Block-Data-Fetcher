-- Create blocks table
-- Stores high-level block information from Solana blockchain

CREATE TABLE blocks (
    -- Primary identifier
    slot BIGINT PRIMARY KEY,
    
    -- Block metadata
    blockhash VARCHAR(88) NOT NULL UNIQUE,
    parent_slot BIGINT,
    block_time TIMESTAMP WITH TIME ZONE,
    block_height BIGINT,
    
    -- Processing metadata
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Foreign key to parent block (self-reference)
    FOREIGN KEY (parent_slot) REFERENCES blocks(slot) ON DELETE SET NULL
);

-- Indexes for common queries
CREATE INDEX idx_blocks_block_time ON blocks(block_time);
CREATE INDEX idx_blocks_block_height ON blocks(block_height);
CREATE INDEX idx_blocks_parent_slot ON blocks(parent_slot);

-- Add comment
COMMENT ON TABLE blocks IS 'Stores Solana block metadata';
COMMENT ON COLUMN blocks.slot IS 'Unique slot number (primary identifier)';
COMMENT ON COLUMN blocks.blockhash IS 'Base58-encoded blockhash';
COMMENT ON COLUMN blocks.parent_slot IS 'Reference to parent block slot';
