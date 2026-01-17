//! # Solana Event Listener Service
//! 
//! This service listens for on-chain events from the Collateral Vault Anchor program.
//! 
//! ## Architecture Flow (Points 9-13):
//! 9. Event Listener catches the event via WebSocket/polling
//! 10. Event Listener updates database with on-chain values
//! 11. Event Listener invalidates cache for affected vaults
//! 12. Event Listener broadcasts update via WebSocket
//! 13. Frontend receives real-time update, refreshes UI
//!
//! ## Event Types Monitored:
//! - DepositEvent
//! - WithdrawEvent  
//! - LockEvent
//! - UnlockEvent
//! - TransferEvent

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use actix_web::web::Data;
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use tokio::time;

use crate::services::AppState;
use crate::websocket::{
    broadcast_balance_update, broadcast_deposit, broadcast_lock, 
    broadcast_unlock, broadcast_withdrawal, broadcast_tvl_update,
};

// ============================================================================
// Event Structures (using raw bytes to avoid borsh version conflicts)
// ============================================================================

/// Helper to convert [u8; 32] to Pubkey string
fn pubkey_to_string(bytes: &[u8; 32]) -> String {
    Pubkey::from(*bytes).to_string()
}

/// Deposit event emitted by the on-chain program
#[derive(Debug, Clone, BorshDeserialize)]
pub struct DepositEvent {
    pub user: [u8; 32],
    pub vault: [u8; 32],
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

impl DepositEvent {
    pub fn user_pubkey(&self) -> String {
        pubkey_to_string(&self.user)
    }
    pub fn vault_pubkey(&self) -> String {
        pubkey_to_string(&self.vault)
    }
}

/// Withdrawal event emitted by the on-chain program
#[derive(Debug, Clone, BorshDeserialize)]
pub struct WithdrawEvent {
    pub user: [u8; 32],
    pub vault: [u8; 32],
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

impl WithdrawEvent {
    pub fn user_pubkey(&self) -> String {
        pubkey_to_string(&self.user)
    }
    pub fn vault_pubkey(&self) -> String {
        pubkey_to_string(&self.vault)
    }
}

/// Lock collateral event
#[derive(Debug, Clone, BorshDeserialize)]
pub struct LockEvent {
    pub vault: [u8; 32],
    pub amount: u64,
    pub new_locked: u64,
    pub new_available: u64,
    pub timestamp: i64,
}

impl LockEvent {
    pub fn vault_pubkey(&self) -> String {
        pubkey_to_string(&self.vault)
    }
}

/// Unlock collateral event
#[derive(Debug, Clone, BorshDeserialize)]
pub struct UnlockEvent {
    pub vault: [u8; 32],
    pub amount: u64,
    pub new_locked: u64,
    pub new_available: u64,
    pub timestamp: i64,
}

impl UnlockEvent {
    pub fn vault_pubkey(&self) -> String {
        pubkey_to_string(&self.vault)
    }
}

/// Transfer event between vaults
#[derive(Debug, Clone, BorshDeserialize)]
pub struct TransferEvent {
    pub from_vault: [u8; 32],
    pub to_vault: [u8; 32],
    pub amount: u64,
    pub timestamp: i64,
}

impl TransferEvent {
    pub fn from_vault_pubkey(&self) -> String {
        pubkey_to_string(&self.from_vault)
    }
    pub fn to_vault_pubkey(&self) -> String {
        pubkey_to_string(&self.to_vault)
    }
}

/// Vault initialized event
#[derive(Debug, Clone, BorshDeserialize)]
pub struct VaultInitializedEvent {
    pub owner: [u8; 32],
    pub vault: [u8; 32],
    pub token_account: [u8; 32],
    pub timestamp: i64,
}

impl VaultInitializedEvent {
    pub fn owner_pubkey(&self) -> String {
        pubkey_to_string(&self.owner)
    }
    pub fn vault_pubkey(&self) -> String {
        pubkey_to_string(&self.vault)
    }
    pub fn token_account_pubkey(&self) -> String {
        pubkey_to_string(&self.token_account)
    }
}

/// All possible vault events
#[derive(Debug, Clone)]
pub enum VaultEvent {
    Deposit(DepositEvent),
    Withdraw(WithdrawEvent),
    Lock(LockEvent),
    Unlock(UnlockEvent),
    Transfer(TransferEvent),
    VaultInitialized(VaultInitializedEvent),
}

// ============================================================================
// Event Listener Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct EventListenerConfig {
    /// How often to poll for new transactions (in milliseconds)
    pub poll_interval_ms: u64,
    /// Number of recent slots to check for logs
    pub slots_to_check: u64,
    /// Whether to use WebSocket subscription (if available) or polling
    pub use_websocket: bool,
    /// Maximum retries for failed event processing
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for EventListenerConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 2000,  // Poll every 2 seconds (reduced frequency)
            slots_to_check: 100,     // Check last 100 slots
            use_websocket: false,    // Use polling by default (more reliable)
            max_retries: 3,
            retry_delay_ms: 500,
        }
    }
}

// ============================================================================
// Event Listener Service
// ============================================================================

pub struct EventListener {
    state: Data<AppState>,
    config: EventListenerConfig,
    processed_signatures: HashMap<String, i64>, // signature -> timestamp
}

impl EventListener {
    pub fn new(state: Data<AppState>, config: EventListenerConfig) -> Self {
        Self {
            state,
            config,
            processed_signatures: HashMap::new(),
        }
    }

    /// Start the event listener service
    pub async fn start(&mut self) {
        // Log immediately on start - BEFORE any async operations
        tracing::info!(
            "🎧 Event Listener starting (poll_interval: {}ms, program_id: {})",
            self.config.poll_interval_ms,
            self.state.program_id
        );

        // Test RPC connection first
        match self.test_rpc_connection().await {
            Ok(_) => {
                tracing::info!("✅ Event Listener RPC connection verified");
            }
            Err(e) => {
                tracing::error!("❌ Event Listener RPC connection failed: {}. Will retry...", e);
                // Don't exit - continue anyway, polling loop will handle retries
            }
        }

        tracing::info!("📡 Event Listener entering polling mode");
        self.run_polling_loop().await;
    }

    /// Test RPC connection before starting the main loop
    async fn test_rpc_connection(&self) -> Result<(), String> {
        // Try to get the current slot as a simple health check
        self.state.solana_client
            .get_slot()
            .await
            .map(|slot| {
                tracing::debug!("RPC health check: current slot = {}", slot);
            })
            .map_err(|e| e.to_string())
    }

    /// Main polling loop to fetch and process program logs
    async fn run_polling_loop(&mut self) {
        let mut interval = time::interval(Duration::from_millis(self.config.poll_interval_ms));
        let mut consecutive_errors = 0u32;
        let max_consecutive_errors = 10;

        loop {
            interval.tick().await;

            match self.poll_and_process_events().await {
                Ok(events_processed) => {
                    consecutive_errors = 0; // Reset error counter on success
                    if events_processed > 0 {
                        tracing::info!("📬 Processed {} events this cycle", events_processed);
                    } else {
                        tracing::trace!("No new events this cycle");
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;
                    tracing::error!(
                        "Event polling error (attempt {}/{}): {}", 
                        consecutive_errors, 
                        max_consecutive_errors,
                        e
                    );

                    if consecutive_errors >= max_consecutive_errors {
                        tracing::error!(
                            "❌ Too many consecutive errors ({}), backing off for 30 seconds",
                            consecutive_errors
                        );
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        consecutive_errors = 0; // Reset after backoff
                    }
                }
            }
        }
    }

    /// Poll for new program logs and process events
    /// Returns the number of events processed
    async fn poll_and_process_events(&mut self) -> Result<usize, EventListenerError> {
        let program_id = self.state.program_id;

        // Get recent signatures for the program
        let signatures = match self.state.solana_client
            .get_signatures_for_address(&program_id)
            .await 
        {
            Ok(sigs) => sigs,
            Err(e) => {
                // Check if it's just "no signatures found" (not an error)
                let err_str = e.to_string();
                if err_str.contains("AccountNotFound") || err_str.contains("not found") {
                    tracing::trace!("No signatures found for program {} (this is normal for new programs)", program_id);
                    return Ok(0);
                }
                return Err(EventListenerError::RpcError(err_str));
            }
        };

        let mut new_events = Vec::new();
        let mut processed_count = 0;

        for sig_info in signatures.iter().take(50) {  // Process last 50 transactions
            let signature_str = sig_info.signature.clone();

            // Skip if already processed
            if self.processed_signatures.contains_key(&signature_str) {
                continue;
            }

            // Skip failed transactions
            if sig_info.err.is_some() {
                self.processed_signatures.insert(signature_str.clone(), chrono::Utc::now().timestamp());
                continue;
            }

            // Parse the signature
            let signature = match Signature::from_str(&signature_str) {
                Ok(sig) => sig,
                Err(e) => {
                    tracing::warn!("Failed to parse signature {}: {}", signature_str, e);
                    self.processed_signatures.insert(signature_str, chrono::Utc::now().timestamp());
                    continue;
                }
            };

            // Fetch transaction details
            match self.fetch_and_parse_transaction(&signature).await {
                Ok(Some(events)) => {
                    new_events.extend(events);
                }
                Ok(None) => {
                    // No events in this transaction - that's fine
                }
                Err(e) => {
                    tracing::warn!("Failed to parse transaction {}: {}", signature_str, e);
                }
            }

            // Mark as processed
            self.processed_signatures.insert(signature_str, chrono::Utc::now().timestamp());
        }

        // Process all new events
        for (event, tx_signature) in new_events {
            match self.process_event(event.clone(), &tx_signature).await {
                Ok(_) => {
                    processed_count += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to process event {:?}: {}", event, e);
                }
            }
        }

        // Cleanup old processed signatures (keep last hour)
        let cutoff = chrono::Utc::now().timestamp() - 3600;
        self.processed_signatures.retain(|_, ts| *ts > cutoff);

        Ok(processed_count)
    }

    /// Fetch and parse a transaction for events
    async fn fetch_and_parse_transaction(
        &self,
        signature: &Signature,
    ) -> Result<Option<Vec<(VaultEvent, String)>>, EventListenerError> {
        let tx = self.state.solana_client
            .get_transaction(
                signature,
                solana_transaction_status::UiTransactionEncoding::Json,
            ).await
            .map_err(|e| EventListenerError::RpcError(e.to_string()))?;

        let signature_str = signature.to_string();
        let mut events = Vec::new();

        // Parse transaction logs for events
        if let Some(meta) = tx.transaction.meta {
            if let Some(log_messages) = meta.log_messages.as_ref().map(|v| v as &Vec<String>) {
                for log in log_messages {
                    // Anchor events are prefixed with "Program data: "
                    if log.starts_with("Program data: ") {
                        let data = log.trim_start_matches("Program data: ");
                        
                        if let Ok(decoded) = bs58::decode(data).into_vec() {
                            if let Some(event) = self.parse_event_data(&decoded) {
                                events.push((event, signature_str.clone()));
                            }
                        }
                    }
                }
            }
        }

        if events.is_empty() {
            Ok(None)
        } else {
            Ok(Some(events))
        }
    }

    /// Parse raw event data into a VaultEvent
    fn parse_event_data(&self, data: &[u8]) -> Option<VaultEvent> {
        if data.len() < 8 {
            return None;
        }

        // Skip the 8-byte discriminator
        let event_data = &data[8..];

        // Try parsing each event type
        // Note: In production, you should check discriminators first
        
        if let Ok(event) = DepositEvent::try_from_slice(event_data) {
            return Some(VaultEvent::Deposit(event));
        }

        if let Ok(event) = WithdrawEvent::try_from_slice(event_data) {
            return Some(VaultEvent::Withdraw(event));
        }

        if let Ok(event) = LockEvent::try_from_slice(event_data) {
            return Some(VaultEvent::Lock(event));
        }

        if let Ok(event) = UnlockEvent::try_from_slice(event_data) {
            return Some(VaultEvent::Unlock(event));
        }

        if let Ok(event) = TransferEvent::try_from_slice(event_data) {
            return Some(VaultEvent::Transfer(event));
        }

        if let Ok(event) = VaultInitializedEvent::try_from_slice(event_data) {
            return Some(VaultEvent::VaultInitialized(event));
        }

        None
    }

    /// Process a parsed event - update database, cache, and broadcast
    async fn process_event(
        &self,
        event: VaultEvent,
        tx_signature: &str,
    ) -> Result<(), EventListenerError> {
        tracing::info!("📨 Processing event: {:?}", event);

        match event {
            VaultEvent::Deposit(e) => {
                self.handle_deposit_event(e, tx_signature).await?;
            }
            VaultEvent::Withdraw(e) => {
                self.handle_withdraw_event(e, tx_signature).await?;
            }
            VaultEvent::Lock(e) => {
                self.handle_lock_event(e, tx_signature).await?;
            }
            VaultEvent::Unlock(e) => {
                self.handle_unlock_event(e, tx_signature).await?;
            }
            VaultEvent::Transfer(e) => {
                self.handle_transfer_event(e, tx_signature).await?;
            }
            VaultEvent::VaultInitialized(e) => {
                self.handle_vault_initialized_event(e, tx_signature).await?;
            }
        }

        Ok(())
    }

    /// Handle deposit event
    async fn handle_deposit_event(
        &self,
        event: DepositEvent,
        tx_signature: &str,
    ) -> Result<(), EventListenerError> {
        let vault_pubkey = event.vault_pubkey();
        let amount = event.amount as i64;
        let new_balance = event.new_balance as i64;

        tracing::info!(
            "💰 Deposit event: vault={}, amount={}, new_balance={}",
            vault_pubkey, amount, new_balance
        );

        // Update database with on-chain values
        self.state.database
            .update_vault_balances(
                &vault_pubkey,
                new_balance,
                0,  // Locked balance unchanged for deposits
                Some(amount), // Add to total deposited
                None,
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Record the transaction
        self.state.database
            .record_transaction(
                &vault_pubkey,
                tx_signature,
                "deposit",
                amount,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Invalidate cache for affected vault
        self.state.cache.invalidate_vault(&vault_pubkey).await;

        // Broadcast update via WebSocket
        broadcast_deposit(
            &vault_pubkey,
            amount,
            tx_signature,
            new_balance,
        ).await;

        // Also broadcast balance update
        let vault = self.state.database
            .get_vault(&vault_pubkey)
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        if let Some(v) = vault {
            broadcast_balance_update(
                &vault_pubkey,
                v.total_balance,
                v.available_balance,
                v.locked_balance,
            ).await;
        }

        // Update TVL
        self.update_tvl().await?;

        tracing::info!("✅ Deposit event processed successfully");
        Ok(())
    }

    /// Handle withdrawal event
    async fn handle_withdraw_event(
        &self,
        event: WithdrawEvent,
        tx_signature: &str,
    ) -> Result<(), EventListenerError> {
        let vault_pubkey = event.vault_pubkey();
        let amount = event.amount as i64;
        let new_balance = event.new_balance as i64;

        tracing::info!(
            "💸 Withdraw event: vault={}, amount={}, new_balance={}",
            vault_pubkey, amount, new_balance
        );

        // Update database
        self.state.database
            .update_vault_balances(
                &vault_pubkey,
                new_balance,
                0,
                None,
                Some(amount), // Add to total withdrawn
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Record transaction
        self.state.database
            .record_transaction(
                &vault_pubkey,
                tx_signature,
                "withdraw",
                amount,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Invalidate cache
        self.state.cache.invalidate_vault(&vault_pubkey).await;

        // Broadcast via WebSocket
        broadcast_withdrawal(&vault_pubkey, amount, tx_signature, new_balance).await;

        // Broadcast balance update
        let vault = self.state.database
            .get_vault(&vault_pubkey)
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        if let Some(v) = vault {
            broadcast_balance_update(
                &vault_pubkey,
                v.total_balance,
                v.available_balance,
                v.locked_balance,
            ).await;
        }

        // Update TVL
        self.update_tvl().await?;

        tracing::info!("✅ Withdraw event processed successfully");
        Ok(())
    }

    /// Handle lock event
    async fn handle_lock_event(
        &self,
        event: LockEvent,
        tx_signature: &str,
    ) -> Result<(), EventListenerError> {
        let vault_pubkey = event.vault_pubkey();
        let amount = event.amount as i64;
        let new_locked = event.new_locked as i64;
        let new_available = event.new_available as i64;

        tracing::info!(
            "🔒 Lock event: vault={}, amount={}, new_locked={}, new_available={}",
            vault_pubkey, amount, new_locked, new_available
        );

        // Get current vault for total balance
        let vault = self.state.database
            .get_vault(&vault_pubkey)
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?
            .ok_or_else(|| EventListenerError::VaultNotFound(vault_pubkey.clone()))?;

        // Update database
        self.state.database
            .update_vault_balances(
                &vault_pubkey,
                vault.total_balance,
                new_locked,
                None,
                None,
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Record transaction
        self.state.database
            .record_transaction(
                &vault_pubkey,
                tx_signature,
                "lock",
                amount,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Invalidate cache
        self.state.cache.invalidate_vault(&vault_pubkey).await;

        // Broadcast via WebSocket
        broadcast_lock(&vault_pubkey, amount, new_locked, new_available).await;

        tracing::info!("✅ Lock event processed successfully");
        Ok(())
    }

    /// Handle unlock event
    async fn handle_unlock_event(
        &self,
        event: UnlockEvent,
        tx_signature: &str,
    ) -> Result<(), EventListenerError> {
        let vault_pubkey = event.vault_pubkey();
        let amount = event.amount as i64;
        let new_locked = event.new_locked as i64;
        let new_available = event.new_available as i64;

        tracing::info!(
            "🔓 Unlock event: vault={}, amount={}, new_locked={}, new_available={}",
            vault_pubkey, amount, new_locked, new_available
        );

        // Get current vault
        let vault = self.state.database
            .get_vault(&vault_pubkey)
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?
            .ok_or_else(|| EventListenerError::VaultNotFound(vault_pubkey.clone()))?;

        // Update database
        self.state.database
            .update_vault_balances(
                &vault_pubkey,
                vault.total_balance,
                new_locked,
                None,
                None,
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Record transaction
        self.state.database
            .record_transaction(
                &vault_pubkey,
                tx_signature,
                "unlock",
                amount,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Invalidate cache
        self.state.cache.invalidate_vault(&vault_pubkey).await;

        // Broadcast via WebSocket
        broadcast_unlock(&vault_pubkey, amount, new_locked, new_available).await;

        tracing::info!("✅ Unlock event processed successfully");
        Ok(())
    }

    /// Handle transfer event
    async fn handle_transfer_event(
        &self,
        event: TransferEvent,
        tx_signature: &str,
    ) -> Result<(), EventListenerError> {
        let from_vault = event.from_vault_pubkey();
        let to_vault = event.to_vault_pubkey();
        let amount = event.amount as i64;

        tracing::info!(
            "↔️ Transfer event: from={}, to={}, amount={}",
            from_vault, to_vault, amount
        );

        // Record transaction for both vaults
        self.state.database
            .record_transaction(
                &from_vault,
                tx_signature,
                "transfer",
                amount,
                Some(&from_vault),
                Some(&to_vault),
                "confirmed",
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Invalidate both caches
        self.state.cache.invalidate_vault(&from_vault).await;
        self.state.cache.invalidate_vault(&to_vault).await;

        // Sync both vaults from chain to get accurate balances
        if let Err(e) = crate::services::VaultManager::sync_vault_from_chain(&self.state, &from_vault).await {
            tracing::warn!("Failed to sync from vault {}: {}", from_vault, e);
        }
        if let Err(e) = crate::services::VaultManager::sync_vault_from_chain(&self.state, &to_vault).await {
            tracing::warn!("Failed to sync to vault {}: {}", to_vault, e);
        }

        tracing::info!("✅ Transfer event processed successfully");
        Ok(())
    }

    /// Handle vault initialized event
    async fn handle_vault_initialized_event(
        &self,
        event: VaultInitializedEvent,
        tx_signature: &str,
    ) -> Result<(), EventListenerError> {
        let vault_pubkey = event.vault_pubkey();
        let owner_pubkey = event.owner_pubkey();
        let token_account = event.token_account_pubkey();

        tracing::info!(
            "🆕 Vault initialized event: vault={}, owner={}, token_account={}",
            vault_pubkey, owner_pubkey, token_account
        );

        // Check if vault already exists in database
        let existing = self.state.database
            .get_vault(&vault_pubkey)
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        if existing.is_none() {
            // Sync new vault from chain to populate database
            if let Err(e) = crate::services::VaultManager::sync_vault_from_chain(&self.state, &vault_pubkey).await {
                tracing::warn!("Failed to sync newly initialized vault {}: {}", vault_pubkey, e);
            } else {
                tracing::info!("Synced newly initialized vault {} from chain", vault_pubkey);
            }
        }

        // Record transaction
        self.state.database
            .record_transaction(
                &vault_pubkey,
                tx_signature,
                "initialize",
                0,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Update TVL
        self.update_tvl().await?;

        tracing::info!("✅ Vault initialized event processed successfully");
        Ok(())
    }

    /// Sync a vault from on-chain data
    async fn sync_vault(&self, vault_pubkey: &str) -> Result<(), EventListenerError> {
        if let Err(e) = crate::services::VaultManager::sync_vault_from_chain(&self.state, vault_pubkey).await {
            tracing::warn!("Failed to sync vault {}: {}", vault_pubkey, e);
        }
        Ok(())
    }

    /// Update TVL stats and broadcast
    async fn update_tvl(&self) -> Result<(), EventListenerError> {
        let stats = self.state.database
            .get_tvl_stats()
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Update cache
        self.state.cache.set_tvl_stats(stats.clone()).await;

        // Broadcast TVL update
        broadcast_tvl_update(stats.total_vaults, stats.total_value_locked).await;

        Ok(())
    }
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum EventListenerError {
    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Vault not found: {0}")]
    VaultNotFound(String),

    #[error("Event processing error: {0}")]
    ProcessingError(String),
}

// ============================================================================
// Public API for starting the event listener
// ============================================================================

/// Start the event listener as a background task
pub async fn run_event_listener(state: Data<AppState>) {
    tracing::info!("🚀 Initializing Event Listener...");
    
    let config = EventListenerConfig::default();
    let mut listener = EventListener::new(state, config);
    
    // This should never return under normal operation
    listener.start().await;
    
    // If we get here, something went wrong
    tracing::error!("❌ Event Listener unexpectedly exited!");
}

/// Start event listener with custom configuration
pub async fn run_event_listener_with_config(
    state: Data<AppState>,
    config: EventListenerConfig,
) {
    tracing::info!("🚀 Initializing Event Listener with custom config...");
    
    let mut listener = EventListener::new(state, config);
    listener.start().await;
    
    tracing::error!("❌ Event Listener unexpectedly exited!");
}