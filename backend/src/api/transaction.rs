use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
};
use std::str::FromStr;

use crate::services::{AppState, TransactionBuilder};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BuildDepositTxRequest {
    pub user_pubkey: String,
    pub user_token_account: String,
    pub vault_token_account: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct BuildWithdrawTxRequest {
    pub user_pubkey: String,
    pub vault_pubkey: String,
    pub vault_token_account: String,
    pub user_token_account: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct BuildInitializeTxRequest {
    pub user_pubkey: String,
    pub mint_pubkey: String,
}

#[derive(Debug, Serialize)]
pub struct UnsignedTransactionResponse {
    /// Base64-encoded serialized transaction (unsigned)
    pub transaction: String,
    /// The blockhash used (for reference)
    pub blockhash: String,
    /// Estimated fee in lamports
    pub estimated_fee: u64,
    /// Accounts that need to sign
    pub signers: Vec<String>,
    /// Message for the user
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct TransactionHistoryResponse {
    pub transactions: Vec<TransactionRecord>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct TransactionRecord {
    pub id: i64,
    pub vault_pubkey: String,
    pub tx_signature: String,
    pub tx_type: String,
    pub amount: i64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionHistoryQuery {
    pub vault_pubkey: Option<String>,
    pub tx_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// ============================================================================
// Route Configuration
// ============================================================================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/transaction")
            // Build unsigned transactions (Frontend will sign these)
            .route("/build/deposit", web::post().to(build_deposit_transaction))
            .route("/build/withdraw", web::post().to(build_withdraw_transaction))
            .route("/build/initialize", web::post().to(build_initialize_transaction))
            // Transaction history
            .route("/history", web::get().to(get_transaction_history))
            .route("/history/{vault_pubkey}", web::get().to(get_vault_transactions))
            .route("/{tx_signature}", web::get().to(get_transaction))
    );
}

// ============================================================================
// Build Unsigned Transaction Endpoints
// These are critical for your architecture:
// 1. Frontend requests unsigned transaction
// 2. Backend builds it with correct accounts
// 3. Frontend presents to wallet for signing
// 4. Wallet submits to Solana
// ============================================================================

/// Build an unsigned deposit transaction
/// 
/// # Flow:
/// 1. Frontend calls this with user's pubkeys and amount
/// 2. Backend builds SPL Token transfer instruction
/// 3. Returns base64-encoded unsigned transaction
/// 4. Frontend deserializes and shows to wallet
/// 5. User signs with Phantom/Solflare
/// 6. Wallet submits to Solana
async fn build_deposit_transaction(
    state: web::Data<AppState>,
    req: web::Json<BuildDepositTxRequest>,
) -> impl Responder {
    tracing::info!(
        "API: Build deposit transaction - user: {}, amount: {}",
        req.user_pubkey,
        req.amount
    );

    // Validate pubkeys
    let user_pubkey = match Pubkey::from_str(&req.user_pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid user pubkey".to_string()));
        }
    };

    let user_token_account = match Pubkey::from_str(&req.user_token_account) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid user token account".to_string()));
        }
    };

    let vault_token_account = match Pubkey::from_str(&req.vault_token_account) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid vault token account".to_string()));
        }
    };

    // Get recent blockhash
    let recent_blockhash = match state.solana_client.get_latest_blockhash().await {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("Failed to get recent blockhash: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get recent blockhash".to_string()));
        }
    };

    // Build the unsigned transaction
    let transaction = match TransactionBuilder::build_deposit_tx(
        &user_pubkey,
        &user_token_account,
        &vault_token_account,
        req.amount,
        recent_blockhash,
    ) {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to build deposit transaction: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(format!("Failed to build transaction: {}", e)));
        }
    };

    // Serialize transaction to base64
    let serialized = match serde_json::to_string(&transaction) {
        Ok(json_str) => base64::encode(json_str.as_bytes()),
        Err(e) => {
            tracing::error!("Failed to serialize transaction: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to serialize transaction".to_string()));
        }
    };

    // Estimate fee (5000 lamports per signature is a reasonable estimate)
    let estimated_fee = TransactionBuilder::estimate_fee(&transaction, 5000);

    let response = UnsignedTransactionResponse {
        transaction: serialized,
        blockhash: recent_blockhash.to_string(),
        estimated_fee,
        signers: vec![req.user_pubkey.clone()],
        message: format!("Deposit {} tokens to vault", req.amount),
    };

    tracing::info!("Built unsigned deposit transaction for user {}", req.user_pubkey);
    HttpResponse::Ok().json(ApiResponse::success(response))
}

/// Build an unsigned withdrawal transaction
async fn build_withdraw_transaction(
    state: web::Data<AppState>,
    req: web::Json<BuildWithdrawTxRequest>,
) -> impl Responder {
    tracing::info!(
        "API: Build withdraw transaction - user: {}, amount: {}",
        req.user_pubkey,
        req.amount
    );

    // Validate pubkeys
    let user_pubkey = match Pubkey::from_str(&req.user_pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid user pubkey".to_string()));
        }
    };

    let vault_pubkey = match Pubkey::from_str(&req.vault_pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid vault pubkey".to_string()));
        }
    };

    let vault_token_account = match Pubkey::from_str(&req.vault_token_account) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid vault token account".to_string()));
        }
    };

    let user_token_account = match Pubkey::from_str(&req.user_token_account) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid user token account".to_string()));
        }
    };

    // Verify vault exists and has sufficient balance
    match state.database.get_vault(&req.vault_pubkey).await {
        Ok(Some(vault)) => {
            if vault.available_balance < req.amount as i64 {
                return HttpResponse::BadRequest()
                    .json(ApiResponse::<()>::error("Insufficient available balance".to_string()));
            }
        }
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(ApiResponse::<()>::error("Vault not found".to_string()));
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Database error".to_string()));
        }
    }

    // Get recent blockhash
    let recent_blockhash = match state.solana_client.get_latest_blockhash().await {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("Failed to get recent blockhash: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get recent blockhash".to_string()));
        }
    };

    // Build the unsigned transaction
    let transaction = match TransactionBuilder::build_withdraw_tx(
        &user_pubkey,
        &vault_pubkey,
        &vault_token_account,
        &user_token_account,
        req.amount,
        recent_blockhash,
    ) {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to build withdraw transaction: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(format!("Failed to build transaction: {}", e)));
        }
    };

    // Serialize transaction to base64
    let serialized = match serde_json::to_string(&transaction) {
        Ok(json_str) => base64::encode(json_str.as_bytes()),
        Err(e) => {
            tracing::error!("Failed to serialize transaction: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to serialize transaction".to_string()));
        }
    };

    let estimated_fee = TransactionBuilder::estimate_fee(&transaction, 5000);

    let response = UnsignedTransactionResponse {
        transaction: serialized,
        blockhash: recent_blockhash.to_string(),
        estimated_fee,
        signers: vec![req.user_pubkey.clone()],
        message: format!("Withdraw {} tokens from vault", req.amount),
    };

    tracing::info!("Built unsigned withdraw transaction for user {}", req.user_pubkey);
    HttpResponse::Ok().json(ApiResponse::success(response))
}

/// Build an unsigned vault initialization transaction
async fn build_initialize_transaction(
    state: web::Data<AppState>,
    req: web::Json<BuildInitializeTxRequest>,
) -> impl Responder {
    tracing::info!(
        "API: Build initialize vault transaction - user: {}",
        req.user_pubkey
    );

    let user_pubkey = match Pubkey::from_str(&req.user_pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid user pubkey".to_string()));
        }
    };

    let mint_pubkey = match Pubkey::from_str(&req.mint_pubkey) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("Invalid mint pubkey".to_string()));
        }
    };

    // Derive the vault PDA
    let (vault_pda, _bump) = Pubkey::find_program_address(
        &[b"vault", user_pubkey.as_ref()],
        &state.program_id,
    );

    // Check if vault already exists
    match state.database.get_vault(&vault_pda.to_string()).await {
        Ok(Some(_)) => {
            return HttpResponse::Conflict()
                .json(ApiResponse::<()>::error("Vault already exists for this user".to_string()));
        }
        Ok(None) => { /* Good, vault doesn't exist */ }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Database error".to_string()));
        }
    }

    // Get recent blockhash
    let recent_blockhash = match state.solana_client.get_latest_blockhash().await {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("Failed to get recent blockhash: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get recent blockhash".to_string()));
        }
    };

    // Build initialize instruction
    // Note: This would need to call your Anchor program's initialize instruction
    // For now, returning the vault PDA info
    let response = UnsignedTransactionResponse {
        transaction: "".to_string(), // Would be filled with actual instruction
        blockhash: recent_blockhash.to_string(),
        estimated_fee: 10000, // Rent + fees
        signers: vec![req.user_pubkey.clone()],
        message: format!("Initialize vault at {}", vault_pda),
    };

    HttpResponse::Ok().json(ApiResponse::success(response))
}

// ============================================================================
// Transaction History Endpoints
// ============================================================================

/// Get transaction history with optional filters
async fn get_transaction_history(
    state: web::Data<AppState>,
    query: web::Query<TransactionHistoryQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    match state.database.get_transactions(
        query.vault_pubkey.as_deref(),
        query.tx_type.as_deref(),
        limit,
        offset,
    ).await {
        Ok(transactions) => {
            let records: Vec<TransactionRecord> = transactions
                .into_iter()
                .map(|t| TransactionRecord {
                    id: t.id,
                    vault_pubkey: t.vault_pubkey,
                    tx_signature: t.tx_signature,
                    tx_type: t.tx_type,
                    amount: t.amount,
                    status: t.status,
                    created_at: t.created_at.to_rfc3339(),
                })
                .collect();

            let response = TransactionHistoryResponse {
                transactions: records,
                total: 0, // Would need a count query
                limit,
                offset,
            };

            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => {
            tracing::error!("Failed to get transaction history: {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get transaction history".to_string()))
        }
    }
}

/// Get transactions for a specific vault
async fn get_vault_transactions(
    state: web::Data<AppState>,
    vault_pubkey: web::Path<String>,
    query: web::Query<TransactionHistoryQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    match state.database.get_vault_transactions(&vault_pubkey, limit).await {
        Ok(transactions) => {
            let records: Vec<TransactionRecord> = transactions
                .into_iter()
                .map(|t| TransactionRecord {
                    id: t.id,
                    vault_pubkey: t.vault_pubkey,
                    tx_signature: t.tx_signature,
                    tx_type: t.tx_type,
                    amount: t.amount,
                    status: t.status,
                    created_at: t.created_at.to_rfc3339(),
                })
                .collect();

            let response = TransactionHistoryResponse {
                transactions: records,
                total: 0,
                limit,
                offset,
            };

            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => {
            tracing::error!("Failed to get vault transactions: {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get vault transactions".to_string()))
        }
    }
}

/// Get a specific transaction by signature
async fn get_transaction(
    state: web::Data<AppState>,
    tx_signature: web::Path<String>,
) -> impl Responder {
    match state.database.get_transaction_by_signature(&tx_signature).await {
        Ok(Some(t)) => {
            let record = TransactionRecord {
                id: t.id,
                vault_pubkey: t.vault_pubkey,
                tx_signature: t.tx_signature,
                tx_type: t.tx_type,
                amount: t.amount,
                status: t.status,
                created_at: t.created_at.to_rfc3339(),
            };
            HttpResponse::Ok().json(ApiResponse::success(record))
        }
        Ok(None) => {
            HttpResponse::NotFound()
                .json(ApiResponse::<()>::error("Transaction not found".to_string()))
        }
        Err(e) => {
            tracing::error!("Failed to get transaction: {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get transaction".to_string()))
        }
    }
}

// Add base64 encoding helper
mod base64 {
    pub fn encode(data: &[u8]) -> String {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        STANDARD.encode(data)
    }
}