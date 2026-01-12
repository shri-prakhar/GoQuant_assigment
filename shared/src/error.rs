use thiserror::Error;


#[derive(Debug , Error)]
pub enum VaultError{
  #[error("Database error: {0}")]
  DatabaseError(String),
  #[error("Invalid pubkey: {0}")]
  InvalidPubkey(String),
  #[error("Vault not found: {0}")]
  VaultNotFound(String),
  #[error("Insufficient Balance : available={available}, required={required}")]
  InsufficientBalance {available : i64 , required : i64},
  #[error("Insufficient locked balance: locked={locked}, required={required}")]
  InsufficientLockedBalance { locked: i64, required: i64 },
  #[error("Invalid amount: {0}")]
  InvalidAmount(String),
  #[error("Arithmetic overflow")]
  Overflow,
  #[error("Arithmetic underflow")]
  Underflow,
  #[error("Balance invariant violation: total={total}, available={available}, locked={locked}")]
  BalanceInvariantViolation { total: i64, available: i64, locked: i64 },
  #[error("Unauthorized operation")]
  Unauthorized,
  #[error("Transaction not found: {0}")]
  TransactionNotFound(String),
  #[error("Solana RPC error: {0}")]
  SolanaRpcError(String),
  #[error("Serialization error: {0}")]
  SerializationError(String),
  #[error("Deserialization error: {0}")]
  DeserializationError(String),
  #[error("Configuration error: {0}")]
  ConfigError(String),
  #[error("Internal error: {0}")]
  InternalError(String),
}

pub type VaultResult<T> = Result<T , VaultError>;