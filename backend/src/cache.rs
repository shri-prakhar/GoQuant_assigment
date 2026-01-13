use std::time::Duration;

use moka::future::Cache as MokaCache;
use shared::{TvlStats, Vault};

#[derive(Clone)]
pub struct Cache {
    pub vaults: MokaCache<String, Vault>,
    pub owner_to_vaults: MokaCache<String, String>,
    pub tvl_cache: MokaCache<String, TvlStats>,
}

impl Cache {
    pub fn new(max_capacity: u64) -> Self {
        Self {
            vaults: MokaCache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(300))
                .time_to_idle(Duration::from_secs(60))
                .build(),

            owner_to_vaults: MokaCache::builder()
                .max_capacity(max_capacity)
                .time_to_live(Duration::from_secs(300))
                .time_to_idle(Duration::from_secs(60))
                .build(),

            tvl_cache: MokaCache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(60))
                .build(),
        }
    }

    pub async fn get_vault(&self, vault_pubkey: &str) -> Option<Vault> {
        self.vaults.get(vault_pubkey).await
    }

    pub async fn set_vault(&self, vault: Vault) {
        let pubkey = vault.vault_pubkey.clone();
        let owner_pubkey = vault.owner_pubkey.clone();

        self.vaults.insert(pubkey.clone(), vault).await;
        self.owner_to_vaults.insert(owner_pubkey, pubkey).await;
    }

    pub async fn invalidate_vault(&self, vault_pubkey: &str) {
        self.vaults.invalidate(vault_pubkey).await;
    }

    pub async fn get_vault_by_owner(&self, owner_pubkey: &str) -> Option<String> {
        self.owner_to_vaults.get(owner_pubkey).await
    }
    pub async fn update_vault_balances(
        &self,
        vault_pubkey: &str,
        total_balance: i64,
        locked_balance: i64,
        available_balance: i64,
    ) -> Option<()> {
        let mut vault = self.vaults.get(vault_pubkey).await?;

        vault.total_balance = total_balance;
        vault.locked_balance = locked_balance;
        vault.available_balance = available_balance;

        self.vaults.insert(vault_pubkey.to_string(), vault).await;

        Some(())
    }

    pub async fn get_tvl_stats(&self) -> Option<TvlStats> {
        self.tvl_cache.get("tvl").await
    }

    pub async fn set_tvl_stats(&self, stats: TvlStats) {
        self.tvl_cache.insert("tvl".to_string(), stats).await;
    }

    pub async fn get_stats(&self) -> CacheStats {
        CacheStats {
            vault_entries: self.vaults.entry_count(),
            owner_entries: self.owner_to_vaults.entry_count(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct CacheStats {
    pub vault_entries: u64,
    pub owner_entries: u64,
}
