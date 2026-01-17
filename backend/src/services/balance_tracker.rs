use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use spl_token::state::Account as TokenAccount;
use std::str::FromStr;

use crate::services::AppState;

pub struct BalanceTracker;

impl BalanceTracker {
    pub async fn get_on_chain_balance(
        state: &AppState,
        token_account_pubkey: &str,
    ) -> Result<u64, BalanceError> {
        let pubkey =
            Pubkey::from_str(token_account_pubkey).map_err(|_| BalanceError::InvalidPubkey)?;

        let account_data = state
            .solana_client
            .get_account_data(&pubkey)
            .await
            .map_err(|e| BalanceError::SolanaRpcError(e.to_string()))?;
        let token_account = TokenAccount::unpack(&account_data)
            .map_err(|e| BalanceError::DeserializationError(e.to_string()))?;
        Ok(token_account.amount)
    }

    pub async fn has_sufficient_balance(
        state: &AppState,
        vault_pubkey: &str,
        required_amount: i64,
    ) -> Result<bool, BalanceError> {
        let vault = state
            .database
            .get_vault(vault_pubkey)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?
            .ok_or(BalanceError::VaultNotFound)?;

        Ok(vault.available_balance >= required_amount)
    }

    pub async fn calculate_utilization(
        state: &AppState,
        vault_pubkey: &str,
    ) -> Result<f64, BalanceError> {
        let vault = state
            .database
            .get_vault(vault_pubkey)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?
            .ok_or(BalanceError::VaultNotFound)?;

        if vault.total_balance == 0 {
            return Ok(0.0);
        }

        Ok((vault.locked_balance as f64 / vault.total_balance as f64) * 100.0)
    }

    pub async fn check_low_balances(
        state: &AppState,
        vault_pubkey: &str,
        threshold: i64,
    ) -> Result<Option<i64>, BalanceError> {
        let vault = state
            .database
            .get_vault(vault_pubkey)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?
            .ok_or(BalanceError::VaultNotFound)?;

        if vault.available_balance < threshold {
            let alert_id = state
                .database
                .create_alert(
                    "low_balance",
                    "warning",
                    Some(vault_pubkey),
                    &format!(
                        "Available balance ({}) below threshold ({})",
                        vault.available_balance, threshold
                    ),
                    None,
                )
                .await
                .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

            tracing::warn!(
                "Low balance alert created for vault {}: {} < {}",
                vault_pubkey,
                vault.available_balance,
                threshold
            );

            return Ok(Some(alert_id));
        }

        Ok(None)
    }

    pub async fn recomcile_balance(
        state: &AppState,
        vault_pubkey: &str,
    ) -> Result<ReconciliationResult, BalanceError> {
        let vault = state
            .database
            .get_vault(vault_pubkey)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?
            .ok_or(BalanceError::VaultNotFound)?;

        let on_chain_balance = Self::get_on_chain_balance(state, &vault.token_account).await?;
        let expected_balance = vault.total_balance;
        let actual_balance = on_chain_balance as i64;
        let discrepancy = actual_balance - expected_balance;

        state
            .database
            .create_balance_snapshot(
                vault_pubkey,
                vault.total_balance,
                vault.locked_balance,
                vault.available_balance,
                actual_balance,
                "reconciliation",
            )
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

        if discrepancy != 0 {
            let log_id = state
                .database
                .log_reconciliation_issue(
                    vault_pubkey,
                    expected_balance,
                    actual_balance,
                    discrepancy,
                )
                .await
                .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

            state
                .database
                .create_alert(
                    "balance_discrepancy",
                    "critical",
                    Some(vault_pubkey),
                    &format!(
                        "Balance mismatch: expected {}, actual {}, diff {}",
                        expected_balance, actual_balance, discrepancy
                    ),
                    Some(serde_json::json!({
                        "reconciliation_log_id": log_id,
                        "expected": expected_balance,
                        "actual": actual_balance,
                        "discrepancy": discrepancy,
                    })),
                )
                .await
                .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;
            tracing::error!(
                "Balance discrepancy detected for vault {}: expected {}, actual {}, diff {}",
                vault_pubkey,
                expected_balance,
                actual_balance,
                discrepancy
            );

            return Ok(ReconciliationResult {
                vault_pubkey: vault_pubkey.to_string(),
                expected_balance,
                actual_balance,
                discrepancy,
                status: ReconciliationStatus::Mismatch,
            });
        }
        tracing::debug!("Balance reconciliation OK for vault {}", vault_pubkey);

        Ok(ReconciliationResult {
            vault_pubkey: vault_pubkey.to_string(),
            expected_balance,
            actual_balance,
            discrepancy: 0,
            status: ReconciliationStatus::Match,
        })
    }

    pub async fn verify_balance_invariant(
        state: &AppState,
        vault_pubkey: &str,
    ) -> Result<bool, BalanceError> {
        let vault = state
            .database
            .get_vault(vault_pubkey)
            .await
            .map_err(|e| BalanceError::DatabaseError(e.to_string()))?
            .ok_or(BalanceError::VaultNotFound)?;

        let calculated_total = vault.available_balance + vault.locked_balance;

        if calculated_total != vault.total_balance {
            tracing::error!(
                "Balance invariant violation for vault {}: total={}, available={}, locked={}, calculated={}",
                vault_pubkey, vault.total_balance, vault.available_balance,
                vault.locked_balance, calculated_total
            );

            state
                .database
                .create_alert(
                    "invariant_violation",
                    "critical",
                    Some(vault_pubkey),
                    &format!(
                        "Balance invariant violated: {} != {} + {}",
                        vault.total_balance, vault.available_balance, vault.locked_balance
                    ),
                    None,
                )
                .await
                .map_err(|e| BalanceError::DatabaseError(e.to_string()))?;

            return Ok(false);
        }

        Ok(true)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ReconciliationResult {
    pub vault_pubkey: String,
    pub expected_balance: i64,
    pub actual_balance: i64,
    pub discrepancy: i64,
    pub status: ReconciliationStatus,
}

#[derive(Debug, serde::Serialize)]
pub enum ReconciliationStatus {
    Match,
    Mismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum BalanceError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Solana RPC error: {0}")]
    SolanaRpcError(String),

    #[error("Invalid pubkey")]
    InvalidPubkey,

    #[error("Vault not found")]
    VaultNotFound,

    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}
