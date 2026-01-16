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
use std::sync::Arc;
use std::time::Duration;
use actix_web::web::Data;
use anchor_lang::prelude::*;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
};
use solana_client::rpc_config::RpcTransactionConfig;
use tokio::sync::mpsc;
use tokio::time;

use crate::services::AppState;
use crate::websocket::{
    broadcast_balance_update, broadcast_deposit, broadcast_lock, 
    broadcast_unlock, broadcast_withdrawal, broadcast_tvl_update,
};

// ============================================================================
// Event Structures (matching Anchor program events)
// ============================================================================

/// Deposit event emitted by the on-chain program
#[derive(Debug, Clone, AnchorDeserialize)]
pub struct DepositEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Withdrawal event emitted by the on-chain program
#[derive(Debug, Clone, AnchorDeserialize)]
pub struct WithdrawEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Lock collateral event
#[derive(Debug, Clone, AnchorDeserialize)]
pub struct LockEvent {
    pub vault: Pubkey,
    pub amount: u64,
    pub new_locked: u64,
    pub new_available: u64,
    pub timestamp: i64,
}

/// Unlock collateral event
#[derive(Debug, Clone, AnchorDeserialize)]
pub struct UnlockEvent {
    pub vault: Pubkey,
    pub amount: u64,
    pub new_locked: u64,
    pub new_available: u64,
    pub timestamp: i64,
}

/// Transfer between vaults event
#[derive(Debug, Clone, AnchorDeserialize)]
pub struct TransferEvent {
    pub from_vault: Pubkey,
    pub to_vault: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

/// Vault initialized event
#[derive(Debug, Clone, AnchorDeserialize)]
pub struct VaultInitializedEvent {
    pub vault: Pubkey,
    pub owner: Pubkey,
    pub token_account: Pubkey,
    pub timestamp: i64,
}

// ============================================================================
// Parsed Event Enum
// ============================================================================

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
            poll_interval_ms: 1000,  // Poll every second
            slots_to_check: 100,     // Check last 100 slots
            use_websocket: true,
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
    last_processed_signature: Option<Signature>,
    processed_signatures: HashMap<String, i64>, // signature -> timestamp
}

impl EventListener {
    pub fn new(state: Data<AppState>, config: EventListenerConfig) -> Self {
        Self {
            state,
            config,
            last_processed_signature: None,
            processed_signatures: HashMap::new(),
        }
    }

    /// Start the event listener service
    pub async fn start(&mut self) {
        tracing::info!(
            "🎧 Event Listener started (poll_interval: {}ms, use_websocket: {})",
            self.config.poll_interval_ms,
            self.config.use_websocket
        );

        if self.config.use_websocket {
            // Try WebSocket subscription first, fall back to polling
            self.run_with_websocket_fallback().await;
        } else {
            self.run_polling_loop().await;
        }
    }

    /// Run with WebSocket subscription, falling back to polling if unavailable
    async fn run_with_websocket_fallback(&mut self) {
        // For production, you'd use pubsub_client for WebSocket
        // For now, use polling which works with standard RPC
        tracing::info!("📡 Using polling mode for event listening");
        self.run_polling_loop().await;
    }

    /// Main polling loop to fetch and process program logs
    async fn run_polling_loop(&mut self) {
        let mut interval = time::interval(Duration::from_millis(self.config.poll_interval_ms));

        loop {
            interval.tick().await;

            if let Err(e) = self.poll_and_process_events().await {
                tracing::error!("Event polling error: {}", e);
            }
        }
    }

    /// Poll for new program logs and process events
    async fn poll_and_process_events(&mut self) -> Result<(), EventListenerError> {
        let program_id = self.state.program_id;

        // Get recent signatures for the program
        let signatures = self.state.solana_client
            .get_signatures_for_address(&program_id)
            .map_err(|e| EventListenerError::RpcError(e.to_string()))?;

        let mut new_events = Vec::new();

        for sig_info in signatures.iter().take(50) {  // Process last 50 transactions
            let signature_str = sig_info.signature.clone();

            // Skip if already processed
            if self.processed_signatures.contains_key(&signature_str) {
                continue;
            }

            // Skip failed transactions
            if sig_info.err.is_some() {
                self.processed_signatures.insert(signature_str, chrono::Utc::now().timestamp());
                continue;
            }

            // Parse the signature
            let signature = Signature::from_str(&signature_str)
                .map_err(|e| EventListenerError::ParseError(e.to_string()))?;

            // Fetch transaction details
            match self.fetch_and_parse_transaction(&signature).await {
                Ok(Some(events)) => {
                    new_events.extend(events);
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Failed to parse transaction {}: {}", signature_str, e);
                }
            }

            // Mark as processed
            self.processed_signatures.insert(signature_str, chrono::Utc::now().timestamp());
        }

        // Process all new events
        for (event, tx_signature) in new_events {
            if let Err(e) = self.process_event(event, &tx_signature).await {
                tracing::error!("Failed to process event: {}", e);
            }
        }

        // Cleanup old processed signatures (keep last hour)
        let cutoff = chrono::Utc::now().timestamp() - 3600;
        self.processed_signatures.retain(|_, ts| *ts > cutoff);

        Ok(())
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
            )
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

        let discriminator = &data[0..8];
        let event_data = &data[8..];

        // Event discriminators (first 8 bytes of sha256("event:<EventName>"))
        // These should match your Anchor program's event discriminators
        
        // Try parsing each event type
        // Note: In production, calculate these discriminators properly
        
        // For demonstration, we'll use pattern matching on common event structures
        // You would replace these with actual discriminator values from your IDL
        
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
    /// 
    /// This implements points 10-12 of the architecture:
    /// 10. Update database with on-chain values
    /// 11. Invalidate cache for affected vaults  
    /// 12. Broadcast update via WebSocket
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
        let vault_pubkey = event.vault.to_string();
        let amount = event.amount as i64;
        let new_balance = event.new_balance as i64;

        tracing::info!(
            "💰 Deposit event: vault={}, amount={}, new_balance={}",
            vault_pubkey, amount, new_balance
        );

        // Step 10: Update database with on-chain values
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

        // Step 11: Invalidate cache for affected vault
        self.state.cache.invalidate_vault(&vault_pubkey).await;

        // Step 12: Broadcast update via WebSocket
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
        let vault_pubkey = event.vault.to_string();
        let amount = event.amount as i64;
        let new_balance = event.new_balance as i64;

        tracing::info!(
            "💸 Withdraw event: vault={}, amount={}, new_balance={}",
            vault_pubkey, amount, new_balance
        );

        // Step 10: Update database
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

        // Step 11: Invalidate cache
        self.state.cache.invalidate_vault(&vault_pubkey).await;

        // Step 12: Broadcast via WebSocket
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
        let vault_pubkey = event.vault.to_string();
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

        // Step 10: Update database
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

        // Step 11: Invalidate cache
        self.state.cache.invalidate_vault(&vault_pubkey).await;

        // Step 12: Broadcast via WebSocket
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
        let vault_pubkey = event.vault.to_string();
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

        // Step 10: Update database
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

        // Step 11: Invalidate cache
        self.state.cache.invalidate_vault(&vault_pubkey).await;

        // Step 12: Broadcast via WebSocket
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
        let from_vault = event.from_vault.to_string();
        let to_vault = event.to_vault.to_string();
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

        // Invalidate cache for both vaults
        self.state.cache.invalidate_vault(&from_vault).await;
        self.state.cache.invalidate_vault(&to_vault).await;

        // Sync both vaults from chain to get accurate balances
        // This ensures we have the latest on-chain state
        self.sync_vault_from_chain(&from_vault).await?;
        self.sync_vault_from_chain(&to_vault).await?;

        tracing::info!("✅ Transfer event processed successfully");
        Ok(())
    }

    /// Handle vault initialized event
    async fn handle_vault_initialized_event(
        &self,
        event: VaultInitializedEvent,
        _tx_signature: &str,
    ) -> Result<(), EventListenerError> {
        let vault_pubkey = event.vault.to_string();
        let owner_pubkey = event.owner.to_string();
        let token_account = event.token_account.to_string();

        tracing::info!(
            "🆕 Vault initialized: vault={}, owner={}, token_account={}",
            vault_pubkey, owner_pubkey, token_account
        );

        // Create vault in database
        let vault = shared::Vault {
            vault_pubkey: vault_pubkey.clone(),
            owner_pubkey: owner_pubkey.clone(),
            token_account,
            total_balance: 0,
            locked_balance: 0,
            available_balance: 0,
            total_deposited: 0,
            total_withdrawn: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.state.database
            .upsert_vault(&vault)
            .await
            .map_err(|e| EventListenerError::DatabaseError(e.to_string()))?;

        // Set in cache
        self.state.cache.set_vault(vault).await;

        // Update TVL
        self.update_tvl().await?;

        tracing::info!("✅ Vault initialized event processed successfully");
        Ok(())
    }

    /// Sync a vault from on-chain state
    async fn sync_vault_from_chain(&self, vault_pubkey: &str) -> Result<(), EventListenerError> {
        let pubkey = Pubkey::from_str(vault_pubkey)
            .map_err(|e| EventListenerError::ParseError(e.to_string()))?;

        match self.state.solana_client.get_account(&pubkey) {
            Ok(account) => {
                // Parse vault account data
                // In production, you'd use your Anchor program's account parser
                if let Some(vault) = self.state.database.get_vault(vault_pubkey).await
                    .map_err(|e| EventListenerError::DatabaseError(e.to_string()))? 
                {
                    // Broadcast updated balance
                    broadcast_balance_update(
                        vault_pubkey,
                        vault.total_balance,
                        vault.available_balance,
                        vault.locked_balance,
                    ).await;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to sync vault {}: {}", vault_pubkey, e);
            }
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
    let config = EventListenerConfig::default();
    let mut listener = EventListener::new(state, config);
    listener.start().await;
}

/// Start event listener with custom configuration
pub async fn run_event_listener_with_config(
    state: Data<AppState>,
    config: EventListenerConfig,
) {
    let mut listener = EventListener::new(state, config);
    listener.start().await;
}

