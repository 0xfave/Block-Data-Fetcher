# Solana Block Data Fetcher 🚀

A robust ETL (Extract, Transform, Load) pipeline for fetching Solana blockchain data, classifying transactions, and storing them in PostgreSQL. Built with Rust for performance and reliability.

## 🌟 Features

- **Complete ETL Pipeline**: Extract blocks from Solana RPC → Transform/classify transactions → Load into PostgreSQL
- **Transaction Classification**: Automatically identifies SOL transfers, SPL token transfers, DEX swaps, NFT operations, and program interactions
- **Robust Error Handling**: Retry logic with exponential backoff for network failures
- **Batch Processing**: Efficient batch insertion with database transactions for atomicity
- **Configurable CLI**: Full command-line interface for all parameters
- **Real-time Statistics**: Progress tracking, throughput metrics, and success rates
- **Idempotent Operations**: UPSERT logic allows re-processing blocks without duplicates

## 📋 Prerequisites

- Rust 1.70+ (2021 edition)
- PostgreSQL 14+
- Helius RPC API key (or any Solana RPC endpoint)

## 🚀 Quick Start

### 1. Setup Database

```bash
# Create database and user
sudo -u postgres psql
CREATE DATABASE solana_block_data;
CREATE USER solana_user WITH PASSWORD 'solana_pass';
GRANT ALL PRIVILEGES ON DATABASE solana_block_data TO solana_user;
\c solana_block_data
GRANT ALL ON SCHEMA public TO solana_user;
\q
```

### 2. Configure Environment

Create a `.env` file:

```env
HELIUS_RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY
DATABASE_URL=postgresql://solana_user:solana_pass@localhost:5432/solana_block_data
RUST_LOG=info
```

### 3. Build and Run

```bash
# Build release version
cargo build --release

# Run with default settings (10 recent blocks)
./target/release/block-data-fetcher

# Run with custom parameters
./target/release/block-data-fetcher --num-blocks 50 --batch-size 25
```

## 📖 Usage

### Basic Commands

```bash
# Process 20 recent blocks
./block-data-fetcher --num-blocks 20

# Process specific slot range
./block-data-fetcher --start-slot 375000000 --end-slot 375000100

# Use custom RPC endpoint
./block-data-fetcher --rpc-url https://api.mainnet-beta.solana.com --num-blocks 10

# Continuous mode (keep processing latest blocks)
./block-data-fetcher --continuous --interval 30
```

### CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `-s, --start-slot <SLOT>` | Starting slot number | latest - 30 |
| `-e, --end-slot <SLOT>` | Ending slot number | latest - 20 |
| `-n, --num-blocks <COUNT>` | Number of blocks to fetch | - |
| `-r, --rpc-url <URL>` | RPC endpoint URL | From .env |
| `-d, --database-url <URL>` | Database connection URL | From .env |
| `-b, --batch-size <SIZE>` | Batch size for processing | 10 |
| `--max-retries <COUNT>` | Maximum retry attempts | 3 |
| `--retry-delay <SECONDS>` | Retry delay in seconds | 2 |
| `-c, --continuous` | Enable continuous mode | false |
| `--interval <SECONDS>` | Poll interval for continuous mode | 10 |
| `-h, --help` | Print help information | - |
| `-V, --version` | Print version information | - |

### Complete Examples

#### Backfill Historical Data
```bash
./block-data-fetcher \
  --start-slot 375000000 \
  --end-slot 375010000 \
  --batch-size 50 \
  --max-retries 5
```

#### Monitor Latest Blocks
```bash
./block-data-fetcher \
  --num-blocks 10 \
  --continuous \
  --interval 20
```

#### Custom RPC with Performance Tuning
```bash
./block-data-fetcher \
  --rpc-url https://api.mainnet-beta.solana.com \
  --num-blocks 100 \
  --batch-size 25 \
  --max-retries 5 \
  --retry-delay 3
```

#### Process Recent Blocks Only
```bash
./block-data-fetcher --num-blocks 5
```

#### Override Database Connection
```bash
./block-data-fetcher \
  --database-url postgresql://user:pass@localhost:5432/mydb \
  --num-blocks 20
```

### Environment Variables

The following environment variables are used when CLI arguments are not provided:

- `HELIUS_RPC_URL` - Solana RPC endpoint
- `DATABASE_URL` - PostgreSQL connection string
- `RUST_LOG` - Logging level (info, debug, trace)

These are loaded from a `.env` file if present.

## 🏗️ Architecture

### High-Level System Overview

```mermaid
graph TB
    subgraph "External Services"
        A[Solana RPC<br/>Helius API]
        B[(PostgreSQL<br/>Database)]
    end
    
    subgraph "CLI Layer"
        C[Command Line<br/>Interface]
    end
    
    subgraph "Pipeline Orchestration"
        D[Pipeline<br/>Coordinator]
        E[Statistics<br/>Tracker]
        F[Error Handler<br/>& Retry Logic]
    end
    
    subgraph "ETL Pipeline"
        G[Extract<br/>RPC Client]
        H[Transform<br/>Classifier]
        I[Load<br/>Database Writer]
    end
    
    C -->|Config| D
    D -->|Fetch Blocks| G
    G -->|Block Data| A
    A -->|Response| G
    G -->|Raw Data| H
    H -->|Classified| I
    I -->|Batch Insert| B
    D -->|Monitor| E
    D -->|Handle Errors| F
    F -->|Retry| G
    E -->|Report| C
    
    style A fill:#e1f5ff
    style B fill:#e1f5ff
    style D fill:#fff4e1
    style G fill:#e8f5e9
    style H fill:#e8f5e9
    style I fill:#e8f5e9
```

### Data Flow Sequence

```mermaid
sequenceDiagram
    participant CLI
    participant Pipeline
    participant RPC as RPC Client
    participant Helius as Helius API
    participant Classifier
    participant DB as PostgreSQL
    
    CLI->>Pipeline: Start (slot range)
    
    loop For each block
        Pipeline->>RPC: fetch_block(slot)
        RPC->>Helius: getBlock(slot, maxSupportedVersion)
        Helius-->>RPC: Block + Transactions
        RPC-->>Pipeline: RawBlockData
        
        Pipeline->>Classifier: classify_transactions(block)
        
        loop For each transaction
            Classifier->>Classifier: analyze_instructions()
            Classifier->>Classifier: determine_type()
        end
        
        Classifier-->>Pipeline: ClassifiedData
        
        Pipeline->>DB: batch_insert(classified_data)
        
        alt Insert Success
            DB-->>Pipeline: OK (rows inserted)
            Pipeline->>Pipeline: update_stats(success)
        else Insert Failure
            DB-->>Pipeline: Error
            Pipeline->>Pipeline: retry_logic()
            Pipeline->>DB: batch_insert(retry)
        end
    end
    
    Pipeline-->>CLI: Statistics Report
```

### Database Schema (ERD)

```mermaid
erDiagram
    BLOCKS ||--o{ TRANSACTIONS : contains
    TRANSACTIONS ||--o{ INSTRUCTIONS : has
    TRANSACTIONS ||--o{ ACCOUNTS : involves
    INSTRUCTIONS }o--|| PROGRAM_REGISTRY : references
    
    BLOCKS {
        bigint slot PK
        text blockhash UK
        bigint parent_slot
        timestamptz block_time
        bigint block_height
        jsonb raw_data
        timestamptz created_at
    }
    
    TRANSACTIONS {
        text signature PK
        bigint slot FK
        int index_in_block
        text fee_payer
        bigint fee
        text status
        text transaction_type
        jsonb raw_data
        timestamptz created_at
    }
    
    INSTRUCTIONS {
        bigserial id PK
        text transaction_signature FK
        int instruction_index
        text program_id
        text instruction_type
        jsonb data
        timestamptz created_at
    }
    
    ACCOUNTS {
        bigserial id PK
        text transaction_signature FK
        text pubkey
        bool is_signer
        bool is_writable
        bigint pre_balance
        bigint post_balance
        timestamptz created_at
    }
    
    PROGRAM_REGISTRY {
        text program_id PK
        text program_name
        text program_type
        text description
        timestamptz created_at
    }
```

### Transaction Classification Logic

```mermaid
flowchart TD
    A[Transaction] --> B{Has Instructions?}
    B -->|No| Z[Unknown Type]
    B -->|Yes| C[Analyze Instructions]
    
    C --> D{System Program<br/>Transfer?}
    D -->|Yes| E[SOL Transfer]
    
    D -->|No| F{Token Program<br/>Transfer?}
    F -->|Yes| G[Token Transfer]
    
    F -->|No| H{DEX Program?}
    H -->|Yes| I[DEX Swap]
    
    H -->|No| J{NFT Operation?}
    J -->|Yes| K[NFT Operation]
    
    J -->|No| L{Known Program?}
    L -->|Yes| M[Program Interaction]
    L -->|No| Z
    
    style E fill:#4caf50
    style G fill:#2196f3
    style I fill:#ff9800
    style K fill:#9c27b0
    style M fill:#00bcd4
    style Z fill:#9e9e9e
```

### Pipeline Stages

1. **Extract**: Fetch blocks from Solana RPC with rate limiting and error handling
2. **Transform**: Classify transactions based on program IDs and instruction data
3. **Load**: Batch insert into PostgreSQL with atomic transactions

### Database Schema

The system uses 6 tables with proper relationships:

- **`blocks`**: Block metadata (slot, blockhash, timestamp, parent relationships)
- **`transactions`**: Transaction details with classification labels, linked to blocks
- **`instructions`**: Individual instruction data, linked to transactions
- **`accounts`**: Account states (pre/post balances, signer status)
- **`program_registry`**: Known Solana programs for classification
- **Indexes**: Optimized for common queries on slots, signatures, and program IDs

### Transaction Classification

Automatically identifies:
- 💸 **SOL Transfers**: Native SOL transfers via System Program
- 🪙 **SPL Token Transfers**: Token transfers via Token Program
- 🔄 **DEX Swaps**: Interactions with Raydium, Orca, Jupiter, etc.
- 🖼️ **NFT Operations**: NFT mints and transfers
- ⚙️ **Program Interactions**: Other program invocations (Drift, Kamino, etc.)
- ❓ **Unknown**: Unclassified transactions

### Key Design Decisions

1. **Batch Processing**: Process blocks in configurable batches (default: 10) to balance memory vs. throughput
2. **UPSERT Strategy**: Use PostgreSQL UPSERT to enable idempotent re-processing without duplicates
3. **Classification at ETL Time**: Pre-compute transaction types for faster queries
4. **Exponential Backoff**: Handle transient failures (network, rate limits) with configurable retry logic
5. **JSONB Storage**: Structured schema + JSONB for raw data provides queryability + flexibility

## 📊 Performance

Typical performance metrics from real-world testing:
- **Throughput**: 200-300 transactions/second (including classification)
- **Batch Processing**: 10 blocks (~12,000 transactions) in ~25 seconds
- **Success Rate**: 99-100% with retry logic enabled
- **Database**: 28,597+ transactions processed across 26+ blocks
- **Idempotency**: UPSERT operations allow safe re-processing without duplicates

## 🛠️ Technology Stack

- **Language**: Rust 2021 edition
- **Async Runtime**: Tokio for concurrent operations
- **Database**: PostgreSQL 16 with sqlx 0.8 (compile-time checked queries)
- **Blockchain**: Solana SDK v2.0 + Helius RPC integration
- **CLI**: clap v4.5 with derive macros
- **Error Handling**: anyhow + thiserror for comprehensive error context
- **Logging**: tracing for structured logging

## 🎓 Key Learnings

### Solana Data Structure
- JsonParsed encoding simplifies instruction parsing
- Account keys array determines signer/fee payer roles
- Program IDs provide reliable classification basis

### Database Design
- UPSERT enables idempotent operations
- JSONB storage provides flexibility for evolving schemas
- Foreign keys ensure referential integrity
- GIN indexes on JSONB optimize query performance

### Rust Patterns
- `Arc<T>` for sharing non-Clone types across async tasks
- Database transactions ensure atomicity
- sqlx compile-time checking catches SQL errors at build time
- clap derive macros create elegant, type-safe CLIs

### Error Handling
- Exponential backoff handles transient network failures
- Detailed error context tracks failure stages
- Continue-on-error pattern processes remaining blocks

## 💡 Future Enhancements

### Enhanced Classification
- More program-specific parsers (MEV, governance, staking)
- NFT metadata extraction and enrichment
- DeFi protocol-specific analysis

### Data Analysis
- SQL views for common analytics queries
- Transaction volume and pattern analytics
- Fee analysis and trends over time

### API Layer
- REST API for querying stored data
- WebSocket for real-time transaction updates
- GraphQL for flexible data queries

### Visualization
- Web dashboard for metrics
- Transaction flow diagrams
- Volume, fee, and program interaction charts

### Optimization
- Parallel block fetching with async concurrency
- True bulk INSERT using PostgreSQL UNNEST
- In-memory caching for program registry

## 🛠️ Development

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy -- --deny warnings

# Run full checks
just full-check

# Run tests
cargo test
```

### Project Structure

```
src/
├── cli.rs           # Command-line interface
├── db/              # Database connection and migrations
├── etl/             # ETL pipeline modules
│   ├── extract.rs   # Block fetching from RPC
│   ├── transform.rs # Transaction classification
│   ├── load.rs      # Database insertion
│   └── parsers/     # Instruction parsers
├── models.rs        # Data models
├── pipeline.rs      # Pipeline orchestration
└── rpc/             # RPC client wrapper

migrations/          # Database migrations
docs/                # Documentation
```

## � Project Summary

### 🎯 Project Goal
Build a production-quality Rust ETL pipeline to extract Solana blockchain data, classify transactions into meaningful categories, and store them in PostgreSQL for analysis and querying.

### ✅ Success Criteria - All Met!

✅ Extract blocks from Solana mainnet via RPC  
✅ Classify transactions into 6 meaningful categories  
✅ Store data in structured PostgreSQL schema  
✅ Handle errors gracefully with retry logic  
✅ Process arbitrary block ranges efficiently  
✅ Provide CLI for flexible configuration  
✅ Track and report real-time statistics  
✅ Maintain high success rate (99%+)  
✅ Achieve good throughput (200+ txs/sec)  
✅ Create comprehensive documentation  

### 🏆 What This Project Demonstrates

- **Production ETL Pipelines**: Building robust data pipelines in Rust with proper error handling
- **Blockchain Data Processing**: Working with Solana's complex transaction structure and RPC APIs
- **Database Design**: Effective schema design with proper relationships, indexes, and JSONB flexibility
- **Async Rust**: Leveraging Tokio for concurrent operations and efficient I/O
- **CLI Design**: Creating user-friendly command-line interfaces with clap
- **Error Resilience**: Implementing retry logic, exponential backoff, and graceful degradation

**Status**: ✅ **Feature Complete**  
**Date**: October 23, 2025  
**Author**: 0xfave

## 🤝 Contributing

This is a personal side project, but suggestions and improvements are welcome!

## 📝 License

This project is licensed under MIT.

## 🙏 Acknowledgments

- Built with [Solana SDK](https://github.com/solana-labs/solana)
- Uses [Helius RPC](https://www.helius.dev/) for enhanced Solana data access
- Inspired by the need for structured Solana transaction data
