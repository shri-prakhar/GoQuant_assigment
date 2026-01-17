#!/bin/bash

# GoQuant Assignment - Development Setup Script
# This script helps set up the development environment and run tests

set -e

echo "GoQuant Assignment - Development Setup"
echo "========================================"

# Check if we're in the right directory
if [ ! -f "Anchor.toml" ]; then
    echo "Error: Please run this script from the project root directory"
    exit 1
fi

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

echo "📋 Checking prerequisites..."

# Check for required tools
if ! command_exists cargo; then
    echo "Cargo is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

if ! command_exists node; then
    echo "Node.js is not installed. Please install from https://nodejs.org/"
    exit 1
fi

if ! command_exists solana; then
    echo "Solana CLI is not installed. Please install from https://docs.solana.com/cli/install-solana-cli-tools"
    exit 1
fi

if ! command_exists anchor; then
    echo "Anchor is not installed. Please install from https://www.anchor-lang.com/docs/installation"
    exit 1
fi

echo "Prerequisites check passed"

# Setup environment
echo "Setting up environment..."

# Create .env file if it doesn't exist
if [ ! -f ".env" ]; then
    echo "Creating .env file..."
    cat > .env << EOF
# Database
DATABASE_URL=postgresql://postgres:password@localhost/goquant_vault

# Solana
SOLANA_RPC_URL=https://api.devnet.solana.com
PROGRAM_ID=A9JDc7TrKR5Qyot3W3t6UQaRz4CTgEURemuSUkWfP9hs

# Server
HOST=0.0.0.0
PORT=3000

# Performance
MAX_DB_CONNECTIONS=50
CACHE_TTL_SECONDS=300
RECONCILIATION_INTERVAL_SECONDS=3600
MONITORING_INTERVAL_SECONDS=60
EOF
    echo "Created .env file"
else
    echo ".env file already exists"
fi

# Install Node.js dependencies
echo " Installing Node.js dependencies..."
cd app
npm install
cd ..
echo " Node.js dependencies installed"

# Build Anchor program
echo "Building Anchor program..."
anchor build
echo "Anchor program built"

# Run tests
echo "Running tests..."

# Backend tests
echo "  Running backend tests..."
cd backend
cargo test
cd ..
echo "Backend tests passed"

# Anchor tests
echo "Running Anchor tests..."
anchor test
echo " Anchor tests passed"

echo ""
echo "Setup complete!"
echo ""
echo "To start the development server:"
echo "1. Start PostgreSQL and create the database:"
echo "   createdb goquant_vault"
echo ""
echo "2. Run database migrations:"
echo "   cd backend && cargo run --bin migrate"
echo ""
echo "3. Start the backend server:"
echo "   cd backend && cargo run"
echo ""
echo "4. In another terminal, start the frontend:"
echo "   cd app && npm run dev"
echo ""
echo "The API will be available at http://localhost:3000"
echo "Health check: http://localhost:3000/health"