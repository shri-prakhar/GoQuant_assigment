use chrono::Utc;
use shared::Vault;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::services::AppState;
pub struct VaultManager;

impl VaultManager {
    pub async fn get_vault(
        state: &AppState,
        vault_pubkey: &str,
    ) -> Result<Option<Vault>, VaultError> {
        if let Some(vault) = state.cache.get_vault(vault_pubkey).await {
            tracing::debug!("Cache HIT for vault {}", vault_pubkey);
            return Ok(Some(vault));
        }

        tracing::debug!("Cache MISS for vaults {}", vault_pubkey);

        let vault = state
            .database
            .get_vault(vault_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
        if let Some(ref v) = vault {
            state.cache.set_vault(v.clone()).await;
        }

        Ok(vault)
    }

    pub async fn get_vault_by_owner(
        state: &AppState,
        owner_pubkey: &str,
    ) -> Result<Option<Vault>, VaultError> {
        if let Some(vault_pubkey) = state.cache.get_vault_by_owner(owner_pubkey).await {
            return Self::get_vault(state, &vault_pubkey).await;
        }

        let vault = state
            .database
            .get_vault_by_owner(owner_pubkey)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        if let Some(ref v) = vault {
            state.cache.set_vault(v.clone()).await;
        }

        Ok(vault)
    }

    pub async fn sync_vault_from_chain(
        state: &AppState,
        vault_pubkey: &str,
    ) -> Result<Vault, VaultError> {
        let pubkey = Pubkey::from_str(vault_pubkey).map_err(|_| VaultError::InvalidPubkey)?;
        let account = state
            .solana_client
            .get_account(&pubkey)
            .map_err(|e| VaultError::SolanaRpcError(e.to_string()))?;

        let vault_data = Self::parse_vault_account(&account.data , vault_pubkey)?;
        state
            .database
            .upsert_vault(&vault_data)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
        state.cache.set_vault(vault_data.clone()).await;
        tracing::info!("Synced vault {} from chain", vault_pubkey);
        Ok(vault_data)
    }

    pub async fn initialize_vault(
        state: &AppState,
        vault_pubkey: &str,
        owner_pubkey: &str,
        token_account: &str,
    ) -> Result<Vault, VaultError> {
        let vault = Vault {
            vault_pubkey: vault_pubkey.to_string(),
            owner_pubkey: owner_pubkey.to_string(),
            token_account: token_account.to_string(),
            total_balance: 0,
            locked_balance: 0,
            available_balance: 0,
            total_deposited: 0,
            total_withdrawn: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        state
            .database
            .upsert_vault(&vault)
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        state.cache.set_vault(vault.clone()).await;

        tracing::info!(
            "Initialized vault {} for owner {}",
            vault_pubkey,
            owner_pubkey
        );

        Ok(vault)
    }

    pub async fn process_deposit(
        state: &AppState,
        vault_pubkey: &str,
        amount: i64,
        tx_signature: &str,
    ) -> Result<Vault, VaultError> {
        let mut vault = Self::get_vault(state, vault_pubkey)
            .await?
            .ok_or(VaultError::VaultNotFound)?;

        vault.total_balance += amount;
        vault.available_balance += amount;
        vault.total_deposited += amount;

        state
            .database
            .update_vault_balances(
                vault_pubkey,
                vault.total_balance,
                vault.locked_balance,
                Some(vault.total_deposited),
                None,
            )
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        state.cache.set_vault(vault.clone()).await;
        state
            .database
            .record_transaction(
                vault_pubkey,
                tx_signature,
                "deposit",
                amount,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
        tracing::info!("Processed deposit of {} to vault {}", amount, vault_pubkey);

        Ok(vault)
    }

    pub async fn process_withdrawal(
        state: &AppState,
        vault_pubkey: &str,
        amount: i64,
        tx_signature: &str,
    ) -> Result<Vault, VaultError> {
        let mut vault = Self::get_vault(state, vault_pubkey)
            .await?
            .ok_or(VaultError::VaultNotFound)?;

        if vault.available_balance < amount {
            return Err(VaultError::InsufficientBalance);
        }

        vault.total_balance -= amount;
        vault.available_balance -= amount;
        vault.total_withdrawn += amount;

        state
            .database
            .update_vault_balances(
                vault_pubkey,
                vault.total_balance,
                vault.locked_balance,
                None,
                Some(vault.total_withdrawn),
            )
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        state.cache.set_vault(vault.clone()).await;

        state
            .database
            .record_transaction(
                vault_pubkey,
                tx_signature,
                "withdraw",
                amount,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        tracing::info!(
            "Processed withdrawal of {} from vault {}",
            amount,
            vault_pubkey
        );

        Ok(vault)
    }

    pub async fn process_lock(
        state: &AppState,
        vault_pubkey: &str,
        amount: i64,
        tx_signature: &str,
    ) -> Result<Vault, VaultError> {
        let mut vault = Self::get_vault(state, vault_pubkey)
            .await?
            .ok_or(VaultError::VaultNotFound)?;
        if vault.available_balance < amount {
            return Err(VaultError::InsufficientBalance);
        }

        vault.locked_balance += amount;
        vault.available_balance -= amount;

        state
            .database
            .update_vault_balances(
                vault_pubkey,
                vault.total_balance,
                vault.locked_balance,
                None,
                None,
            )
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;
        state.cache.set_vault(vault.clone()).await;

        // Record transaction
        state
            .database
            .record_transaction(
                vault_pubkey,
                tx_signature,
                "lock",
                amount,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        tracing::info!("Locked {} collateral in vault {}", amount, vault_pubkey);

        Ok(vault)
    }

    pub async fn process_unlock(
        state: &AppState,
        vault_pubkey: &str,
        amount: i64,
        tx_signature: &str,
    ) -> Result<Vault, VaultError> {
        let mut vault = Self::get_vault(state, vault_pubkey)
            .await?
            .ok_or(VaultError::VaultNotFound)?;

        if vault.locked_balance < amount {
            return Err(VaultError::InsufficientLockedBalance);
        }

        vault.locked_balance -= amount;
        vault.available_balance += amount;

        state
            .database
            .update_vault_balances(
                vault_pubkey,
                vault.total_balance,
                vault.locked_balance,
                None,
                None,
            )
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        state.cache.set_vault(vault.clone()).await;

        state
            .database
            .record_transaction(
                vault_pubkey,
                tx_signature,
                "unlock",
                amount,
                None,
                None,
                "confirmed",
            )
            .await
            .map_err(|e| VaultError::DatabaseError(e.to_string()))?;

        tracing::info!("Unlocked {} collateral in vault {}", amount, vault_pubkey);

        Ok(vault)
    }
    fn parse_vault_account(data: &[u8], vault_pubkey: &str) -> Result<Vault, VaultError> {
        if data.len() < 8 {
            return Err(VaultError::DeserializationError(
                "Account data too short".to_string()
            ));
        }
        
        let vault_data = &data[8..];
        
        if vault_data.len() < 113 { // 32 + 32 + 8*5 + 8 + 1 = 113
            return Err(VaultError::DeserializationError(
                format!("Expected at least 113 bytes, got {}", vault_data.len())
            ));
        }
        fn read_pubkey(data: &[u8], offset: usize) -> Result<String, VaultError> {
            if data.len() < offset + 32 {
                return Err(VaultError::DeserializationError("Not enough data for pubkey".to_string()));
            }
            let pubkey_bytes: [u8; 32] = data[offset..offset+32]
                .try_into()
                .map_err(|_| VaultError::DeserializationError("Invalid pubkey bytes".to_string()))?;
            Ok(Pubkey::new_from_array(pubkey_bytes).to_string())
        }
        
        fn read_u64(data: &[u8], offset: usize) -> Result<u64, VaultError> {
            if data.len() < offset + 8 {
                return Err(VaultError::DeserializationError("Not enough data for u64".to_string()));
            }
            let bytes: [u8; 8] = data[offset..offset+8]
                .try_into()
                .map_err(|_| VaultError::DeserializationError("Invalid u64 bytes".to_string()))?;
            Ok(u64::from_le_bytes(bytes))
        }
        
        fn read_i64(data: &[u8], offset: usize) -> Result<i64, VaultError> {
            if data.len() < offset + 8 {
                return Err(VaultError::DeserializationError("Not enough data for i64".to_string()));
            }
            let bytes: [u8; 8] = data[offset..offset+8]
                .try_into()
                .map_err(|_| VaultError::DeserializationError("Invalid i64 bytes".to_string()))?;
            Ok(i64::from_le_bytes(bytes))
        }
        
        let mut offset = 0;
        
        let owner_pubkey = read_pubkey(vault_data, offset)?;
        offset += 32;
        
        let token_account = read_pubkey(vault_data, offset)?;
        offset += 32;
        
        let total_balance = read_u64(vault_data, offset)? as i64;
        offset += 8;
        
        let locked_balance = read_u64(vault_data, offset)? as i64;
        offset += 8;
        
        let available_balance = read_u64(vault_data, offset)? as i64;
        offset += 8;
        
        let total_deposited = read_u64(vault_data, offset)? as i64;
        offset += 8;
        
        let total_withdrawn = read_u64(vault_data, offset)? as i64;
        offset += 8;
        
        let created_at_unix = read_i64(vault_data, offset)?;
        
        let created_at = chrono::DateTime::from_timestamp(created_at_unix, 0)
            .ok_or(VaultError::DeserializationError("Invalid timestamp".to_string()))?;
        
        if total_balance != (available_balance + locked_balance) {
            tracing::warn!(
                "Balance invariant violation in vault {}: total={}, available={}, locked={}",
                vault_pubkey, total_balance, available_balance, locked_balance
            );
        }
        
        Ok(Vault {
            vault_pubkey: vault_pubkey.to_string(),
            owner_pubkey,
            token_account,
            total_balance,
            locked_balance,
            available_balance,
            total_deposited,
            total_withdrawn,
            created_at,
            updated_at: Utc::now(), // Use current time for updated_at
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Database error : {0}")]
    DatabaseError(String),
    #[error("Solana RPC Error : {0}")]
    SolanaRpcError(String),
    #[error("Invalid Pubkey Format")]
    InvalidPubkey,
    #[error("Vault not found")]
    VaultNotFound,
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Insufficient locked balance")]
    InsufficientLockedBalance,
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}
