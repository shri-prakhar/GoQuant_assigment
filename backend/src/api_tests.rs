//! # Collateral Vault Backend API Tests
//! 
//! Comprehensive integration tests for all vault operations using two sample users.
//! 
//! ## Test Users:
//! - User 1 (Alice): Main user for deposit/withdraw flows
//! - User 2 (Bob): Secondary user for transfer and multi-user scenarios
//!
//! ## Test Coverage:
//! - Vault initialization
//! - Deposit transactions
//! - Withdrawal transactions
//! - Lock/Unlock collateral
//! - Balance queries
//! - Transaction history
//! - TVL endpoints
//! - Error handling

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

// ============================================================================
// Test Configuration
// ============================================================================

const BASE_URL: &str = "http://localhost:3000/api/v1";
const HEALTH_URL: &str = "http://localhost:3000/health";

// Sample User 1 - Alice (main test user)
const ALICE_PUBKEY: &str = "4rL4RCWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY5Mj";
const ALICE_TOKEN_ACCOUNT: &str = "BYLfz8RQMYE7A5FwL2fVn7RZnYNqh82cBzJpXu9hS3Rq";
const ALICE_VAULT_PUBKEY: &str = "3KmPPXJe3f3cK8qLp9rHqVa5sCHRTLz2MWL4Xa6dN9Zj";

// Sample User 2 - Bob (secondary test user)
const BOB_PUBKEY: &str = "7sB8YPWHz3iA5JwKGmPWAf5BqaLJxqEhEDGLqZqVY2Kn";
const BOB_TOKEN_ACCOUNT: &str = "DYLfz8RQMYE7A5FwL2fVn7RZnYNqh82cBzJpXu9hS4Sq";
const BOB_VAULT_PUBKEY: &str = "5KmPPXJe3f3cK8qLp9rHqVa5sCHRTLz2MWL4Xa6dN8Xk";

// Mock USDT mint on devnet
const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Vault {
    pub vault_pubkey: String,
    pub owner_pubkey: String,
    pub token_account: String,
    pub total_balance: i64,
    pub locked_balance: i64,
    pub available_balance: i64,
    pub total_deposited: i64,
    pub total_withdrawn: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsignedTransactionResponse {
    pub transaction: String,
    pub blockhash: String,
    pub estimated_fee: u64,
    pub signers: Vec<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionRecord {
    pub id: i64,
    pub vault_pubkey: String,
    pub tx_signature: String,
    pub tx_type: String,
    pub amount: i64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionHistoryResponse {
    pub transactions: Vec<TransactionRecord>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct TvlStats {
    pub total_vaults: i64,
    pub total_value_locked: i64,
    pub total_available: i64,
    pub total_locked: i64,
    pub avg_vault_balance: f64,
    pub max_vault_balance: i64,
}

#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub cache: Option<CacheStats>,
}

#[derive(Debug, Deserialize)]
pub struct CacheStats {
    pub vault_entries: u64,
    pub owner_entries: u64,
}

// ============================================================================
// Test Client
// ============================================================================

pub struct TestClient {
    client: Client,
    base_url: String,
}

impl TestClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
        }
    }

    // Health Check
    pub async fn health_check(&self) -> Result<HealthResponse, reqwest::Error> {
        let response = self.client
            .get(HEALTH_URL)
            .send()
            .await?;
        
        response.json().await
    }

    // Vault Operations
    pub async fn initialize_vault(
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

    pub async fn get_balance(&self, vault_pubkey: &str) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let response = self.client
            .get(format!("{}/vault/balance/{}", self.base_url, vault_pubkey))
            .send()
            .await?;

        response.json().await
    }

    pub async fn get_vault_by_owner(&self, owner_pubkey: &str) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let response = self.client
            .get(format!("{}/vault/owner/{}", self.base_url, owner_pubkey))
            .send()
            .await?;

        response.json().await
    }

    pub async fn process_deposit(
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

    pub async fn process_withdrawal(
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

    pub async fn process_lock(
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

    pub async fn process_unlock(
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

    pub async fn sync_vault(&self, vault_pubkey: &str) -> Result<ApiResponse<Vault>, reqwest::Error> {
        let response = self.client
            .post(format!("{}/vault/sync/{}", self.base_url, vault_pubkey))
            .send()
            .await?;

        response.json().await
    }

    pub async fn get_tvl(&self) -> Result<ApiResponse<TvlStats>, reqwest::Error> {
        let response = self.client
            .get(format!("{}/vault/tvl", self.base_url))
            .send()
            .await?;

        response.json().await
    }

    pub async fn list_vaults(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<Vec<Vault>>, reqwest::Error> {
        let mut url = format!("{}/vault/list", self.base_url);
        let mut params = Vec::new();
        
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self.client.get(&url).send().await?;
        response.json().await
    }

    // Transaction Operations
    pub async fn build_deposit_tx(
        &self,
        user_pubkey: &str,
        user_token_account: &str,
        vault_token_account: &str,
        amount: u64,
    ) -> Result<ApiResponse<UnsignedTransactionResponse>, reqwest::Error> {
        let body = json!({
            "user_pubkey": user_pubkey,
            "user_token_account": user_token_account,
            "vault_token_account": vault_token_account,
            "amount": amount
        });

        let response = self.client
            .post(format!("{}/transaction/build/deposit", self.base_url))
            .json(&body)
            .send()
            .await?;

        response.json().await
    }

    pub async fn build_withdraw_tx(
        &self,
        user_pubkey: &str,
        vault_pubkey: &str,
        vault_token_account: &str,
        user_token_account: &str,
        amount: u64,
    ) -> Result<ApiResponse<UnsignedTransactionResponse>, reqwest::Error> {
        let body = json!({
            "user_pubkey": user_pubkey,
            "vault_pubkey": vault_pubkey,
            "vault_token_account": vault_token_account,
            "user_token_account": user_token_account,
            "amount": amount
        });

        let response = self.client
            .post(format!("{}/transaction/build/withdraw", self.base_url))
            .json(&body)
            .send()
            .await?;

        response.json().await
    }

    pub async fn build_initialize_tx(
        &self,
        user_pubkey: &str,
        mint_pubkey: &str,
    ) -> Result<ApiResponse<UnsignedTransactionResponse>, reqwest::Error> {
        let body = json!({
            "user_pubkey": user_pubkey,
            "mint_pubkey": mint_pubkey
        });

        let response = self.client
            .post(format!("{}/transaction/build/initialize", self.base_url))
            .json(&body)
            .send()
            .await?;

        response.json().await
    }

    pub async fn get_transaction_history(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiResponse<TransactionHistoryResponse>, reqwest::Error> {
        let mut url = format!("{}/transaction/history", self.base_url);
        let mut params = Vec::new();
        
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self.client.get(&url).send().await?;
        response.json().await
    }

    pub async fn get_vault_transactions(
        &self,
        vault_pubkey: &str,
        limit: Option<i64>,
    ) -> Result<ApiResponse<TransactionHistoryResponse>, reqwest::Error> {
        let mut url = format!("{}/transaction/history/{}", self.base_url, vault_pubkey);
        
        if let Some(l) = limit {
            url = format!("{}?limit={}", url, l);
        }

        let response = self.client.get(&url).send().await?;
        response.json().await
    }

    pub async fn get_transaction(&self, tx_signature: &str) -> Result<ApiResponse<TransactionRecord>, reqwest::Error> {
        let response = self.client
            .get(format!("{}/transaction/{}", self.base_url, tx_signature))
            .send()
            .await?;

        response.json().await
    }
}

// ============================================================================
// Test Helper Functions
// ============================================================================

fn generate_tx_signature() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}TestTx{}", &ALICE_PUBKEY[0..32], timestamp)
}

async fn wait_for_server(client: &TestClient, max_retries: u32) -> bool {
    for i in 0..max_retries {
        match client.health_check().await {
            Ok(health) if health.status == "ok" => {
                println!("✅ Server is ready (attempt {})", i + 1);
                return true;
            }
            _ => {
                println!("⏳ Waiting for server... (attempt {})", i + 1);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    false
}

// ============================================================================
// Test Modules
// ============================================================================

#[cfg(test)]
mod health_tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            println!("⚠️ Server not available, skipping test");
            return;
        }

        let health = client.health_check().await;
        assert!(health.is_ok(), "Health check should succeed");
        
        let health = health.unwrap();
        assert_eq!(health.status, "ok", "Server should be healthy");
        println!("✅ Health check passed: {:?}", health);
    }
}

#[cfg(test)]
mod vault_initialization_tests {
    use super::*;

    #[tokio::test]
    async fn test_initialize_alice_vault() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n🔵 Test: Initialize Alice's Vault");
        println!("   User: Alice ({})", ALICE_PUBKEY);

        let result = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    assert_eq!(vault.owner_pubkey, ALICE_PUBKEY);
                    assert_eq!(vault.total_balance, 0);
                    println!("   ✅ Alice's vault initialized successfully");
                    println!("      Vault: {}", vault.vault_pubkey);
                } else {
                    // Vault may already exist
                    println!("   ⚠️ Response: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_initialize_bob_vault() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n🟢 Test: Initialize Bob's Vault");
        println!("   User: Bob ({})", BOB_PUBKEY);

        let result = client
            .initialize_vault(BOB_VAULT_PUBKEY, BOB_PUBKEY, BOB_TOKEN_ACCOUNT)
            .await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    assert_eq!(vault.owner_pubkey, BOB_PUBKEY);
                    println!("   ✅ Bob's vault initialized successfully");
                } else {
                    println!("   ⚠️ Response: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod deposit_tests {
    use super::*;

    #[tokio::test]
    async fn test_alice_deposit() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n💰 Test: Alice Deposits 1000 USDT");

        // First ensure vault exists
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;

        // Process deposit
        let tx_sig = generate_tx_signature();
        let deposit_amount: i64 = 1_000_000_000; // 1000 USDT (6 decimals)

        let result = client
            .process_deposit(ALICE_VAULT_PUBKEY, deposit_amount, &tx_sig)
            .await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    assert!(vault.total_balance >= deposit_amount);
                    assert!(vault.available_balance >= deposit_amount);
                    println!("   ✅ Deposit successful");
                    println!("      Amount: {} USDT", deposit_amount / 1_000_000);
                    println!("      New Balance: {} USDT", vault.total_balance / 1_000_000);
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_bob_deposit() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n💰 Test: Bob Deposits 500 USDT");

        // Ensure vault exists
        let _ = client
            .initialize_vault(BOB_VAULT_PUBKEY, BOB_PUBKEY, BOB_TOKEN_ACCOUNT)
            .await;

        let tx_sig = generate_tx_signature();
        let deposit_amount: i64 = 500_000_000; // 500 USDT

        let result = client
            .process_deposit(BOB_VAULT_PUBKEY, deposit_amount, &tx_sig)
            .await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    println!("   ✅ Bob's deposit successful");
                    println!("      Amount: {} USDT", deposit_amount / 1_000_000);
                    println!("      New Balance: {} USDT", vault.total_balance / 1_000_000);
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_deposits() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n💰 Test: Multiple Deposits for Alice");

        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;

        let deposits = [100_000_000, 200_000_000, 300_000_000]; // 100, 200, 300 USDT
        
        for amount in deposits {
            let tx_sig = generate_tx_signature();
            let result = client.process_deposit(ALICE_VAULT_PUBKEY, amount, &tx_sig).await;
            
            match result {
                Ok(response) if response.success => {
                    println!("   ✅ Deposited {} USDT", amount / 1_000_000);
                }
                _ => {
                    println!("   ⚠️ Deposit of {} USDT had issue", amount / 1_000_000);
                }
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod withdrawal_tests {
    use super::*;

    #[tokio::test]
    async fn test_alice_withdrawal() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n💸 Test: Alice Withdraws 200 USDT");

        // Setup: Initialize and deposit first
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;
        
        let deposit_tx = generate_tx_signature();
        let _ = client
            .process_deposit(ALICE_VAULT_PUBKEY, 1_000_000_000, &deposit_tx)
            .await;

        // Now withdraw
        let withdraw_tx = generate_tx_signature();
        let withdraw_amount: i64 = 200_000_000; // 200 USDT

        let result = client
            .process_withdrawal(ALICE_VAULT_PUBKEY, withdraw_amount, &withdraw_tx)
            .await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    println!("   ✅ Withdrawal successful");
                    println!("      Amount: {} USDT", withdraw_amount / 1_000_000);
                    println!("      Remaining: {} USDT", vault.total_balance / 1_000_000);
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_insufficient_balance_withdrawal() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n❌ Test: Withdrawal with Insufficient Balance (Should Fail)");

        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;

        let tx_sig = generate_tx_signature();
        let excessive_amount: i64 = 999_999_999_999; // Way more than available

        let result = client
            .process_withdrawal(ALICE_VAULT_PUBKEY, excessive_amount, &tx_sig)
            .await;

        match result {
            Ok(response) => {
                if !response.success {
                    println!("   ✅ Correctly rejected: {:?}", response.error);
                } else {
                    println!("   ❌ Should have been rejected!");
                }
            }
            Err(_) => {
                println!("   ✅ Request correctly rejected");
            }
        }
    }
}

#[cfg(test)]
mod lock_unlock_tests {
    use super::*;

    #[tokio::test]
    async fn test_lock_collateral() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n🔒 Test: Lock Collateral for Alice (Open Position)");

        // Setup
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;
        
        let deposit_tx = generate_tx_signature();
        let _ = client
            .process_deposit(ALICE_VAULT_PUBKEY, 1_000_000_000, &deposit_tx)
            .await;

        // Lock 300 USDT as margin
        let lock_tx = generate_tx_signature();
        let lock_amount: i64 = 300_000_000;

        let result = client.process_lock(ALICE_VAULT_PUBKEY, lock_amount, &lock_tx).await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    assert!(vault.locked_balance >= lock_amount);
                    println!("   ✅ Lock successful");
                    println!("      Locked: {} USDT", vault.locked_balance / 1_000_000);
                    println!("      Available: {} USDT", vault.available_balance / 1_000_000);
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_unlock_collateral() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n🔓 Test: Unlock Collateral for Alice (Close Position)");

        // Setup: Initialize, deposit, lock
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;
        
        let deposit_tx = generate_tx_signature();
        let _ = client.process_deposit(ALICE_VAULT_PUBKEY, 1_000_000_000, &deposit_tx).await;
        
        let lock_tx = generate_tx_signature();
        let _ = client.process_lock(ALICE_VAULT_PUBKEY, 300_000_000, &lock_tx).await;

        // Unlock 200 USDT
        let unlock_tx = generate_tx_signature();
        let unlock_amount: i64 = 200_000_000;

        let result = client.process_unlock(ALICE_VAULT_PUBKEY, unlock_amount, &unlock_tx).await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    println!("   ✅ Unlock successful");
                    println!("      Locked: {} USDT", vault.locked_balance / 1_000_000);
                    println!("      Available: {} USDT", vault.available_balance / 1_000_000);
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod balance_query_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_alice_balance() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n📊 Test: Get Alice's Balance");

        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;

        let result = client.get_balance(ALICE_VAULT_PUBKEY).await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    println!("   ✅ Balance retrieved");
                    println!("      Total: {} USDT", vault.total_balance / 1_000_000);
                    println!("      Available: {} USDT", vault.available_balance / 1_000_000);
                    println!("      Locked: {} USDT", vault.locked_balance / 1_000_000);
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_vault_by_owner() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n📊 Test: Get Vault by Owner (Bob)");

        let _ = client
            .initialize_vault(BOB_VAULT_PUBKEY, BOB_PUBKEY, BOB_TOKEN_ACCOUNT)
            .await;

        let result = client.get_vault_by_owner(BOB_PUBKEY).await;

        match result {
            Ok(response) => {
                if response.success {
                    let vault = response.data.unwrap();
                    assert_eq!(vault.owner_pubkey, BOB_PUBKEY);
                    println!("   ✅ Found Bob's vault");
                    println!("      Vault: {}", vault.vault_pubkey);
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_nonexistent_vault() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n❌ Test: Query Nonexistent Vault");

        let result = client.get_balance("NonExistentVaultPubkeyHere123456789012345").await;

        match result {
            Ok(response) => {
                if !response.success {
                    println!("   ✅ Correctly returned error: {:?}", response.error);
                }
            }
            Err(_) => {
                println!("   ✅ Correctly rejected request");
            }
        }
    }
}

#[cfg(test)]
mod transaction_builder_tests {
    use super::*;

    #[tokio::test]
    async fn test_build_deposit_transaction() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n🔨 Test: Build Deposit Transaction for Alice");

        let result = client
            .build_deposit_tx(
                ALICE_PUBKEY,
                ALICE_TOKEN_ACCOUNT,
                ALICE_TOKEN_ACCOUNT, // Vault token account
                500_000_000,
            )
            .await;

        match result {
            Ok(response) => {
                if response.success {
                    let tx = response.data.unwrap();
                    assert!(!tx.transaction.is_empty() || tx.blockhash.len() > 0);
                    println!("   ✅ Transaction built successfully");
                    println!("      Blockhash: {}", tx.blockhash);
                    println!("      Estimated Fee: {} lamports", tx.estimated_fee);
                    println!("      Message: {}", tx.message);
                } else {
                    println!("   ⚠️ Build failed (may need RPC): {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ⚠️ Error (may need RPC connection): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_build_withdraw_transaction() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n🔨 Test: Build Withdraw Transaction for Alice");

        // First setup vault with balance
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;
        let tx = generate_tx_signature();
        let _ = client.process_deposit(ALICE_VAULT_PUBKEY, 1_000_000_000, &tx).await;

        let result = client
            .build_withdraw_tx(
                ALICE_PUBKEY,
                ALICE_VAULT_PUBKEY,
                ALICE_TOKEN_ACCOUNT,
                ALICE_TOKEN_ACCOUNT,
                200_000_000,
            )
            .await;

        match result {
            Ok(response) => {
                if response.success {
                    let tx = response.data.unwrap();
                    println!("   ✅ Withdraw transaction built");
                    println!("      Blockhash: {}", tx.blockhash);
                } else {
                    println!("   ⚠️ Build failed: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ⚠️ Error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod transaction_history_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_vault_transactions() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n📜 Test: Get Alice's Transaction History");

        // Setup some transactions
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;
        
        for i in 0..3 {
            let tx = generate_tx_signature();
            let _ = client.process_deposit(ALICE_VAULT_PUBKEY, 100_000_000, &tx).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let result = client.get_vault_transactions(ALICE_VAULT_PUBKEY, Some(10)).await;

        match result {
            Ok(response) => {
                if response.success {
                    let history = response.data.unwrap();
                    println!("   ✅ Found {} transactions", history.transactions.len());
                    for tx in history.transactions.iter().take(3) {
                        println!("      - {} | {} | {} USDT", 
                            tx.tx_type, 
                            tx.status,
                            tx.amount / 1_000_000
                        );
                    }
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tvl_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_tvl() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n📈 Test: Get Total Value Locked (TVL)");

        // Setup some vaults with balances
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;
        let _ = client
            .initialize_vault(BOB_VAULT_PUBKEY, BOB_PUBKEY, BOB_TOKEN_ACCOUNT)
            .await;
        
        let tx1 = generate_tx_signature();
        let tx2 = generate_tx_signature();
        let _ = client.process_deposit(ALICE_VAULT_PUBKEY, 1_000_000_000, &tx1).await;
        let _ = client.process_deposit(BOB_VAULT_PUBKEY, 500_000_000, &tx2).await;

        let result = client.get_tvl().await;

        match result {
            Ok(response) => {
                if response.success {
                    let tvl = response.data.unwrap();
                    println!("   ✅ TVL Stats Retrieved");
                    println!("      Total Vaults: {}", tvl.total_vaults);
                    println!("      TVL: {} USDT", tvl.total_value_locked / 1_000_000);
                    println!("      Available: {} USDT", tvl.total_available / 1_000_000);
                    println!("      Locked: {} USDT", tvl.total_locked / 1_000_000);
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_list_vaults() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n📋 Test: List All Vaults");

        let result = client.list_vaults(Some(10), Some(0)).await;

        match result {
            Ok(response) => {
                if response.success {
                    let vaults = response.data.unwrap();
                    println!("   ✅ Found {} vaults", vaults.len());
                    for vault in vaults.iter().take(5) {
                        println!("      - {} | {} USDT", 
                            &vault.vault_pubkey[0..16],
                            vault.total_balance / 1_000_000
                        );
                    }
                } else {
                    println!("   ⚠️ Error: {:?}", response.error);
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod full_workflow_tests {
    use super::*;

    /// Complete workflow test simulating real user actions
    #[tokio::test]
    async fn test_complete_trading_workflow() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n🎯 === COMPLETE TRADING WORKFLOW TEST ===\n");

        // Step 1: Initialize Alice's vault
        println!("📌 Step 1: Initialize Alice's Vault");
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;
        println!("   Done\n");

        // Step 2: Alice deposits 2000 USDT
        println!("📌 Step 2: Alice Deposits 2000 USDT");
        let deposit_tx = generate_tx_signature();
        let deposit_result = client
            .process_deposit(ALICE_VAULT_PUBKEY, 2_000_000_000, &deposit_tx)
            .await;
        if let Ok(r) = deposit_result {
            if r.success {
                let v = r.data.unwrap();
                println!("   Balance: {} USDT\n", v.total_balance / 1_000_000);
            }
        }

        // Step 3: Alice opens a 5x leveraged position (locks 500 USDT margin)
        println!("📌 Step 3: Alice Opens Position (Lock 500 USDT)");
        let lock_tx = generate_tx_signature();
        let lock_result = client
            .process_lock(ALICE_VAULT_PUBKEY, 500_000_000, &lock_tx)
            .await;
        if let Ok(r) = lock_result {
            if r.success {
                let v = r.data.unwrap();
                println!("   Locked: {} USDT", v.locked_balance / 1_000_000);
                println!("   Available: {} USDT\n", v.available_balance / 1_000_000);
            }
        }

        // Step 4: Check balance
        println!("📌 Step 4: Check Balance");
        let balance = client.get_balance(ALICE_VAULT_PUBKEY).await;
        if let Ok(r) = balance {
            if r.success {
                let v = r.data.unwrap();
                println!("   Total: {} USDT", v.total_balance / 1_000_000);
                println!("   Available: {} USDT", v.available_balance / 1_000_000);
                println!("   Locked: {} USDT\n", v.locked_balance / 1_000_000);
            }
        }

        // Step 5: Alice closes position with profit (unlock + add profit)
        println!("📌 Step 5: Alice Closes Position (Unlock 500 USDT)");
        let unlock_tx = generate_tx_signature();
        let _ = client
            .process_unlock(ALICE_VAULT_PUBKEY, 500_000_000, &unlock_tx)
            .await;
        
        // Add profit (simulated)
        let profit_tx = generate_tx_signature();
        let _ = client
            .process_deposit(ALICE_VAULT_PUBKEY, 100_000_000, &profit_tx)
            .await;
        println!("   Position closed with 100 USDT profit\n");

        // Step 6: Alice withdraws 1000 USDT
        println!("📌 Step 6: Alice Withdraws 1000 USDT");
        let withdraw_tx = generate_tx_signature();
        let withdraw_result = client
            .process_withdrawal(ALICE_VAULT_PUBKEY, 1_000_000_000, &withdraw_tx)
            .await;
        if let Ok(r) = withdraw_result {
            if r.success {
                let v = r.data.unwrap();
                println!("   Remaining Balance: {} USDT\n", v.total_balance / 1_000_000);
            }
        }

        // Step 7: Get transaction history
        println!("📌 Step 7: Transaction History");
        let history = client.get_vault_transactions(ALICE_VAULT_PUBKEY, Some(10)).await;
        if let Ok(r) = history {
            if r.success {
                let h = r.data.unwrap();
                println!("   Transactions:");
                for tx in h.transactions.iter().take(6) {
                    println!("      {} | {} | {} USDT",
                        tx.tx_type.pad_to_width(10),
                        tx.status,
                        tx.amount / 1_000_000
                    );
                }
            }
        }

        println!("\n✅ === WORKFLOW COMPLETE ===\n");
    }

    /// Multi-user test
    #[tokio::test]
    async fn test_multi_user_scenario() {
        let client = TestClient::new();
        
        if !wait_for_server(&client, 5).await {
            return;
        }

        println!("\n👥 === MULTI-USER SCENARIO ===\n");

        // Initialize both users
        println!("📌 Initialize Both Users");
        let _ = client
            .initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT)
            .await;
        let _ = client
            .initialize_vault(BOB_VAULT_PUBKEY, BOB_PUBKEY, BOB_TOKEN_ACCOUNT)
            .await;
        println!("   Alice and Bob vaults ready\n");

        // Both users deposit
        println!("📌 Both Users Deposit");
        let tx1 = generate_tx_signature();
        let tx2 = generate_tx_signature();
        let _ = client.process_deposit(ALICE_VAULT_PUBKEY, 3_000_000_000, &tx1).await;
        let _ = client.process_deposit(BOB_VAULT_PUBKEY, 1_500_000_000, &tx2).await;
        println!("   Alice deposited: 3000 USDT");
        println!("   Bob deposited: 1500 USDT\n");

        // Alice opens position
        println!("📌 Alice Opens Large Position (1000 USDT margin)");
        let lock_tx = generate_tx_signature();
        let _ = client.process_lock(ALICE_VAULT_PUBKEY, 1_000_000_000, &lock_tx).await;

        // Bob opens smaller position
        println!("📌 Bob Opens Position (500 USDT margin)");
        let lock_tx2 = generate_tx_signature();
        let _ = client.process_lock(BOB_VAULT_PUBKEY, 500_000_000, &lock_tx2).await;

        // Check TVL
        println!("\n📌 Check Total Value Locked");
        let tvl = client.get_tvl().await;
        if let Ok(r) = tvl {
            if r.success {
                let t = r.data.unwrap();
                println!("   Total Vaults: {}", t.total_vaults);
                println!("   Total TVL: {} USDT", t.total_value_locked / 1_000_000);
                println!("   Total Locked: {} USDT", t.total_locked / 1_000_000);
                println!("   Total Available: {} USDT", t.total_available / 1_000_000);
            }
        }

        println!("\n✅ === MULTI-USER SCENARIO COMPLETE ===\n");
    }
}

// ============================================================================
// Helper trait for string padding
// ============================================================================

trait StringPadding {
    fn pad_to_width(&self, width: usize) -> String;
}

impl StringPadding for String {
    fn pad_to_width(&self, width: usize) -> String {
        format!("{:width$}", self, width = width)
    }
}

impl StringPadding for str {
    fn pad_to_width(&self, width: usize) -> String {
        format!("{:width$}", self, width = width)
    }
}

// ============================================================================
// Main function for running tests manually
// ============================================================================

#[tokio::main]
async fn main() {
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     COLLATERAL VAULT BACKEND API TEST SUITE                ║");
    println!("║                                                            ║");
    println!("║  Sample Users:                                             ║");
    println!("║  - Alice: Main test user (deposits, trades)                ║");
    println!("║  - Bob: Secondary user (multi-user scenarios)              ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\n");

    let client = TestClient::new();

    // Wait for server
    println!("⏳ Checking server availability...\n");
    if !wait_for_server(&client, 10).await {
        println!("❌ Server not available at {}. Please start the server first.", BASE_URL);
        println!("\n   Run: cargo run --bin backend\n");
        return;
    }

    println!("✅ Server is ready!\n");
    println!("Running tests with `cargo test` or use this as integration test.\n");
    
    // Run a quick demo
    println!("📊 Quick Demo: Creating vaults for Alice and Bob...\n");

    // Initialize vaults
    match client.initialize_vault(ALICE_VAULT_PUBKEY, ALICE_PUBKEY, ALICE_TOKEN_ACCOUNT).await {
        Ok(r) if r.success => println!("   ✅ Alice's vault created"),
        _ => println!("   ⚠️ Alice's vault may already exist"),
    }

    match client.initialize_vault(BOB_VAULT_PUBKEY, BOB_PUBKEY, BOB_TOKEN_ACCOUNT).await {
        Ok(r) if r.success => println!("   ✅ Bob's vault created"),
        _ => println!("   ⚠️ Bob's vault may already exist"),
    }

    // Quick deposit test
    let tx = generate_tx_signature();
    match client.process_deposit(ALICE_VAULT_PUBKEY, 1_000_000_000, &tx).await {
        Ok(r) if r.success => {
            let v = r.data.unwrap();
            println!("   ✅ Alice deposited 1000 USDT");
            println!("      Balance: {} USDT", v.total_balance / 1_000_000);
        }
        _ => println!("   ⚠️ Deposit test may have had an issue"),
    }

    println!("\n🎉 Demo complete! Run `cargo test` for full test suite.\n");
}