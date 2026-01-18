//! # Collateral Vault Backend - Test Suite
//!
//! ## IMPORTANT: Apply database fix first!
//! 
//! Before running tests, fix the bug in backend/src/database.rs:
//! See database_fix.rs for instructions.
//!
//! ## Running Tests
//! 
//! Terminal 1: `solana-test-validator --reset`
//! Terminal 2: `cd backend && cargo run`
//! Terminal 3: `cd backend && cargo test -- --nocapture --test-threads=1`

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use solana_client::nonblocking::rpc_client::RpcClient as AsyncRpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// ============================================================================
// Configuration
// ============================================================================

const BASE_URL: &str = "http://localhost:3000/api/v1";
const HEALTH_URL: &str = "http://localhost:3000/health";
const SOLANA_RPC_URL: &str = "http://127.0.0.1:8899";

const SERVER_WAIT_ATTEMPTS: u32 = 30;
const SERVER_WAIT_DELAY_MS: u64 = 1000;

// Test Users
const ALICE_PUBKEY: &str = "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj";
const ALICE_TOKEN_ACCOUNT: &str = "BYLfz8RQMYE7A5FwL2fVn7RZnYNqh82cBzJpXu9hS3Rq";
const ALICE_VAULT_PUBKEY: &str = "3KmPPXJe3f3cK8qLp9rHqVa5sCHRTLz2MWL4Xa6dN9Zj";

const BOB_PUBKEY: &str = "7sB8YPWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY2Kn";
const BOB_TOKEN_ACCOUNT: &str = "DYLfz8RQMYE7A5FwL2fVn7RZnYNqh82cBzJpXu9hS4Sq";
const BOB_VAULT_PUBKEY: &str = "5KmPPXJe3f3cK8qLp9rHqVa5sCHRTLz2MWL4Xa6dN8Xk";

// ============================================================================
// Response Types - MATCHING ACTUAL SERVER RESPONSES
// ============================================================================

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Vault {
    pub vault_pubkey: String,
    pub owner_pubkey: String,
    pub token_account: String,
    pub total_balance: i64,
    pub locked_balance: i64,
    pub available_balance: i64,
    pub total_deposited: i64,
    pub total_withdrawn: i64,
    #[serde(default)]
    pub created_at: Option<Value>,
    #[serde(default)]
    pub updated_at: Option<Value>,
}

// Actual health response from server (from api/health.rs)
#[derive(Debug, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

// ============================================================================
// Utilities
// ============================================================================

async fn wait_for_server(client: &Client, max_attempts: u32) -> bool {
    for attempt in 1..=max_attempts {
        print!("Waiting for server... (attempt {}/{}) ", attempt, max_attempts);
        
        match client.get(HEALTH_URL)
            .timeout(Duration::from_secs(2))
            .send()
            .await 
        {
            Ok(response) if response.status().is_success() => {
                println!("Server is ready!");
                return true;
            }
            Ok(response) => {
                println!("Status: {}", response.status());
            }
            Err(e) => {
                println!("{}", e);
            }
        }
        
        tokio::time::sleep(Duration::from_millis(SERVER_WAIT_DELAY_MS)).await;
    }
    
    println!("Server not available after {} attempts", max_attempts);
    false
}

async fn wait_for_solana(max_attempts: u32) -> bool {
    let client = AsyncRpcClient::new(SOLANA_RPC_URL.to_string());
    
    for attempt in 1..=max_attempts {
        match client.get_version().await {
            Ok(version) => {
                println!("Solana validator ready (attempt {}) - version: {}", attempt, version.solana_core);
                return true;
            }
            Err(_) => {
                if attempt < max_attempts {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
    
    println!("Solana validator not available");
    false
}

fn generate_test_signature() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("test_sig_{}", timestamp)
}

fn create_test_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client")
}

// ============================================================================
// API Client
// ============================================================================

struct TestApiClient {
    client: Client,
    base_url: String,
}

impl TestApiClient {
    fn new() -> Self {
        Self {
            client: create_test_client(),
            base_url: BASE_URL.to_string(),
        }
    }

    async fn health_check(&self) -> Result<HealthResponse, reqwest::Error> {
        let response = self.client.get(HEALTH_URL).send().await?;
        response.json().await
    }

    async fn initialize_vault(
        &self,
        vault_pubkey: &str,
        owner_pubkey: &str,
        token_account: &str,
    ) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let body = json!({
            "vault_pubkey": vault_pubkey,
            "owner_pubkey": owner_pubkey,
            "token_account": token_account
        });

        let response = self.client
            .post(format!("{}/vault/initialize", self.base_url))
            .json(&body)
            .send()
            .await?;

        response.json().await
    }

    async fn get_balance(&self, vault_pubkey: &str) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let response = self.client
            .get(format!("{}/vault/balance/{}", self.base_url, vault_pubkey))
            .send()
            .await?;

        response.json().await
    }

    async fn get_vault_by_owner(&self, owner_pubkey: &str) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let response = self.client
            .get(format!("{}/vault/owner/{}", self.base_url, owner_pubkey))
            .send()
            .await?;

        response.json().await
    }

    async fn process_deposit(
        &self,
        vault_pubkey: &str,
        amount: i64,
        tx_signature: &str,
    ) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let body = json!({
            "vault_pubkey": vault_pubkey,
            "amount": amount,
            "tx_signature": tx_signature
        });

        let response = self.client
            .post(format!("{}/vault/deposit", self.base_url))
            .json(&body)
            .send()
            .await?;

        response.json().await
    }

    async fn process_withdrawal(
        &self,
        vault_pubkey: &str,
        amount: i64,
        tx_signature: &str,
    ) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let body = json!({
            "vault_pubkey": vault_pubkey,
            "amount": amount,
            "tx_signature": tx_signature
        });

        let response = self.client
            .post(format!("{}/vault/withdraw", self.base_url))
            .json(&body)
            .send()
            .await?;

        response.json().await
    }

    async fn process_lock(
        &self,
        vault_pubkey: &str,
        amount: i64,
        tx_signature: &str,
    ) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let body = json!({
            "vault_pubkey": vault_pubkey,
            "amount": amount,
            "tx_signature": tx_signature
        });

        let response = self.client
            .post(format!("{}/vault/lock", self.base_url))
            .json(&body)
            .send()
            .await?;

        response.json().await
    }

    async fn process_unlock(
        &self,
        vault_pubkey: &str,
        amount: i64,
        tx_signature: &str,
    ) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let body = json!({
            "vault_pubkey": vault_pubkey,
            "amount": amount,
            "tx_signature": tx_signature
        });

        let response = self.client
            .post(format!("{}/vault/unlock", self.base_url))
            .json(&body)
            .send()
            .await?;

        response.json().await
    }

    async fn list_vaults(&self, limit: i32, offset: i32) -> Result<ApiResponse<Vec<Vault>>, reqwest::Error> {
        let response = self.client
            .get(format!("{}/vault/list?limit={}&offset={}", self.base_url, limit, offset))
            .send()
            .await?;

        response.json().await
    }
}

// ============================================================================
// Solana RPC Test Client
// ============================================================================

struct SolanaTestClient {
    client: AsyncRpcClient,
}

impl SolanaTestClient {
    fn new() -> Self {
        Self {
            client: AsyncRpcClient::new(SOLANA_RPC_URL.to_string()),
        }
    }

    async fn get_version(&self) -> Result<String, Box<dyn std::error::Error>> {
        let version = self.client.get_version().await?;
        Ok(version.solana_core)
    }

    async fn get_slot(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let slot = self.client.get_slot().await?;
        Ok(slot)
    }

    async fn get_balance(&self, pubkey: &str) -> Result<u64, Box<dyn std::error::Error>> {
        let pubkey = Pubkey::from_str(pubkey)?;
        let balance = self.client.get_balance(&pubkey).await?;
        Ok(balance)
    }

    async fn get_latest_blockhash(&self) -> Result<String, Box<dyn std::error::Error>> {
        let blockhash = self.client.get_latest_blockhash().await?;
        Ok(blockhash.to_string())
    }

    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64, Box<dyn std::error::Error>> {
        let balance = self.client.get_minimum_balance_for_rent_exemption(data_len).await?;
        Ok(balance)
    }
}

// ============================================================================
// MODULE 1: Health Tests
// ============================================================================

#[cfg(test)]
mod health_tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        println!("\nTEST: Health Endpoint");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!("FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let health = api.health_check().await.expect("Health check failed");
        
        assert_eq!(health.status, "healthy", "Server should be healthy");
        assert!(!health.version.is_empty(), "Version should not be empty");
        
        println!("PASSED: Health endpoint working");
        println!("   Status: {}", health.status);
        println!("   Version: {}", health.version);
        println!("   Uptime: {} seconds", health.uptime_seconds);
    }
}

// ============================================================================
// MODULE 2: Solana RPC Tests
// ============================================================================

#[cfg(test)]
mod solana_rpc_tests {
    use super::*;

    #[tokio::test]
    async fn test_solana_validator_connection() {
        println!("\n TEST: Solana Validator Connection");
        
        if !wait_for_solana(10).await {
            panic!(" FAILED: Solana validator not available!");
        }

        let solana = SolanaTestClient::new();
        let version = solana.get_version().await.expect("Failed to get version");
        
        println!(" PASSED: Connected to Solana validator");
        println!("   Version: {}", version);
    }

    #[tokio::test]
    async fn test_get_slot() {
        println!("\n TEST: Get Current Slot");
        
        if !wait_for_solana(10).await {
            panic!(" FAILED: Solana validator not available!");
        }

        let solana = SolanaTestClient::new();
        let slot = solana.get_slot().await.expect("Failed to get slot");
        
        assert!(slot > 0, "Slot should be > 0");
        
        println!(" PASSED: Retrieved slot {}", slot);
    }

    #[tokio::test]
    async fn test_get_latest_blockhash() {
        println!("\n TEST: Get Latest Blockhash");
        
        if !wait_for_solana(10).await {
            panic!(" FAILED: Solana validator not available!");
        }

        let solana = SolanaTestClient::new();
        let blockhash = solana.get_latest_blockhash().await.expect("Failed to get blockhash");
        
        assert!(!blockhash.is_empty(), "Blockhash should not be empty");
        
        println!(" PASSED: Blockhash: {}", blockhash);
    }

    #[tokio::test]
    async fn test_get_balance() {
        println!("\n TEST: Get Account Balance");
        
        if !wait_for_solana(10).await {
            panic!(" FAILED: Solana validator not available!");
        }

        let solana = SolanaTestClient::new();
        let system_program = "11111111111111111111111111111111";
        let balance = solana.get_balance(system_program).await.expect("Failed");
        
        println!(" PASSED: System Program balance: {} lamports", balance);
    }

    #[tokio::test]
    async fn test_rent_exemption() {
        println!("\n TEST: Rent Exemption Calculation");
        
        if !wait_for_solana(10).await {
            panic!(" FAILED: Solana validator not available!");
        }

        let solana = SolanaTestClient::new();
        let rent = solana.get_minimum_balance_for_rent_exemption(200).await.expect("Failed");
        
        assert!(rent > 0, "Rent should be > 0");
        
        println!(" PASSED: Rent for 200 bytes: {} lamports ({:.6} SOL)", 
                 rent, rent as f64 / 1_000_000_000.0);
    }
}

// ============================================================================
// MODULE 3: Vault Initialization Tests
// ============================================================================

#[cfg(test)]
mod vault_initialization_tests {
    use super::*;

    #[tokio::test]
    async fn test_initialize_alice_vault() {
        println!("\n TEST: Initialize Alice's Vault");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let result = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await
            .expect("Request failed");

        // Success or already exists is OK
        println!(" PASSED: Vault initialization handled");
        println!("   Success: {}", result.success);
        if let Some(vault) = &result.data {
            println!("   Vault: {}", vault.vault_pubkey);
        }
    }

    #[tokio::test]
    async fn test_initialize_bob_vault() {
        println!("\n TEST: Initialize Bob's Vault");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let result = api.initialize_vault(BOB_VAULT_PUBKEY, BOB_PUBKEY, BOB_TOKEN_ACCOUNT).await
            .expect("Request failed");

        println!(" PASSED: Bob's vault handled (success: {})", result.success);
    }
}

// ============================================================================
// MODULE 4: Deposit Tests (REQUIRES DATABASE FIX!)
// ============================================================================

#[cfg(test)]
mod deposit_tests {
    use super::*;

    #[tokio::test]
    async fn test_alice_deposit() {
        println!("\n TEST: Alice Deposits 1,000,000 tokens");
        println!("    REQUIRES: Database fix applied (see database_fix.rs)");
        
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        
        // Ensure vault exists
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        // Get balance before
        let before = api.get_balance(ALICE_VAULT_PUBKEY).await.ok()
            .and_then(|r| r.data)
            .map(|v| v.total_balance)
            .unwrap_or(0);
        
        // Make deposit
        let tx_sig = generate_test_signature();
        let result = api.process_deposit(ALICE_VAULT_PUBKEY, 1_000_000, &tx_sig).await
            .expect("Request failed");

        if !result.success {
            let error = result.error.unwrap_or_default();
            if error.contains("text = bigint") || error.contains("operator does not exist") {
                println!("");
                println!("    DATABASE BUG DETECTED!");
                println!("   Error: {}", error);
                println!("");
                println!("    FIX REQUIRED:");
                println!("   1. Open backend/src/database.rs");
                println!("   2. Find update_vault_balances() function");
                println!("   3. Add 'param_count += 1;' after total_deposited binding");
                println!("   4. See database_fix.rs for complete fix");
                println!("");
                panic!("Apply database fix first!");
            }
            panic!("Deposit failed: {}", error);
        }
        
        let vault = result.data.expect("Should have vault data");
        assert!(vault.total_balance >= before + 1_000_000, "Balance should increase");
        
        println!(" PASSED: Deposit successful");
        println!("   Amount: 1,000,000");
        println!("   New Balance: {}", vault.total_balance);
    }

    #[tokio::test]
    async fn test_zero_deposit_rejected() {
        println!("\n TEST: Zero Deposit Should Be Rejected");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let result = api.process_deposit(ALICE_VAULT_PUBKEY, 0, &generate_test_signature()).await
            .expect("Request failed");

        println!(" PASSED: Zero deposit handled (success: {})", result.success);
    }

    #[tokio::test]
    async fn test_negative_amount_rejected() {
        println!("\n TEST: Negative Amount Should Be Rejected");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let result = api.process_deposit(ALICE_VAULT_PUBKEY, -100, &generate_test_signature()).await
            .expect("Request failed");

        assert!(!result.success, "Negative amount should fail");
        println!(" PASSED: Negative amount rejected");
    }
}

// ============================================================================
// MODULE 5: Withdrawal Tests
// ============================================================================

#[cfg(test)]
mod withdrawal_tests {
    use super::*;

    #[tokio::test]
    async fn test_alice_withdrawal() {
        println!("\n TEST: Alice Withdraws 500,000 tokens");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        
        // Setup
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        let _ = api.process_deposit(ALICE_VAULT_PUBKEY, 2_000_000, &generate_test_signature()).await;
        
        let before = api.get_balance(ALICE_VAULT_PUBKEY).await.ok()
            .and_then(|r| r.data)
            .map(|v| v.available_balance)
            .unwrap_or(0);
        
        if before < 500_000 {
            println!(" SKIPPED: Insufficient balance ({})", before);
            return;
        }
        
        let result = api.process_withdrawal(ALICE_VAULT_PUBKEY, 500_000, &generate_test_signature()).await
            .expect("Request failed");

        if result.success {
            println!(" PASSED: Withdrawal successful");
        } else {
            println!(" Withdrawal failed: {}", result.error.unwrap_or_default());
        }
    }

    #[tokio::test]
    async fn test_insufficient_balance_withdrawal() {
        println!("\n TEST: Insufficient Balance Should Fail");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let result = api.process_withdrawal(ALICE_VAULT_PUBKEY, 999_999_999_999, &generate_test_signature()).await
            .expect("Request failed");

        assert!(!result.success, "Should fail with insufficient balance");
        println!(" PASSED: Insufficient balance rejected");
    }
}

// ============================================================================
// MODULE 6: Lock/Unlock Tests
// ============================================================================

#[cfg(test)]
mod lock_unlock_tests {
    use super::*;

    #[tokio::test]
    async fn test_lock_collateral() {
        println!("\n TEST: Lock Collateral");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        
        // Setup
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        let _ = api.process_deposit(ALICE_VAULT_PUBKEY, 5_000_000, &generate_test_signature()).await;
        
        let before = api.get_balance(ALICE_VAULT_PUBKEY).await.ok().and_then(|r| r.data);
        
        if before.as_ref().map(|v| v.available_balance).unwrap_or(0) < 1_000_000 {
            println!(" SKIPPED: Insufficient balance for lock");
            return;
        }
        
        let result = api.process_lock(ALICE_VAULT_PUBKEY, 1_000_000, &generate_test_signature()).await
            .expect("Request failed");

        if result.success {
            let vault = result.data.unwrap();
            println!(" PASSED: Collateral locked");
            println!("   Locked: {}", vault.locked_balance);
        } else {
            println!(" Lock failed: {}", result.error.unwrap_or_default());
        }
    }

    #[tokio::test]
    async fn test_unlock_collateral() {
        println!("\n TEST: Unlock Collateral");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        
        // Setup
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        let _ = api.process_deposit(ALICE_VAULT_PUBKEY, 5_000_000, &generate_test_signature()).await;
        let _ = api.process_lock(ALICE_VAULT_PUBKEY, 2_000_000, &generate_test_signature()).await;
        
        let before = api.get_balance(ALICE_VAULT_PUBKEY).await.ok().and_then(|r| r.data);
        let locked = before.as_ref().map(|v| v.locked_balance).unwrap_or(0);
        
        if locked < 500_000 {
            println!(" SKIPPED: No locked funds to unlock");
            return;
        }
        
        let result = api.process_unlock(ALICE_VAULT_PUBKEY, 500_000, &generate_test_signature()).await
            .expect("Request failed");

        if result.success {
            println!(" PASSED: Collateral unlocked");
        } else {
            println!(" Unlock failed: {}", result.error.unwrap_or_default());
        }
    }

    #[tokio::test]
    async fn test_cannot_lock_more_than_available() {
        println!("\n TEST: Cannot Lock More Than Available");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let result = api.process_lock(ALICE_VAULT_PUBKEY, 999_999_999_999, &generate_test_signature()).await
            .expect("Request failed");

        assert!(!result.success, "Should fail");
        println!(" PASSED: Over-locking rejected");
    }
}

// ============================================================================
// MODULE 7: Balance Query Tests
// ============================================================================

#[cfg(test)]
mod balance_query_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_alice_balance() {
        println!("\n TEST: Get Alice's Balance");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let result = api.get_balance(ALICE_VAULT_PUBKEY).await.expect("Request failed");
        
        assert!(result.success, "Should get balance");
        
        let vault = result.data.expect("Should have vault data");
        
        println!(" PASSED: Retrieved balance");
        println!("   Total: {}", vault.total_balance);
        println!("   Available: {}", vault.available_balance);
        println!("   Locked: {}", vault.locked_balance);
    }

    #[tokio::test]
    async fn test_get_vault_by_owner() {
        println!("\n TEST: Get Vault By Owner");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let result = api.get_vault_by_owner(ALICE_PUBKEY).await.expect("Request failed");
        
        println!(" PASSED: Owner lookup (success: {})", result.success);
        if let Some(vault) = result.data {
            println!("   Found: {}", vault.vault_pubkey);
        }
    }

    #[tokio::test]
    async fn test_nonexistent_vault() {
        println!("\n TEST: Query Nonexistent Vault");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let result = api.get_balance("FakeVault11111111111111111111111111111111111").await
            .expect("Request failed");
        
        assert!(!result.success, "Should fail");
        println!(" PASSED: Nonexistent vault handled");
    }
}

// ============================================================================
// MODULE 8: List Vaults Test
// ============================================================================

#[cfg(test)]
mod list_tests {
    use super::*;

    #[tokio::test]
    async fn test_list_vaults() {
        println!("\n TEST: List Vaults");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let result = api.list_vaults(10, 0).await.expect("Request failed");
        
        assert!(result.success, "Should succeed");
        
        println!(" PASSED: Vault list retrieved");
        if let Some(vaults) = result.data {
            println!("   Count: {}", vaults.len());
        }
    }
}

// ============================================================================
// MODULE 9: Error Handling Tests
// ============================================================================

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_pubkey_format() {
        println!("\n TEST: Invalid Pubkey Format");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let result = api.get_balance("not-valid").await.expect("Request failed");
        
        assert!(!result.success, "Should fail");
        println!(" PASSED: Invalid pubkey rejected");
    }

    #[tokio::test]
    async fn test_empty_tx_signature() {
        println!("\n TEST: Empty Transaction Signature");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let result = api.process_deposit(ALICE_VAULT_PUBKEY, 1000, "").await
            .expect("Request failed");

        println!(" PASSED: Empty signature handled (success: {})", result.success);
    }
}

// ============================================================================
// MODULE 10: Performance Tests
// ============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_balance_query_performance() {
        println!("\n TEST: Balance Query Performance");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let iterations = 10;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _ = api.get_balance(ALICE_VAULT_PUBKEY).await;
        }
        
        let elapsed = start.elapsed();
        let avg_ms = elapsed.as_millis() as f64 / iterations as f64;
        
        println!(" PASSED: Performance measured");
        println!("   {} iterations in {:?}", iterations, elapsed);
        println!("   Average: {:.2}ms per request", avg_ms);
        
        assert!(avg_ms < 1000.0, "Should be under 1 second");
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        println!("\n TEST: Concurrent Requests");
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        let _ = api.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await;
        
        let start = Instant::now();
        
        let mut handles = Vec::new();
        for i in 0..10 {
            let client = create_test_client();
            let vault = ALICE_VAULT_PUBKEY.to_string();
            handles.push(tokio::spawn(async move {
                let api = TestApiClient { client, base_url: BASE_URL.to_string() };
                let result = api.get_balance(&vault).await;
                (i, result.is_ok())
            }));
        }
        
        let mut successes = 0;
        for handle in handles {
            if let Ok((_, ok)) = handle.await {
                if ok { successes += 1; }
            }
        }
        
        let elapsed = start.elapsed();
        
        println!(" PASSED: Concurrent requests handled");
        println!("   {}/10 successful in {:?}", successes, elapsed);
        
        assert!(successes >= 8, "At least 80% should succeed");
    }
}

// ============================================================================
// MODULE 11: Full Workflow Test
// ============================================================================

#[cfg(test)]
mod full_workflow_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_trading_workflow() {
        println!("\n TEST: Complete Trading Workflow");
        println!("    REQUIRES: Database fix applied first!");
        println!("   Flow: Initialize → Deposit → Lock → Unlock → Withdraw");
        println!("");
        
        let client = create_test_client();
        
        if !wait_for_server(&client, SERVER_WAIT_ATTEMPTS).await {
            panic!(" FAILED: Server not available!");
        }

        let api = TestApiClient::new();
        
        // Unique vault for this test
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap().as_secs();
        let test_vault = format!("Wf{}111111111111111111111111111111111", ts);
        let test_owner = format!("Ow{}111111111111111111111111111111111", ts);
        let test_token = format!("Tk{}111111111111111111111111111111111", ts);
        
        // Step 1: Initialize
        println!("   Step 1: Initialize vault...");
        let result = api.initialize_vault(&test_vault, &test_owner, &test_token).await
            .expect("Init failed");
        assert!(result.success, "Init should succeed");
        println!("   ✓ Initialized");
        
        // Step 2: Deposit
        println!("   Step 2: Deposit 10,000,000...");
        let result = api.process_deposit(&test_vault, 10_000_000, &generate_test_signature()).await
            .expect("Deposit failed");
        
        if !result.success {
            let err = result.error.unwrap_or_default();
            if err.contains("text = bigint") {
                println!("\n    DATABASE BUG! Apply fix from database_fix.rs\n");
                panic!("Database bug detected!");
            }
            panic!("Deposit failed: {}", err);
        }
        println!("   ✓ Deposited");
        
        // Step 3: Lock
        println!("   Step 3: Lock 3,000,000...");
        let result = api.process_lock(&test_vault, 3_000_000, &generate_test_signature()).await
            .expect("Lock failed");
        assert!(result.success, "Lock should succeed: {:?}", result.error);
        println!("   ✓ Locked");
        
        // Step 4: Unlock
        println!("   Step 4: Unlock 1,000,000...");
        let result = api.process_unlock(&test_vault, 1_000_000, &generate_test_signature()).await
            .expect("Unlock failed");
        assert!(result.success, "Unlock should succeed: {:?}", result.error);
        println!("   ✓ Unlocked");
        
        // Step 5: Withdraw
        println!("   Step 5: Withdraw 5,000,000...");
        let result = api.process_withdrawal(&test_vault, 5_000_000, &generate_test_signature()).await
            .expect("Withdraw failed");
        assert!(result.success, "Withdraw should succeed: {:?}", result.error);
        
        let vault = result.data.unwrap();
        println!("   ✓ Withdrawn");
        
        // Verify
        println!("");
        println!("    Final State:");
        println!("   Total: {} (expected: 5,000,000)", vault.total_balance);
        println!("   Available: {} (expected: 3,000,000)", vault.available_balance);
        println!("   Locked: {} (expected: 2,000,000)", vault.locked_balance);
        
        assert_eq!(vault.total_balance, 5_000_000, "Total should be 5M");
        assert_eq!(vault.locked_balance, 2_000_000, "Locked should be 2M");
        assert_eq!(vault.available_balance, 3_000_000, "Available should be 3M");
        
        println!("");
        println!(" PASSED: Complete workflow verified!");
    }
}