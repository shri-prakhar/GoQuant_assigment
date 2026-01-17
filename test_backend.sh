#!/bin/bash
# ============================================================================
# Collateral Vault Backend Test Script
# ============================================================================
# This script tests all major backend API endpoints
# 
# Prerequisites:
#   1. PostgreSQL running with collateral_vault database
#   2. Solana local validator running: solana-test-validator --reset
#   3. Backend server running: cargo run (in backend directory)
#
# Usage:
#   chmod +x test_backend.sh
#   ./test_backend.sh
# ============================================================================

set -e

# Configuration
BASE_URL="${BASE_URL:-http://localhost:3000}"
VERBOSE="${VERBOSE:-false}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
PASSED=0
FAILED=0
SKIPPED=0

# Generate unique IDs for this test run
TIMESTAMP=$(date +%s)
ALICE_VAULT="AliceVault${TIMESTAMP}12345678901234567890"
ALICE_OWNER="AliceOwner123456789012345678901234567890"
ALICE_TOKEN="AliceToken123456789012345678901234567890"
BOB_VAULT="BobVault${TIMESTAMP}123456789012345678901234"
BOB_OWNER="BobOwner12345678901234567890123456789012"
BOB_TOKEN="BobToken12345678901234567890123456789012"

# Helper functions
print_header() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}  $1"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
}

print_test() {
    printf "  %-50s " "$1"
}

pass() {
    echo -e "${GREEN}✅ PASS${NC}"
    ((PASSED++))
}

fail() {
    echo -e "${RED}❌ FAIL${NC} - $1"
    ((FAILED++))
}

skip() {
    echo -e "${YELLOW}⚠️  SKIP${NC} - $1"
    ((SKIPPED++))
}

# Check if server is available
check_server() {
    print_test "Server Health Check"
    RESPONSE=$(curl -s -w "\n%{http_code}" $BASE_URL/health 2>/dev/null || echo "000")
    HTTP_CODE=$(echo "$RESPONSE" | tail -n 1)
    BODY=$(echo "$RESPONSE" | head -n -1)
    
    if [ "$HTTP_CODE" = "200" ]; then
        STATUS=$(echo "$BODY" | jq -r '.status' 2>/dev/null)
        if [ "$STATUS" = "ok" ]; then
            pass
            return 0
        fi
    fi
    fail "Server not responding (HTTP $HTTP_CODE)"
    return 1
}

# Wait for server
wait_for_server() {
    echo "Waiting for server at $BASE_URL..."
    for i in {1..10}; do
        if curl -s $BASE_URL/health > /dev/null 2>&1; then
            echo -e "${GREEN}Server is ready!${NC}"
            return 0
        fi
        echo "  Attempt $i/10..."
        sleep 1
    done
    echo -e "${RED}Server not available after 10 attempts${NC}"
    return 1
}

# ============================================================================
# TEST FUNCTIONS
# ============================================================================

test_health_endpoint() {
    print_header "Health & Metrics Endpoints"
    
    # Health check
    print_test "GET /health"
    RESPONSE=$(curl -s $BASE_URL/health)
    STATUS=$(echo "$RESPONSE" | jq -r '.status' 2>/dev/null)
    [ "$STATUS" = "ok" ] && pass || fail "Expected status 'ok', got '$STATUS'"
    
    # Metrics endpoint
    print_test "GET /metrics"
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" $BASE_URL/metrics)
    [ "$HTTP_CODE" = "200" ] && pass || fail "HTTP $HTTP_CODE"
}

test_vault_initialization() {
    print_header "Vault Initialization"
    
    # Initialize Alice's vault
    print_test "Initialize Alice's vault"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/initialize \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"owner_pubkey\": \"$ALICE_OWNER\",
            \"token_account\": \"$ALICE_TOKEN\"
        }")
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success' 2>/dev/null)
    [ "$SUCCESS" = "true" ] && pass || fail "$(echo $RESPONSE | jq -r '.error')"
    
    # Initialize Bob's vault
    print_test "Initialize Bob's vault"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/initialize \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$BOB_VAULT\",
            \"owner_pubkey\": \"$BOB_OWNER\",
            \"token_account\": \"$BOB_TOKEN\"
        }")
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success' 2>/dev/null)
    [ "$SUCCESS" = "true" ] && pass || fail "$(echo $RESPONSE | jq -r '.error')"
    
    # Verify vault created with zero balance
    print_test "Verify initial balance is zero"
    RESPONSE=$(curl -s $BASE_URL/api/v1/vault/balance/$ALICE_VAULT)
    BALANCE=$(echo "$RESPONSE" | jq -r '.data.total_balance' 2>/dev/null)
    [ "$BALANCE" = "0" ] && pass || fail "Expected 0, got $BALANCE"
}

test_deposit_operations() {
    print_header "Deposit Operations"
    
    # First deposit - 1000 USDT (1,000,000,000 lamports)
    print_test "Deposit 1000 USDT to Alice"
    TX_SIG="DepositTx1_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/deposit \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"amount\": 1000000000,
            \"tx_signature\": \"$TX_SIG\"
        }")
    BALANCE=$(echo "$RESPONSE" | jq -r '.data.total_balance' 2>/dev/null)
    [ "$BALANCE" = "1000000000" ] && pass || fail "Expected 1000000000, got $BALANCE"
    
    # Second deposit - 500 USDT
    print_test "Deposit 500 USDT to Alice (cumulative)"
    TX_SIG="DepositTx2_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/deposit \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"amount\": 500000000,
            \"tx_signature\": \"$TX_SIG\"
        }")
    BALANCE=$(echo "$RESPONSE" | jq -r '.data.total_balance' 2>/dev/null)
    [ "$BALANCE" = "1500000000" ] && pass || fail "Expected 1500000000, got $BALANCE"
    
    # Deposit to Bob's vault
    print_test "Deposit 2000 USDT to Bob"
    TX_SIG="DepositTx3_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/deposit \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$BOB_VAULT\",
            \"amount\": 2000000000,
            \"tx_signature\": \"$TX_SIG\"
        }")
    BALANCE=$(echo "$RESPONSE" | jq -r '.data.total_balance' 2>/dev/null)
    [ "$BALANCE" = "2000000000" ] && pass || fail "Expected 2000000000, got $BALANCE"
}

test_withdrawal_operations() {
    print_header "Withdrawal Operations"
    
    # Withdraw 200 USDT from Alice
    print_test "Withdraw 200 USDT from Alice"
    TX_SIG="WithdrawTx1_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/withdraw \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"amount\": 200000000,
            \"tx_signature\": \"$TX_SIG\"
        }")
    BALANCE=$(echo "$RESPONSE" | jq -r '.data.total_balance' 2>/dev/null)
    [ "$BALANCE" = "1300000000" ] && pass || fail "Expected 1300000000, got $BALANCE"
    
    # Verify total_withdrawn updated
    print_test "Verify total_withdrawn tracked"
    RESPONSE=$(curl -s $BASE_URL/api/v1/vault/balance/$ALICE_VAULT)
    WITHDRAWN=$(echo "$RESPONSE" | jq -r '.data.total_withdrawn' 2>/dev/null)
    [ "$WITHDRAWN" = "200000000" ] && pass || fail "Expected 200000000, got $WITHDRAWN"
}

test_lock_unlock_operations() {
    print_header "Lock/Unlock Collateral Operations"
    
    # Lock 500 USDT for margin
    print_test "Lock 500 USDT collateral"
    TX_SIG="LockTx1_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/lock \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"amount\": 500000000,
            \"tx_signature\": \"$TX_SIG\"
        }")
    LOCKED=$(echo "$RESPONSE" | jq -r '.data.locked_balance' 2>/dev/null)
    [ "$LOCKED" = "500000000" ] && pass || fail "Expected locked 500000000, got $LOCKED"
    
    # Verify available balance decreased
    print_test "Verify available balance decreased"
    RESPONSE=$(curl -s $BASE_URL/api/v1/vault/balance/$ALICE_VAULT)
    AVAILABLE=$(echo "$RESPONSE" | jq -r '.data.available_balance' 2>/dev/null)
    [ "$AVAILABLE" = "800000000" ] && pass || fail "Expected available 800000000, got $AVAILABLE"
    
    # Unlock 200 USDT
    print_test "Unlock 200 USDT collateral"
    TX_SIG="UnlockTx1_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/unlock \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"amount\": 200000000,
            \"tx_signature\": \"$TX_SIG\"
        }")
    LOCKED=$(echo "$RESPONSE" | jq -r '.data.locked_balance' 2>/dev/null)
    [ "$LOCKED" = "300000000" ] && pass || fail "Expected locked 300000000, got $LOCKED"
    
    # Verify available balance increased
    print_test "Verify available balance increased"
    RESPONSE=$(curl -s $BASE_URL/api/v1/vault/balance/$ALICE_VAULT)
    AVAILABLE=$(echo "$RESPONSE" | jq -r '.data.available_balance' 2>/dev/null)
    [ "$AVAILABLE" = "1000000000" ] && pass || fail "Expected available 1000000000, got $AVAILABLE"
}

test_balance_invariants() {
    print_header "Balance Invariant Checks"
    
    # Check: total = available + locked
    print_test "Invariant: total = available + locked"
    RESPONSE=$(curl -s $BASE_URL/api/v1/vault/balance/$ALICE_VAULT)
    TOTAL=$(echo "$RESPONSE" | jq -r '.data.total_balance' 2>/dev/null)
    AVAILABLE=$(echo "$RESPONSE" | jq -r '.data.available_balance' 2>/dev/null)
    LOCKED=$(echo "$RESPONSE" | jq -r '.data.locked_balance' 2>/dev/null)
    CALC=$((AVAILABLE + LOCKED))
    [ "$TOTAL" = "$CALC" ] && pass || fail "Total $TOTAL != Available $AVAILABLE + Locked $LOCKED"
    
    # Check: total_deposited - total_withdrawn = total_balance
    print_test "Invariant: deposited - withdrawn = total"
    DEPOSITED=$(echo "$RESPONSE" | jq -r '.data.total_deposited' 2>/dev/null)
    WITHDRAWN=$(echo "$RESPONSE" | jq -r '.data.total_withdrawn' 2>/dev/null)
    CALC=$((DEPOSITED - WITHDRAWN))
    [ "$TOTAL" = "$CALC" ] && pass || fail "Deposited $DEPOSITED - Withdrawn $WITHDRAWN != Total $TOTAL"
}

test_query_operations() {
    print_header "Query Operations"
    
    # Get vault by owner
    print_test "Get vault by owner pubkey"
    RESPONSE=$(curl -s $BASE_URL/api/v1/vault/owner/$ALICE_OWNER)
    VAULT_PK=$(echo "$RESPONSE" | jq -r '.data.vault_pubkey' 2>/dev/null)
    [ "$VAULT_PK" = "$ALICE_VAULT" ] && pass || fail "Expected $ALICE_VAULT, got $VAULT_PK"
    
    # List all vaults
    print_test "List vaults with pagination"
    RESPONSE=$(curl -s "$BASE_URL/api/v1/vault/list?limit=10&offset=0")
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success' 2>/dev/null)
    [ "$SUCCESS" = "true" ] && pass || fail "List failed"
    
    # Get TVL stats
    print_test "Get Total Value Locked stats"
    RESPONSE=$(curl -s $BASE_URL/api/v1/vault/tvl)
    TVL=$(echo "$RESPONSE" | jq -r '.data.total_value_locked' 2>/dev/null)
    [ -n "$TVL" ] && [ "$TVL" != "null" ] && pass || fail "TVL not returned"
    
    # Get nonexistent vault
    print_test "Get nonexistent vault returns error"
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" $BASE_URL/api/v1/vault/balance/NonExistent123456789012345678901234567890)
    [ "$HTTP_CODE" = "404" ] && pass || fail "Expected 404, got $HTTP_CODE"
}

test_transaction_history() {
    print_header "Transaction History"
    
    # Get all transaction history
    print_test "Get global transaction history"
    RESPONSE=$(curl -s "$BASE_URL/api/v1/transaction/history?limit=50")
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success' 2>/dev/null)
    [ "$SUCCESS" = "true" ] && pass || fail "History query failed"
    
    # Get vault-specific transactions
    print_test "Get Alice's transaction history"
    RESPONSE=$(curl -s "$BASE_URL/api/v1/transaction/history/$ALICE_VAULT?limit=10")
    COUNT=$(echo "$RESPONSE" | jq -r '.data.transactions | length' 2>/dev/null)
    [ "$COUNT" -gt 0 ] 2>/dev/null && pass || fail "Expected transactions, got $COUNT"
    
    # Get specific transaction
    print_test "Get transaction by signature"
    TX_SIG="DepositTx1_${TIMESTAMP}"
    RESPONSE=$(curl -s "$BASE_URL/api/v1/transaction/$TX_SIG")
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success' 2>/dev/null)
    [ "$SUCCESS" = "true" ] && pass || fail "Transaction not found"
}

test_error_handling() {
    print_header "Error Handling"
    
    # Withdraw more than available
    print_test "Reject withdrawal > available balance"
    TX_SIG="FailWithdraw_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/withdraw \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"amount\": 9999999999999,
            \"tx_signature\": \"$TX_SIG\"
        }")
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success' 2>/dev/null)
    [ "$SUCCESS" = "false" ] && pass || fail "Should reject insufficient balance"
    
    # Lock more than available
    print_test "Reject lock > available balance"
    TX_SIG="FailLock_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/lock \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"amount\": 9999999999999,
            \"tx_signature\": \"$TX_SIG\"
        }")
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success' 2>/dev/null)
    [ "$SUCCESS" = "false" ] && pass || fail "Should reject insufficient balance"
    
    # Unlock more than locked
    print_test "Reject unlock > locked balance"
    TX_SIG="FailUnlock_${TIMESTAMP}"
    RESPONSE=$(curl -s -X POST $BASE_URL/api/v1/vault/unlock \
        -H "Content-Type: application/json" \
        -d "{
            \"vault_pubkey\": \"$ALICE_VAULT\",
            \"amount\": 9999999999999,
            \"tx_signature\": \"$TX_SIG\"
        }")
    SUCCESS=$(echo "$RESPONSE" | jq -r '.success' 2>/dev/null)
    [ "$SUCCESS" = "false" ] && pass || fail "Should reject insufficient locked"
    
    # Invalid pubkey format
    print_test "Reject invalid pubkey format"
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" $BASE_URL/api/v1/vault/balance/invalid)
    [ "$HTTP_CODE" = "404" ] || [ "$HTTP_CODE" = "400" ] && pass || fail "Expected 400/404, got $HTTP_CODE"
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

main() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     ${GREEN}COLLATERAL VAULT BACKEND TEST SUITE${NC}                       ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}                                                                ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}  Target: $BASE_URL                              ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}  Time:   $(date)                    ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
    
    # Check server availability
    if ! wait_for_server; then
        echo ""
        echo -e "${RED}ERROR: Backend server not available at $BASE_URL${NC}"
        echo ""
        echo "Please ensure:"
        echo "  1. PostgreSQL is running"
        echo "  2. Solana local validator is running: solana-test-validator --reset"
        echo "  3. Backend is running: cd backend && cargo run"
        echo ""
        exit 1
    fi
    
    # Run all tests
    test_health_endpoint
    test_vault_initialization
    test_deposit_operations
    test_withdrawal_operations
    test_lock_unlock_operations
    test_balance_invariants
    test_query_operations
    test_transaction_history
    test_error_handling
    
    # Print summary
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}                        TEST SUMMARY                            ${BLUE}║${NC}"
    echo -e "${BLUE}╠════════════════════════════════════════════════════════════════╣${NC}"
    echo -e "${BLUE}║${NC}  ${GREEN}PASSED:${NC}  $PASSED                                                   ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}  ${RED}FAILED:${NC}  $FAILED                                                   ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}  ${YELLOW}SKIPPED:${NC} $SKIPPED                                                   ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
    
    # Final vault state
    echo ""
    echo "Final Alice Vault State:"
    curl -s $BASE_URL/api/v1/vault/balance/$ALICE_VAULT | jq '.data | {
        total_balance: (.total_balance / 1000000 | tostring + " USDT"),
        available_balance: (.available_balance / 1000000 | tostring + " USDT"),
        locked_balance: (.locked_balance / 1000000 | tostring + " USDT"),
        total_deposited: (.total_deposited / 1000000 | tostring + " USDT"),
        total_withdrawn: (.total_withdrawn / 1000000 | tostring + " USDT")
    }'
    
    echo ""
    echo "Final Bob Vault State:"
    curl -s $BASE_URL/api/v1/vault/balance/$BOB_VAULT | jq '.data | {
        total_balance: (.total_balance / 1000000 | tostring + " USDT"),
        available_balance: (.available_balance / 1000000 | tostring + " USDT"),
        locked_balance: (.locked_balance / 1000000 | tostring + " USDT")
    }'
    
    echo ""
    
    # Exit with appropriate code
    [ $FAILED -eq 0 ] && exit 0 || exit 1
}

# Run main
main "$@"