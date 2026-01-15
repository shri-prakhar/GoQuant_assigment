use chrono::Utc;
use shared::{
    Alert, AuditTrailEntry, BalanceSnapshot, ReconciliationLog, TransactionRecord, TvlStats, Vault,
};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(100)
            .min_connections(10)
            .acquire_timeout(Duration::from_secs(3))
            .idle_timeout(Duration::from_secs(600))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }
    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("../migrations").run(&self.pool).await?;
        Ok(())
    }

    pub async fn upsert_vault(&self, vault: &Vault) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        INSERT INTO vaults(
          vault_pubkey, owner_pubkey, token_account,
          total_balance, locked_balance, total_deposited, total_withdrawn, created_at
        ) VALUES ($1 , $2 , $3 , $4 , $5 ,$6 , $7 , $8)
         ON CONFLICT (vault_pubkey)
         DO UPDATE SET
                total_balance = EXCLUDED.total_balance,
                locked_balance = EXCLUDED.locked_balance,
                total_deposited = EXCLUDED.total_deposited,
                total_withdrawn = EXCLUDED.total_withdrawn,
                updated_at = NOW()     
      "#,
        )
        .bind(&vault.vault_pubkey)
        .bind(&vault.owner_pubkey)
        .bind(&vault.token_account)
        .bind(&vault.total_balance)
        .bind(vault.locked_balance)
        .bind(vault.total_deposited)
        .bind(vault.total_withdrawn)
        .bind(&vault.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_vault(&self, vault_pubkey: &str) -> Result<Option<Vault>, sqlx::Error> {
        let vault = sqlx::query_as::<_, Vault>(
            "
            SELECT * FROM vaults WHERE vault_pubkey = $1
          ",
        )
        .bind(vault_pubkey)
        .fetch_optional(&self.pool)
        .await?;

        Ok(vault)
    }

    pub async fn get_vault_by_owner(
        &self,
        owner_pubkey: &str,
    ) -> Result<Option<Vault>, sqlx::Error> {
        let vault = sqlx::query_as(" SELECT * FROM vaults WHERE owner_pubkey = $1")
            .bind(owner_pubkey)
            .fetch_optional(&self.pool)
            .await?;

        Ok(vault)
    }

    pub async fn get_all_vaults(&self, limit: i64, offset: i64) -> Result<Vec<Vault>, sqlx::Error> {
        let vaults =
            sqlx::query_as("SELECT * FROM vaults ORDER BY created_at DESC LIMIT $1 OFFSET $2")
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

        Ok(vaults)
    }

    pub async fn get_vault_count(&self) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM vaults")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    pub async fn update_vault_balances(
        &self,
        vault_pubkey: &str,
        total_balance: i64,
        locked_balance: i64,
        total_deposited: Option<i64>,
        total_withdrawn: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        let mut query = String::from("UPDATE vaults SET total_balance=$1 , locked_balance=$2");
        let mut param_count = 3;

        if total_deposited.is_some() {
            query.push_str(&format!(", total_deposited = ${}", param_count));
        }
        if total_withdrawn.is_some() {
            query.push_str(&format!(", total_withdrawn = ${}", param_count));
            param_count += 1;
        }

        query.push_str(&format!(
            ", updated_at = NOW() WHERE vault_pubkey = ${}",
            param_count
        ));

        let mut q = sqlx::query(&query).bind(total_balance).bind(locked_balance);

        if let Some(deposited) = total_deposited {
            q = q.bind(deposited)
        }
        if let Some(withdrawn) = total_withdrawn {
            q = q.bind(withdrawn);
        }

        q = q.bind(vault_pubkey);
        q.execute(&self.pool).await?;

        Ok(())
    }

    pub async fn record_transaction(
        &self,
        vault_pubkey: &str,
        tx_signature: &str,
        tx_type: &str,
        amount: i64,
        from_vault: Option<&str>,
        to_vault: Option<&str>,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO transactions (
                vault_pubkey, tx_signature, tx_type, amount,
                from_vault, to_vault, status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (tx_signature) DO NOTHING
            "#,
        )
        .bind(vault_pubkey)
        .bind(tx_signature)
        .bind(tx_type)
        .bind(amount)
        .bind(from_vault)
        .bind(to_vault)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_transaction_status(
        &self,
        tx_signature: &str,
        status: &str,
        block_time: Option<i64>,
        slot: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE transactions 
            SET status = $1, block_time = $2, slot = $3, confirmed_at = NOW()
            WHERE tx_signature = $4
            "#,
        )
        .bind(status)
        .bind(block_time)
        .bind(slot)
        .bind(tx_signature)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_transactions(
        &self,
        vault_pubkey: Option<&str>,
        tx_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TransactionRecord>, sqlx::Error> {
        let mut query = "SELECT id, vault_pubkey, tx_signature, tx_type, amount, status, created_at FROM transactions WHERE 1=1".to_string();
        let mut param_count = 0;

        if vault_pubkey.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND vault_pubkey = ${}", param_count));
        }

        if tx_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND tx_type = ${}", param_count));
        }

        param_count += 1;
        query.push_str(&format!(" ORDER BY created_at DESC LIMIT ${}", param_count));
        param_count += 1;
        query.push_str(&format!(" OFFSET ${}", param_count));

        let mut q = sqlx::query_as::<_, TransactionRecord>(&query);

        if let Some(vault) = vault_pubkey {
            q = q.bind(vault);
        }

        if let Some(tx_type_val) = tx_type {
            q = q.bind(tx_type_val);
        }

        q = q.bind(limit);
        q = q.bind(offset);

        q.fetch_all(&self.pool).await
    }

    pub async fn get_vault_transactions(
        &self,
        vault_pubkey: &str,
        limit: i64,
    ) -> Result<Vec<TransactionRecord>, sqlx::Error> {
        let transactions = sqlx::query_as::<_, TransactionRecord>(
            r#"
            SELECT * FROM transactions 
            WHERE vault_pubkey = $1 
            ORDER BY created_at DESC 
            LIMIT $2
            "#,
        )
        .bind(vault_pubkey)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(transactions)
    }
    pub async fn get_transaction_by_signature(
        &self,
        tx_signature: &str,
    ) -> Result<Option<TransactionRecord>, sqlx::Error> {
        sqlx::query_as!(
            TransactionRecord,
            "SELECT * FROM transactions WHERE tx_signature = $1",
            tx_signature
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create_balance_snapshot(
        &self,
        vault_pubkey: &str,
        total_balance: i64,
        locked_balance: i64,
        available_balance: i64,
        on_chain_token_balance: i64,
        snapshot_type: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO balance_snapshots (
                vault_pubkey, total_balance, locked_balance, available_balance,
                on_chain_token_balance, snapshot_type
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(vault_pubkey)
        .bind(total_balance)
        .bind(locked_balance)
        .bind(available_balance)
        .bind(on_chain_token_balance)
        .bind(snapshot_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_latest_snapshot(
        &self,
        vault_pubkey: &str,
    ) -> Result<Option<BalanceSnapshot>, sqlx::Error> {
        let snapshot = sqlx::query_as::<_, BalanceSnapshot>(
            r#"
            SELECT * FROM balance_snapshots 
            WHERE vault_pubkey = $1 
            ORDER BY snapshot_ts DESC 
            LIMIT 1
            "#,
        )
        .bind(vault_pubkey)
        .fetch_optional(&self.pool)
        .await?;

        Ok(snapshot)
    }

    pub async fn log_reconciliation_issue(
        &self,
        vault_pubkey: &str,
        expected_balance: i64,
        actual_balance: i64,
        discrepancy: i64,
    ) -> Result<i64, sqlx::Error> {
        let rec = sqlx::query(
            r#"
            INSERT INTO reconciliation_logs (
                vault_pubkey, expected_balance, actual_balance, discrepancy
            ) VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(vault_pubkey)
        .bind(expected_balance)
        .bind(actual_balance)
        .bind(discrepancy)
        .fetch_one(&self.pool)
        .await?;

        Ok(rec.get("id"))
    }

    pub async fn resolve_reconciliation(
        &self,
        id: i64,
        resolution_notes: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE reconciliation_logs 
            SET resolution_status = 'resolved', 
                resolution_notes = $1, 
                resolved_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(resolution_notes)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_unresolved_reconciliations(
        &self,
        limit: i64,
    ) -> Result<Vec<ReconciliationLog>, sqlx::Error> {
        let logs = sqlx::query_as::<_, ReconciliationLog>(
            r#"
            SELECT * FROM reconciliation_logs 
            WHERE resolution_status != 'resolved'
            ORDER BY detected_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    pub async fn create_alert(
        &self,
        alert_type: &str,
        severity: &str,
        vault_pubkey: Option<&str>,
        message: &str,
        details: Option<serde_json::Value>,
    ) -> Result<i64, sqlx::Error> {
        let alert = sqlx::query(
            r#"
            INSERT INTO alerts (alert_type, severity, vault_pubkey, message, details)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(alert_type)
        .bind(severity)
        .bind(vault_pubkey)
        .bind(message)
        .bind(details)
        .fetch_one(&self.pool)
        .await?;

        Ok(alert.get("id"))
    }

    pub async fn get_active_alerts(&self, limit: i64) -> Result<Vec<Alert>, sqlx::Error> {
        let alerts = sqlx::query_as::<_, Alert>(
            r#"
            SELECT * FROM alerts 
            WHERE status = 'active' 
            ORDER BY created_at DESC 
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(alerts)
    }
    pub async fn acknowledge_alert(&self, alert_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE alerts 
            SET status = 'acknowledged', acknowledged_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(alert_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn resolve_alert(&self, alert_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE alerts 
            SET status = 'resolved', resolved_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(alert_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn create_audit_entry(
        &self,
        event_type: &str,
        vault_pubkey: Option<&str>,
        user_pubkey: Option<&str>,
        amount: Option<i64>,
        tx_signature: Option<&str>,
        event_data: serde_json::Value,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        let entry = sqlx::query(
            r#"
            INSERT INTO audit_trail (
                event_type, vault_pubkey, user_pubkey, amount, 
                tx_signature, event_data, ip_address, user_agent
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(event_type)
        .bind(vault_pubkey)
        .bind(user_pubkey)
        .bind(amount)
        .bind(tx_signature)
        .bind(event_data)
        .bind(ip_address)
        .bind(user_agent)
        .fetch_one(&self.pool)
        .await?;

        Ok(entry.get("id"))
    }

    pub async fn get_vault_audit_trail(
        &self,
        vault_pubkey: &str,
        limit: i64,
    ) -> Result<Vec<AuditTrailEntry>, sqlx::Error> {
        let entries = sqlx::query_as::<_, AuditTrailEntry>(
            r#"
            SELECT * FROM audit_trail 
            WHERE vault_pubkey = $1 
            ORDER BY created_at DESC 
            LIMIT $2
            "#,
        )
        .bind(vault_pubkey)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    pub async fn get_tvl_stats(&self) -> Result<TvlStats, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM tvl_stats")
            .fetch_one(&self.pool)
            .await?;

        Ok(TvlStats {
            total_vaults: row.get("total_vaults"),
            total_value_locked: row.get("total_value_locked"),
            total_locked: row.get("total_available"),
            total_available: row.get("total_locked"),
            avg_vault_balance: row.get("avg_vault_balance"),
            max_vault_balance: row.get("max_vault_balance"),
            timestamp: Utc::now(),
        })
    }
}
