//! # Collateral Vault Backend Server
//!
//! This is the main entry point for the Collateral Vault Management System backend.
//! It provides a REST API for vault operations, real-time WebSocket updates,
//! and maintains synchronization with the Solana blockchain.
//!
//! ## Features
//!
//! - REST API for vault management (deposit, withdraw, lock, unlock)
//! - Real-time WebSocket notifications for vault updates
//! - PostgreSQL database for persistent storage
//! - Redis-like caching for performance
//! - Event listener for on-chain transaction monitoring
//! - Balance reconciliation and monitoring services
//! - Health checks and metrics endpoints
//!
//! ## Architecture
//!
//! The server initializes several key components:
//!
//! 1. **Configuration**: Loads environment variables and settings
//! 2. **Database**: PostgreSQL connection with migrations
//! 3. **Cache**: In-memory cache for vault data
//! 4. **Solana Client**: RPC connection to Solana network
//! 5. **Background Services**:
//!    - Vault monitor for periodic health checks
//!    - Balance reconciler for on-chain/off-chain sync
//!    - Event listener for real-time blockchain events
//! 6. **HTTP Server**: Actix-web server with CORS, logging, compression
//!
//! ## API Endpoints
//!
//! - `GET /health` - Health check
//! - `GET /metrics` - Prometheus metrics
//! - `GET /ws` - WebSocket connection
//! - `/api/v1/vault/*` - Vault operations
//! - `/api/v1/transaction/*` - Transaction building

use std::{sync::Arc, time::Duration};
use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use solana_client::nonblocking::rpc_client::RpcClient as AsyncRpcClient;
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

/// Main entry point for the Collateral Vault Backend Server
///
/// Initializes all components in order:
/// 1. Logging and tracing
/// 2. Environment configuration
/// 3. Database connection and migrations
/// 4. Cache initialization
/// 5. Solana RPC client
/// 6. Background services (monitor, reconciler, event listener)
/// 7. HTTP server with routes
///
/// # Panics
///
/// Panics if:
/// - Configuration loading fails
/// - Database connection fails
/// - Database migrations fail
/// - HTTP server binding fails
///
/// # Environment Variables
///
/// See `Config::from_env()` for required environment variables.
#[actix_web::main]
async fn main() -> Result<(), std::io::Error>{
    // Initialize tracing with default level filters
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=debug,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(" Starting Collateral Vault Management System Backend");

    // Load environment variables from .env file if present
    dotenv::dotenv().ok();

    // Load and validate configuration
    let config = Config::from_env().expect("Failed to load configuration");
    tracing::info!(" Configuration loaded");

    // Initialize database connection and run migrations
    let database = Database::new(&config.database_url)
        .await
        .expect("Failed to connect to database");
    tracing::info!("  Database connected");
    database
        .run_migrations()
        .await
        .expect("Failed to run migrations");
    tracing::info!(" Database migrations completed");
    match database.cleanup_invalid_vaults().await {
    Ok(count) if count > 0 => {
        tracing::info!("🧹 Cleaned up {} invalid test vaults", count);
    }
    Ok(_) => {
        tracing::debug!("No invalid vaults to clean up");
    }
    Err(e) => {
        tracing::warn!("Failed to cleanup invalid vaults: {}", e);
    }
}
    // Initialize cache with specified capacity
    let cache = Cache::new(20_000);
    tracing::info!(" Cache initialized with 20,000 entry capacity");

    // Initialize Solana RPC client
    let solana_client = AsyncRpcClient::new(config.solana_rpc_url.clone());
    tracing::info!(" Solana RPC client initialized: {}", config.solana_rpc_url);

    // Create shared application state
    let app_state = web::Data::new(services::AppState {
        database: database.clone(),
        cache: cache.clone(),
        config: config.clone(),
        solana_client: Arc::new(solana_client),
        program_id: config.program_id,
    });

    // Start background services

    // Vault monitor - periodic health checks and maintenance
    let monitor_state = app_state.clone();
    tokio::spawn(async move {
        vault_moniter::run_monitor(monitor_state).await;
    });

    // Balance reconciler - sync on-chain and off-chain balances
    let reconcile_state = app_state.clone();
    tokio::spawn(async move {
        services::balance_reconciler::run_reconciler(reconcile_state).await;
    });

    // Event listener - monitor blockchain for vault events
    let event_listener_state = app_state.clone();
    tokio::spawn(async move {
        loop {
            let state = event_listener_state.clone();
            match tokio::spawn(async move {
                event_listner::run_event_listener(state).await;
            }).await {
                Ok(_) => tracing::warn!("Event listener exited, restarting..."),
                Err(e) => tracing::error!("Event listener panicked: {:?}", e),
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    tracing::info!(" Background services started (monitor, reconciler, event listener)");

    // Configure and start HTTP server
    let bind_address = format!("{}:{}", config.host, config.port);
    tracing::info!(" Server listening on http://{}", bind_address);

    HttpServer::new(move || {
        // Configure CORS for cross-origin requests
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            // Request logging middleware
            .wrap(middleware::Logger::default())
            // Response compression
            .wrap(middleware::Compress::default())
            // Tracing integration
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(cors)
            // Health check endpoint
            .route("/health", web::get().to(api::health::health_check))
            // Metrics endpoint for monitoring
            .route("/metrics", web::get().to(monitering::metrics::metrics))
            // WebSocket endpoint for real-time updates
            .route("/ws", web::get().to(websocket::ws_handler))
            // API v1 routes
            .service(
                web::scope("/api/v1")
                    .configure(api::vault::configure)
                    .configure(api::transaction::configure),
            )
    })
    // Configure worker threads (2x CPU cores for optimal performance)
    .workers(num_cpus::get() * 2)
    .bind(bind_address)?
    .run()
    .await
}
