use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, types::JsonValue};

#[derive(Debug , Clone , Serialize , Deserialize, FromRow )]
pub struct Vault{
  pub vault_pubkey: String,
  pub owner_pubkey : String,
  pub token_account : String,
  pub total_balance : i64,
  pub locked_balance : i64,
  pub available_balance : i64,
  pub total_deposited : i64,
  pub total_withdrawn : i64,
  pub created_at : DateTime<Utc>,
  pub updated_at : DateTime<Utc>,
}

impl Vault {
  #[inline]
  pub fn available(&self) -> i64 {
    self.available_balance
  }
  #[inline]
  pub fn has_available(&self , amount : i64) -> bool {
    self.available_balance >= amount
  }
  #[inline]
  pub fn has_locked(&self , amount: i64) -> bool {
    self.locked_balance >= amount
  }
  #[inline]
  pub fn verify_invariant(&self) -> bool {
    self.total_balance == (self.available_balance + self.locked_balance)
  }
  #[inline]
  pub fn utilization(&self) -> f64 {
    if self.total_balance == 0 {
      0.0
    }else {
      (self.locked_balance as f64 / self.total_balance as f64) * 100.0
    }
  }
}

#[derive(Debug , Clone , Serialize , Deserialize , FromRow)]
pub struct TransactionRecord{
  #[sqlx(default)]
  pub id : i64,
  pub vault_pubkey: String,
  pub tx_signature: String,
  pub tx_type: String,
  pub amount: i64,
  pub from_vault: Option<String>,
  pub to_vault: Option<String>,
  pub status: String,
  pub block_time: Option<i64>,
  pub slot: Option<i64>,
  pub created_at :  DateTime<Utc>,
  pub confirmed_at : Option<DateTime<Utc>>,
  pub meta : Option<JsonValue>,
}


#[derive(Debug , Clone , Serialize , Deserialize , PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdraw,
    Lock,
    Unlock,
    Transfer,
}

impl TransactionType {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

impl TransactionStatus {
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

