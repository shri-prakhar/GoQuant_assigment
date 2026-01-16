use std::sync::Arc;
use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use solana_client::rpc_client::RpcClient;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod cache;
mod config;
mod database;
mod monitering;
mod services;
mod websocket;
mod api_tests;

use config::Config;

use crate::{cache::Cache, database::Database, services::{event_listner, vault_moniter}};

#[actix_web::main]
async fn main() -> Result<(), std::io::Error>{
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(" Starting Collateral Vault Management System Backend");

    dotenv::dotenv().ok();
    let config = Config::from_env().expect("Failed to load configuration");
    tracing::info!("Configuration loaded");
    let database = Database::new(&config.database_url)
        .await
        .expect("Failed to connect to database");
    tracing::info!(" Database connected");
    database
        .run_migrations()
        .await
        .expect("Failed to run migrations");
    tracing::info!("Database migrations completed");

    let cache = Cache::new(20_000);
    tracing::info!(" Cache initialized with 20,000 entry capacity");

    let solana_client = RpcClient::new(config.solana_rpc_url.clone());
    tracing::info!("Solana RPC client initialized: {}", config.solana_rpc_url);

    let app_state = web::Data::new(services::AppState {
        database: database.clone(),
        cache: cache.clone(),
        config: config.clone(),
        solana_client: Arc::new(solana_client),
        program_id: config.program_id,
    });

    let monitor_state = app_state.clone();
    tokio::spawn(async move {
        vault_moniter::run_monitor(monitor_state).await;
    });

    let reconcile_state = app_state.clone();
    tokio::spawn(async move {
        services::balance_reconciler::run_reconciler(reconcile_state).await;
    });
    let event_listener_state = app_state.clone();
    tokio::spawn(async move {
        tracing::info!("🎧 Spawning Event Listener background task...");
        event_listner::run_event_listener(event_listener_state).await;
    });
    tracing::info!("Background services started (monitor, reconciler)");

    let bind_address = format!("{}:{}", config.host, config.port);
    tracing::info!("Server listening on http://{}", bind_address);
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(cors)
            .route("/health", web::get().to(api::health::health_check))
            .route("/metrics", web::get().to(monitering::metrics::metrics))
            .route("/ws", web::get().to(websocket::ws_handler))
            .service(
                web::scope("/api/v1")
                    .configure(api::vault::configure)
                    .configure(api::transaction::configure),
            )
    })
    .workers(num_cpus::get() * 2)
    .bind(bind_address)?
    .run()
    .await
}
