use std::time::Duration;

use actix_web::web::Data;
use tokio::time;

use crate::services::{AppState, BalanceTracker};

pub async fn run_monitor(state: Data<AppState>) {
    let interval_secs = state.config.monitoring_interval_seconds;
    let mut interval = time::interval(Duration::from_secs(interval_secs));
    tracing::info!("Vault Monitor started (interval: {}s)", interval_secs);
    loop {
        interval.tick().await;
        if let Err(e) = monitor_cycle(&state).await {
            tracing::error!("Monitor cycle error: {}", e);
        }
    }
}

async fn monitor_cycle(state: &AppState) -> Result<(), MonitorError> {
    tracing::debug!("Running monitoring cycle...");
    let vaults = state
        .database
        .get_all_vaults(10000, 0)
        .await
        .map_err(|e| MonitorError::DatabaseError(e.to_string()))?;
    tracing::debug!("Monitoring {} vaults", vaults.len());
    for vault in vaults {
        if vault.vault_pubkey.len() < 32 || vault.vault_pubkey.len() > 44 {
            tracing::debug!("Skipping vault with invalid pubkey: {}", vault.vault_pubkey);
            continue;
        }
        if let Err(e) = BalanceTracker::verify_balance_invariant(state, &vault.vault_pubkey).await {
            tracing::error!(
                "Balance invariant check failed for vault {}: {}",
                vault.vault_pubkey,
                e
            );
        }
        let threshold = (vault.total_balance as f64 * 0.1) as i64;
        if threshold > 0 {
            if let Err(e) =
                BalanceTracker::check_low_balances(state, &vault.vault_pubkey, threshold).await
            {
                tracing::error!(
                    "Low balance check failed for vault {}: {}",
                    vault.vault_pubkey,
                    e
                );
            }
        }
        match BalanceTracker::calculate_utilization(state, &vault.vault_pubkey).await {
            Ok(utilization) if utilization > 90.0 => {
                tracing::warn!(
                    "High utilization for vault {}: {:.2}%",
                    vault.vault_pubkey,
                    utilization
                );

                let _ = state
                    .database
                    .create_alert(
                        "high_utilization",
                        "warning",
                        Some(&vault.vault_pubkey),
                        &format!("Vault utilization at {:.2}%", utilization),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                tracing::error!(
                    "Utilization check failed for vault {}: {}",
                    vault.vault_pubkey,
                    e
                );
            }

            _ => {}
        }

        if let Err(e) = update_tvl_stats(state).await {
            tracing::error!("TVL stats update failed: {}", e);
        }

        tracing::debug!("Monitor cycle completed");
    }
    Ok(())
}

async fn update_tvl_stats(state: &AppState) -> Result<(), MonitorError> {
    let stats = state
        .database
        .get_tvl_stats()
        .await
        .map_err(|e| MonitorError::DatabaseError(e.to_string()))?;

    state.cache.set_tvl_stats(stats.clone()).await;

    tracing::debug!(
        "TVL Stats: {} vaults, ${} total",
        stats.total_vaults,
        stats.total_value_locked
    );

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    #[error("Database error: {0}")]
    DatabaseError(String),
}
