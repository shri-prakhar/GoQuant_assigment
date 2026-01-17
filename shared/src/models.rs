//! # Shared Data Models
//!
//! Common data structures used across the Collateral Vault system.
//! These models are shared between the backend API and other components.
//!
//! ## Core Entities
//!
//! - `Vault`: Represents a user's collateral vault
//! - `TransactionRecord`: Records all vault operations
//! - `TvlStats`: Total Value Locked statistics
//! - Various supporting types for alerts, audits, and snapshots

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, types::JsonValue};

/// Represents a collateral vault owned by a user
///
/// A vault holds tokens as collateral that can be deposited, withdrawn,
/// locked for DeFi protocols, or transferred between vaults.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Vault {
    /// Public key of the vault account on Solana
    pub vault_pubkey: String,
    /// Public key of the vault owner
    pub owner_pubkey: String,
    /// Associated token account that holds the collateral tokens
    pub token_account: String,
    /// Total balance of tokens in the vault (available + locked)
    pub total_balance: i64,
    /// Amount of tokens currently locked for DeFi protocols
    pub locked_balance: i64,
    /// Amount of tokens available for withdrawal or locking
    pub available_balance: i64,
    /// Total amount deposited into this vault over its lifetime
    pub total_deposited: i64,
    /// Total amount withdrawn from this vault over its lifetime
    pub total_withdrawn: i64,
    /// When the vault was created
    pub created_at: DateTime<Utc>,
    /// When the vault was last updated
    pub updated_at: DateTime<Utc>,
}

impl Vault {
    /// Get the available balance for operations
    #[inline]
    pub fn available(&self) -> i64 {
        self.available_balance
    }

    /// Check if vault has sufficient available balance
    #[inline]
    pub fn has_available(&self, amount: i64) -> bool {
        self.available_balance >= amount
    }

    /// Check if vault has sufficient locked balance
    #[inline]
    pub fn has_locked(&self, amount: i64) -> bool {
        self.locked_balance >= amount
    }

    /// Verify that the vault's balance invariant holds
    ///
    /// The invariant is: total_balance = available_balance + locked_balance
    #[inline]
    pub fn verify_invariant(&self) -> bool {
        self.total_balance == (self.available_balance + self.locked_balance)
    }

    /// Calculate the utilization percentage of the vault
    ///
    /// Returns the percentage of total balance that is locked (0.0 to 100.0)
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total_balance == 0 {
            0.0
        } else {
            (self.locked_balance as f64 / self.total_balance as f64) * 100.0
        }
    }
}

/// Record of a vault transaction/operation
///
/// Tracks all operations performed on vaults for audit and history purposes.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TransactionRecord {
    /// Database primary key
    #[sqlx(default)]
    pub id: i64,
    /// Public key of the vault involved in the transaction
    pub vault_pubkey: String,
    /// Solana transaction signature
    pub tx_signature: String,
    /// Type of transaction (deposit, withdraw, lock, unlock, transfer)
    pub tx_type: String,
    /// Amount of tokens involved in the transaction
    pub amount: i64,
    /// Source vault for transfers (optional)
    pub from_vault: Option<String>,
    /// Destination vault for transfers (optional)
    pub to_vault: Option<String>,
    /// Current status of the transaction
    pub status: String,
    /// Solana block time when transaction was confirmed
    pub block_time: Option<i64>,
    /// Solana slot number
    pub slot: Option<i64>,
    /// When this record was created in the database
    pub created_at: DateTime<Utc>,
    /// When the transaction was confirmed on-chain
    pub confirmed_at: Option<DateTime<Utc>>,
    /// Additional metadata as JSON
    pub meta: Option<JsonValue>,
}

/// Types of vault transactions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    /// Deposit tokens into vault
    Deposit,
    /// Withdraw tokens from vault
    Withdraw,
    /// Lock collateral for DeFi use
    Lock,
    /// Unlock previously locked collateral
    Unlock,
    /// Transfer collateral between vaults
    Transfer,
}

impl TransactionType {
    /// Convert enum to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Deposit => "deposit",
            TransactionType::Lock => "lock",
            TransactionType::Transfer => "transfer",
            TransactionType::Unlock => "unlock",
            TransactionType::Withdraw => "withdraw",
        }
    }
}

/// Status of a vault transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    /// Transaction is pending confirmation
    Pending,
    /// Transaction has been confirmed on-chain
    Confirmed,
    /// Transaction failed
    Failed,
}

impl TransactionStatus {
    /// Convert enum to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionStatus::Pending => "pending",
            TransactionStatus::Confirmed => "confirmed",
            TransactionStatus::Failed => "failed",
        }
    }
}

#[derive(Debug , Clone , Serialize , Deserialize , FromRow)]
pub struct BalanceSnapshot{
  #[sqlx(default)]
  pub id: i64,
  pub vault_pubkey: String,
  pub total_balance: i64,
  pub locked_balance: i64,
  pub available_balance: i64,
  pub on_chain_token_balance: i64,
  pub snapshot_type: String,
  pub snapshot_ts: DateTime<Utc>,
  pub discrepancy: i64,
}

#[derive(Debug , Clone , Serialize , Deserialize , PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotType{
  Hourly,
  Daily,
  Reconciliation
}

impl SnapshotType{
  pub fn as_str(&self) -> &'static str {
    match self {
        SnapshotType::Daily => "daily",
        SnapshotType::Hourly => "hourly",
        SnapshotType::Reconciliation => "reconciliation",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReconciliationLog {
    pub id: i64,
    pub vault_pubkey: String,
    pub expected_balance: i64,
    pub actual_balance: i64,
    pub discrepancy: i64,
    pub resolution_status: String,
    pub resolution_notes: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReconciliationStatus {
    Detected,
    Investigating,
    Resolved,
}

impl ReconciliationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReconciliationStatus::Detected => "detected",
            ReconciliationStatus::Investigating => "investigating",
            ReconciliationStatus::Resolved => "resolved",
        }
    }
}

#[derive(Debug , Clone , Serialize , Deserialize , FromRow)]
pub struct AuditTrailEntry {
  pub id : i64,
  pub event_type : String,
  pub vault_pubkey : Option<String>,
  pub user_pubkey :Option< String>,
  pub amount : Option<i64>,
  pub tx_signature: Option<String>,
  pub event_data: JsonValue,
  pub ip_address: Option<String>,
  pub user_agent: Option<String>,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    VaultCreated,
    BalanceChange,
    Deposit,
    Withdraw,
    Lock,
    Unlock,
    Transfer,
    Reconciliation,
    Alert,
    Error,
}

impl AuditEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventType::VaultCreated => "vault_created",
            AuditEventType::BalanceChange => "balance_change",
            AuditEventType::Deposit => "deposit",
            AuditEventType::Withdraw => "withdraw",
            AuditEventType::Lock => "lock",
            AuditEventType::Unlock => "unlock",
            AuditEventType::Transfer => "transfer",
            AuditEventType::Reconciliation => "reconciliation",
            AuditEventType::Alert => "alert",
            AuditEventType::Error => "error",
        }
    }
}

#[derive(Debug , Clone , Deserialize , Serialize , FromRow)]
pub struct Alert {
    pub id: i64,
    pub alert_type: String,
    pub severity: String,
    pub vault_pubkey: Option<String>,
    pub message: String,
    pub details: Option<JsonValue>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "INFO",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
}

impl AlertStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertStatus::Active => "active",
            AlertStatus::Acknowledged => "acknowledged",
            AlertStatus::Resolved => "resolved",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvlStats {
    pub total_vaults: i64,
    pub total_value_locked: i64,
    pub total_available: i64,
    pub total_locked: i64,
    pub avg_vault_balance: f64,
    pub max_vault_balance: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug , Clone , Serialize , Deserialize)]
pub struct CreateVaultRequest{
  pub vault_pubkey: String,
  pub owner_pubkey: String,
  pub token_account: String,
} 

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDepositRequest {
    pub vault_pubkey: String,
    pub amount: i64,
    pub tx_signature: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessWithdrawalRequest {
    pub vault_pubkey: String,
    pub amount: i64,
    pub tx_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockCollateralRequest {
    pub vault_pubkey: String,
    pub amount: i64,
    pub tx_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockCollateralRequest {
    pub vault_pubkey: String,
    pub amount: i64,
    pub tx_signature: String,
}

#[derive(Debug , Clone , Serialize ,Deserialize)]

pub struct ApiResponse<T>{
  pub success : bool ,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data : Option<T>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error : Option<String>,
}

impl<T> ApiResponse<T>{
    pub fn success(data: T) -> Self {
      Self { success: true, data: Some(data), error: None }
    }

    pub fn error(error : String) -> Self{
      Self { success: false, data: None, error: Some(error) }
    }
}

#[derive(Debug , Clone , Serialize , Deserialize)]
pub struct PaginationParams{
  #[serde(default = "default_limit")]
  pub limit : i64,
  #[serde(default)]
  pub offset : i64
}

fn default_limit() -> i64 {
  100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        let has_more = (offset + items.len() as i64) < total;
        Self {
            items,
            total,
            limit,
            offset,
            has_more,
        }
    }
}

