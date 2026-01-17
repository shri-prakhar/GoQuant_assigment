use std::time::Duration;

use tokio::time;

use crate::services::{AppState, BalanceTracker};

pub async fn run_reconciler(state: actix_web::web::Data<AppState>) {
    let interval_secs = state.config.reconciliation_interval_seconds;
    let mut interval = time::interval(Duration::from_secs(interval_secs));

    tracing::info!("Balance Reconciler started (interval: {}s)", interval_secs);

    loop {
        interval.tick().await;

        if let Err(e) = reconciliation_cycle(&state).await {
            tracing::error!("Reconciliation cycle error: {}", e);
        }
    }
}

async fn reconciliation_cycle(state: &AppState) -> Result<(), ReconcilerError> {
    tracing::info!("Starting reconciliation cycle...");
    let vaults = state
        .database
        .get_all_vaults(10000, 0)
        .await
        .map_err(|_| ReconcilerError::DatabaseError("Database error".to_string()))?;
    let mut total_vaults = 0;
    let mut mismatches = 0;
    let mut errors = 0;

    for vault in vaults {
        total_vaults += 1;
         if vault.vault_pubkey.len() < 32 || vault.vault_pubkey.len() > 44 {
            tracing::warn!("Skipping vault with invalid pubkey format: {}", vault.vault_pubkey);
            errors += 1;
            continue;
        }
        match BalanceTracker::recomcile_balance(state, &vault.vault_pubkey).await {
            Ok(result) => match result.status {
                crate::services::balance_tracker::ReconciliationStatus::Mismatch => {
                    mismatches += 1;
                    tracing::warn!(
                        "Mismatch for vault {}: expected {}, actual {}, diff {}",
                        result.vault_pubkey,
                        result.expected_balance,
                        result.actual_balance,
                        result.discrepancy
                    );
                }
                _ => {}
            },
            Err(e) => {
                errors += 1;
                tracing::error!(
                    "Reconciliation failed for vault {}: {}",
                    vault.vault_pubkey,
                    e
                );
            }
        }
    }

    tracing::info!(
        "Reconciliation cycle completed: {} vaults, {} mismatches, {} errors",
        total_vaults,
        mismatches,
        errors
    );

    if mismatches > 0 {
        state
            .database
            .create_alert(
                "reconciliation_summary",
                "warning",
                None,
                &format!(
                    "Reconciliation found {} mismatches out of {} vaults",
                    mismatches, total_vaults
                ),
                Some(serde_json::json!({
                    "total_vaults": total_vaults,
                    "mismatches": mismatches,
                    "errors": errors,
                })),
            )
            .await
            .map_err(|e| ReconcilerError::DatabaseError(e.to_string()))?;
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ReconcilerError {
    #[error("Database error: {0}")]
    DatabaseError(String),
}
