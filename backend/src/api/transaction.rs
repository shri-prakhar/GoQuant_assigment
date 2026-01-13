use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use shared::ApiResponse;

use crate::services::AppState;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/transactions").route("/{vault_pubkey}", web::get().to(get_transactions)),
    );
}

#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

async fn get_transactions(
    state: web::Data<AppState>,
    vault_pubkey: web::Path<String>,
    query: web::Query<TransactionQuery>,
) -> impl Responder {
    match state
        .database
        .get_vault_transactions(&vault_pubkey, query.limit)
        .await
    {
        Ok(transactions) => HttpResponse::Ok().json(ApiResponse::success(transactions)),
        Err(e) => {
            tracing::error!("Failed to get transactions: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e.to_string()))
        }
    }
}
