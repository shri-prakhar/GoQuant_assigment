use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

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
  #[sqlx(default)]
  pub created_at : i64
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
}

#[derive(Debug , Clone , Serialize , Deserialize , FromRow)]
pub struct TransactionRecord{
  #[sqlx(default)]
  id : i64,
  pub vault_pubkey: String,
  pub tx_signature: String,
  pub tx_type: String,
  pub amount: i64,
  pub from_vault: Option<String>,
  pub to_vault: Option<String>,
  pub status: String,
  pub block_time: Option<i64>,
  pub slot: Option<i64>,
  #[sqlx(default)]
  pub created_at :  DateTime<Utc>
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
}


#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Alert {
    pub id: i64,
    pub alert_type: String,
    pub severity: String,
    pub vault_pubkey: Option<String>,
    pub message: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
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
