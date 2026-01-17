# GoQuant Assignment - Collateral Vault System

A comprehensive collateral vault management system built on Solana blockchain using Anchor framework. This system allows users to deposit, withdraw, lock, and unlock collateral tokens while maintaining real-time synchronization between on-chain state and off-chain database.

## 🏗️ Architecture

### System Components

1. **Anchor Program** (`programs/goquant_assignment/`)

   - On-chain Solana program handling vault operations
   - Manages collateral deposits, withdrawals, locking/unlocking
   - Emits events for off-chain processing

2. **Backend API** (`backend/`)

   - REST API server built with Actix-web
   - PostgreSQL database for persistent storage
   - Redis-like caching layer
   - Real-time WebSocket updates
   - Event listener for on-chain synchronization

3. **Shared Library** (`shared/`)

   - Common data models and utilities
   - Used by both program and backend

4. **Database Migrations** (`migrations/`)
   - SQL migrations for PostgreSQL schema
   - Managed with sqlx

### Data Flow

```
User Request → API → Transaction Builder → Solana RPC → On-chain Program
                                                        ↓
Event Listener ← WebSocket/Polling ← Solana RPC ← Events
    ↓
Database Update → Cache Invalidation → WebSocket Broadcast → Frontend
```

## � Project Structure

```
goquant_assignment/
├── Anchor.toml                 # Anchor configuration
├── Cargo.toml                  # Workspace Cargo.toml
├── package.json               # Node.js dependencies
├── tsconfig.json              # TypeScript configuration
├── rust-toolchain.toml        # Rust toolchain version
├── setup.sh                   # Development setup script
├── README.md                  # This file
├── examples/                  # API usage examples
│   └── README.md
├── migrations/                # Database migrations
│   └── 20260111110050_initial_schema.sql
├── programs/                  # Solana programs
│   └── goquant_assignment/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs         # Program entry point
│           ├── error.rs       # Program errors
│           ├── instructions/  # Program instructions
│           └── states/        # Program state structs
├── backend/                   # Rust backend API server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # Server entry point
│       ├── config.rs          # Configuration loading
│       ├── database.rs        # PostgreSQL database layer
│       ├── cache.rs           # In-memory caching
│       ├── websocket.rs       # WebSocket real-time updates
│       ├── api/               # REST API endpoints
│       ├── services/          # Business logic services
│       ├── monitering/        # Monitoring and metrics
│       └── api_tests.rs       # Integration tests
├── shared/                    # Shared Rust library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── models.rs          # Shared data models
│       ├── error.rs           # Shared error types
│       └── utils.rs           # Utility functions
├── tests/                     # TypeScript tests
│   └── goquant_assignment.ts
└── target/                    # Build artifacts
```

## �🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Node.js 18+
- PostgreSQL 14+
- Solana CLI tools
- Anchor framework

### Quick Setup Script

For a quick development setup, run:

```bash
./setup.sh
```

This script will:

- Check prerequisites
- Create environment configuration
- Install dependencies
- Build the Anchor program
- Run all tests

### Manual Setup

1. **Clone the repository**

   ```bash
   git clone <repository-url>
   cd goquant_assignment
   ```

2. **Install dependencies**

   ```bash
   # Install Anchor
   cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
   avm install latest
   avm use latest

   # Install Solana CLI
   sh -c "$(curl -sSfL https://release.solana.com/v1.16.0/install)"

   # Install Node.js dependencies
   cd app && npm install
   ```

3. **Database Setup**

   ```bash
   # Create PostgreSQL database
   createdb goquant_vault

   # Set environment variables
   export DATABASE_URL="postgresql://username:password@localhost/goquant_vault"
   export SOLANA_RPC_URL="https://api.devnet.solana.com"
   export PROGRAM_ID="A9JDc7TrKR5Qyot3W3t6UQaRz4CTgEURemuSUkWfP9hs"
   ```

4. **Build and Deploy**

   ```bash
   # Build the Anchor program
   anchor build

   # Deploy to devnet
   anchor deploy

   # Run database migrations
   cd backend && cargo run --bin migrate

   # Start the backend
   cargo run
   ```

## 📡 API Documentation

### Base URL

```
http://localhost:3000/api/v1
```

### Health Check

```http
GET /health
```

### Vault Operations

#### Initialize Vault

```http
POST /api/v1/vault/initialize
Content-Type: application/json

{
  "vault_pubkey": "string",
  "owner_pubkey": "string",
  "token_account": "string"
}
```

#### Get Vault Balance

```http
GET /api/v1/vault/balance/{vault_pubkey}
```

#### Deposit Collateral

```http
POST /api/v1/vault/deposit
Content-Type: application/json

{
  "vault_pubkey": "string",
  "amount": 1000000,
  "tx_signature": "string"
}
```

#### Withdraw Collateral

```http
POST /api/v1/vault/withdraw
Content-Type: application/json

{
  "vault_pubkey": "string",
  "amount": 500000,
  "tx_signature": "string"
}
```

#### Lock Collateral

```http
POST /api/v1/vault/lock
Content-Type: application/json

{
  "vault_pubkey": "string",
  "amount": 200000,
  "tx_signature": "string"
}
```

#### Unlock Collateral

```http
POST /api/v1/vault/unlock
Content-Type: application/json

{
  "vault_pubkey": "string",
  "amount": 200000,
  "tx_signature": "string"
}
```

### Transaction Operations

#### Build Deposit Transaction

```http
POST /api/v1/transaction/deposit
Content-Type: application/json

{
  "vault_pubkey": "string",
  "user_pubkey": "string",
  "amount": 1000000
}
```

#### Build Withdraw Transaction

```http
POST /api/v1/transaction/withdraw
Content-Type: application/json

{
  "vault_pubkey": "string",
  "user_pubkey": "string",
  "amount": 500000
}
```

### WebSocket Real-time Updates

Connect to `/ws` for real-time vault updates:

```javascript
const ws = new WebSocket("ws://localhost:3000/ws");

ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log("Vault update:", update);
};
```

Update types:

- `balance_update`: Vault balance changes
- `deposit`: Deposit events
- `withdrawal`: Withdrawal events
- `lock`: Collateral locked
- `unlock`: Collateral unlocked
- `tvl_update`: Total Value Locked changes

## 🧪 Testing

### Backend Tests

```bash
cd backend
cargo test
```

### API Integration Tests

```bash
cd backend
cargo run --bin api_tests
```

### Anchor Program Tests

```bash
anchor test
```

## 🔧 Configuration

### Environment Variables

| Variable                          | Description                     | Default                         |
| --------------------------------- | ------------------------------- | ------------------------------- |
| `DATABASE_URL`                    | PostgreSQL connection string    | Required                        |
| `SOLANA_RPC_URL`                  | Solana RPC endpoint             | `https://api.devnet.solana.com` |
| `PROGRAM_ID`                      | Deployed program ID             | Required                        |
| `HOST`                            | Server bind address             | `0.0.0.0`                       |
| `PORT`                            | Server port                     | `3000`                          |
| `MAX_DB_CONNECTIONS`              | Database connection pool size   | `50`                            |
| `CACHE_TTL_SECONDS`               | Cache TTL in seconds            | `300`                           |
| `RECONCILIATION_INTERVAL_SECONDS` | Balance reconciliation interval | `3600`                          |
| `MONITORING_INTERVAL_SECONDS`     | Monitoring interval             | `60`                            |

## 📊 Monitoring & Metrics

### Health Endpoints

- `GET /health` - Service health status
- `GET /metrics` - Prometheus metrics

### Monitoring Features

- Vault balance reconciliation
- On-chain vs off-chain balance validation
- Transaction monitoring
- Performance metrics
- Alert system for discrepancies

## 🗄️ Database Schema

### Core Tables

#### vaults

```sql
CREATE TABLE vaults (
  vault_pubkey VARCHAR PRIMARY KEY,
  owner_pubkey VARCHAR NOT NULL,
  token_account VARCHAR NOT NULL,
  total_balance BIGINT NOT NULL DEFAULT 0,
  available_balance BIGINT NOT NULL DEFAULT 0,
  locked_balance BIGINT NOT NULL DEFAULT 0,
  total_deposited BIGINT NOT NULL DEFAULT 0,
  total_withdrawn BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

#### transactions

```sql
CREATE TABLE transactions (
  id SERIAL PRIMARY KEY,
  vault_pubkey VARCHAR NOT NULL REFERENCES vaults(vault_pubkey),
  tx_signature VARCHAR UNIQUE NOT NULL,
  tx_type VARCHAR NOT NULL,
  amount BIGINT,
  fee BIGINT,
  status VARCHAR NOT NULL DEFAULT 'pending',
  block_time TIMESTAMP WITH TIME ZONE,
  slot BIGINT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

## 🔒 Security Considerations

- All transactions require valid signatures
- Balance validation on every operation
- Reconciliation checks for discrepancies
- Audit trail for all operations
- Rate limiting and monitoring

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 📞 Support

For questions or issues, please open a GitHub issue or contact the development team.
