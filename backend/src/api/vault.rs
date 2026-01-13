use actix_web::{web, HttpResponse, Responder};
use shared::{
    ApiResponse, CreateVaultRequest, LockCollateralRequest, PaginationParams,
    ProcessDepositRequest, ProcessWithdrawalRequest, UnlockCollateralRequest,
};

use crate::services::{AppState, VaultManager};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/vault")
            .route("initialize", web::post().to(initialize_vault))
            .route("/balance/{vault_pubkey}", web::get().to(get_balance))
            .route("/owner/{owner_pubkey}", web::get().to(get_vault_by_owner))
            .route("/deposit", web::post().to(process_deposit))
            .route("/withdraw", web::post().to(process_withdrawal))
            .route("/lock", web::post().to(process_lock))
            .route("/unlock", web::post().to(process_unlock))
            .route("/sync/{vault_pubkey}", web::post().to(sync_vault))
            .route("/tvl", web::get().to(get_tvl))
            .route("/list", web::get().to(list_vaults)),
    );
}

async fn initialize_vault(
    state: web::Data<AppState>,
    req: web::Json<CreateVaultRequest>,
) -> impl Responder {
    tracing::info!("API: Initialize vault {}", req.vault_pubkey);

    match VaultManager::initialize_vault(
        &state,
        &req.vault_pubkey,
        &req.owner_pubkey,
        &req.token_account,
    )
    .await
    {
        Ok(vault) => HttpResponse::Ok().json(ApiResponse::success(vault)),
        Err(e) => {
            tracing::error!("Failed to initialize vault: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}

async fn get_balance(
    state: web::Data<AppState>,
    vault_pubkey: web::Path<String>,
) -> impl Responder {
    let start = std::time::Instant::now();

    match VaultManager::get_vault(&state, &vault_pubkey).await {
        Ok(Some(vault)) => {
            let elapsed = start.elapsed();
            tracing::debug!("Balance query took {:?}", elapsed);

            HttpResponse::Ok().json(ApiResponse::success(vault))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(ApiResponse::<()>::error("Vault not found".to_string()))
        }
        Err(e) => {
            tracing::error!("Failed to get vault balance: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}
async fn get_vault_by_owner(
    state: web::Data<AppState>,
    owner_pubkey: web::Path<String>,
) -> impl Responder {
    match VaultManager::get_vault_by_owner(&state, &owner_pubkey).await {
        Ok(Some(vault)) => HttpResponse::Ok().json(ApiResponse::success(vault)),
        Ok(None) => HttpResponse::NotFound().json(ApiResponse::<()>::error(
            "Vault not found for owner".to_string(),
        )),
        Err(e) => {
            tracing::error!("Failed to get vault by owner: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}

async fn process_deposit(
    state: web::Data<AppState>,
    req: web::Json<ProcessDepositRequest>,
) -> impl Responder {
    tracing::info!(
        "API: Process deposit {} to vault {}",
        req.amount,
        req.vault_pubkey
    );

    let start = std::time::Instant::now();

    match VaultManager::process_deposit(&state, &req.vault_pubkey, req.amount, &req.tx_signature)
        .await
    {
        Ok(vault) => {
            let elapsed = start.elapsed();
            tracing::info!("Deposit processed in {:?}", elapsed);

            HttpResponse::Ok().json(ApiResponse::success(vault))
        }
        Err(e) => {
            tracing::error!("Failed to process deposit: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}

async fn process_withdrawal(
    state: web::Data<AppState>,
    req: web::Json<ProcessWithdrawalRequest>,
) -> impl Responder {
    tracing::info!(
        "API: Process withdrawal {} from vault {}",
        req.amount,
        req.vault_pubkey
    );

    let start = std::time::Instant::now();

    match VaultManager::process_withdrawal(&state, &req.vault_pubkey, req.amount, &req.tx_signature)
        .await
    {
        Ok(vault) => {
            let elapsed = start.elapsed();
            tracing::info!("Withdrawal processed in {:?}", elapsed);

            HttpResponse::Ok().json(ApiResponse::success(vault))
        }
        Err(e) => {
            tracing::error!("Failed to process withdrawal: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}

async fn process_lock(
    state: web::Data<AppState>,
    req: web::Json<LockCollateralRequest>,
) -> impl Responder {
    tracing::info!(
        "API: Process lock {} in vault {}",
        req.amount,
        req.vault_pubkey
    );

    match VaultManager::process_lock(&state, &req.vault_pubkey, req.amount, &req.tx_signature).await
    {
        Ok(vault) => HttpResponse::Ok().json(ApiResponse::success(vault)),
        Err(e) => {
            tracing::error!("Failed to process lock: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}

async fn process_unlock(
    state: web::Data<AppState>,
    req: web::Json<UnlockCollateralRequest>,
) -> impl Responder {
    tracing::info!(
        "API: Process unlock {} in vault {}",
        req.amount,
        req.vault_pubkey
    );

    match VaultManager::process_unlock(&state, &req.vault_pubkey, req.amount, &req.tx_signature)
        .await
    {
        Ok(vault) => HttpResponse::Ok().json(ApiResponse::success(vault)),
        Err(e) => {
            tracing::error!("Failed to process unlock: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}

async fn sync_vault(state: web::Data<AppState>, vault_pubkey: web::Path<String>) -> impl Responder {
    tracing::info!("API: Sync vault {}", vault_pubkey);

    match VaultManager::sync_vault_from_chain(&state, &vault_pubkey).await {
        Ok(vault) => HttpResponse::Ok().json(ApiResponse::success(vault)),
        Err(e) => {
            tracing::error!("Failed to sync vault: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}

async fn get_tvl(state: web::Data<AppState>) -> impl Responder {
    if let Some(stats) = state.cache.get_tvl_stats().await {
        return HttpResponse::Ok().json(ApiResponse::success(stats));
    }

    // Cache miss - query database
    match state.database.get_tvl_stats().await {
        Ok(stats) => {
            // Update cache
            state.cache.set_tvl_stats(stats.clone()).await;
            HttpResponse::Ok().json(ApiResponse::success(stats))
        }
        Err(e) => {
            tracing::error!("Failed to get TVL stats: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}

async fn list_vaults(
    state: web::Data<AppState>,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    match state
        .database
        .get_all_vaults(query.limit, query.offset)
        .await
    {
        Ok(vaults) => HttpResponse::Ok().json(ApiResponse::success(vaults)),
        Err(e) => {
            tracing::error!("Failed to list vaults: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}
