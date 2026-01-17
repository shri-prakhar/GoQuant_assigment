//! # Configuration Module
//!
//! This module handles loading and validation of environment variables
//! for the Collateral Vault Backend Server.
//!
//! ## Environment Variables
//!
//! | Variable | Description | Default | Required |
//! |----------|-------------|---------|----------|
//! | `DATABASE_URL` | PostgreSQL connection string | - | Yes |
//! | `SOLANA_RPC_URL` | Solana RPC endpoint | `https://api.devnet.solana.com` | No |
//! | `PROGRAM_ID` | Deployed program ID | - | Yes |
//! | `HOST` | Server bind address | `0.0.0.0` | No |
//! | `PORT` | Server port | `3000` | No |
//! | `MAX_DB_CONNECTIONS` | Database connection pool size | `50` | No |
//! | `CACHE_TTL_SECONDS` | Cache TTL in seconds | `300` | No |
//! | `RECONCILIATION_INTERVAL_SECONDS` | Balance reconciliation interval | `3600` | No |
//! | `MONITORING_INTERVAL_SECONDS` | Monitoring interval | `60` | No |

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Server configuration loaded from environment variables
///
/// This struct contains all configuration values needed to run the server.
/// Use `Config::from_env()` to load from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Host address to bind the HTTP server to
    pub host: String,
    /// Port number for the HTTP server
    pub port: u16,
    /// PostgreSQL database connection URL
    pub database_url: String,
    /// Solana RPC endpoint URL
    pub solana_rpc_url: String,
    /// Public key of the deployed Anchor program
    pub program_id: Pubkey,
    /// Maximum number of database connections in the pool
    pub max_db_connections: u32,
    /// Time-to-live for cached data in seconds
    pub cache_ttl_seconds: u32,
    /// Interval between balance reconciliation runs in seconds
    pub reconciliation_interval_seconds: u64,
    /// Interval between monitoring checks in seconds
    pub monitoring_interval_seconds: u64,
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// # Panics
    ///
    /// Panics if required environment variables are missing or invalid:
    /// - `DATABASE_URL`: Must be a valid PostgreSQL connection string
    /// - `PROGRAM_ID`: Must be a valid Solana public key
    /// - `PORT`: Must be a valid port number (if set)
    /// - `MAX_DB_CONNECTIONS`: Must be a valid number (if set)
    /// - `CACHE_TTL_SECONDS`: Must be a valid number (if set)
    /// - `RECONCILIATION_INTERVAL_SECONDS`: Must be a valid number (if set)
    /// - `MONITORING_INTERVAL_SECONDS`: Must be a valid number (if set)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use backend::config::Config;
    ///
    /// // Set environment variables first
    /// std::env::set_var("DATABASE_URL", "postgresql://user:pass@localhost/db");
    /// std::env::set_var("PROGRAM_ID", "A9JDc7TrKR5Qyot3W3t6UQaRz4CTgEURemuSUkWfP9hs");
    ///
    /// let config = Config::from_env();
    /// assert_eq!(config.port, 3000); // default value
    /// ```
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingEnvVar("DATABASE_URL"))?;

        let solana_rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

        let program_id_str =
            std::env::var("PROGRAM_ID").map_err(|_| ConfigError::MissingEnvVar("PROGRAM_ID"))?;

        let program_id = Pubkey::from_str(&program_id_str)
            .map_err(|e| ConfigError::InvalidProgramId(e.to_string()))?;

        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|_| ConfigError::InvalidPort)?;

        let max_db_connections = std::env::var("MAX_DB_CONNECTIONS")
            .unwrap_or_else(|_| "50".to_string())
            .parse()
            .map_err(|_| ConfigError::InvalidNumber("MAX_DB_CONNECTIONS"))?;

        let cache_ttl_seconds = std::env::var("CACHE_TTL_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .map_err(|_| ConfigError::InvalidNumber("CACHE_TTL_SECONDS"))?;
        let reconciliation_interval_seconds = std::env::var("RECONCILIATION_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .map_err(|_| ConfigError::InvalidNumber("RECONCILIATION_INTERVAL_SECONDS"))?;

        let monitoring_interval_seconds = std::env::var("MONITORING_INTERVAL_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .map_err(|_| ConfigError::InvalidNumber("MONITORING_INTERVAL_SECONDS"))?;

        Ok(Config {
            host,
            port,
            database_url,
            solana_rpc_url,
            program_id,
            max_db_connections,
            cache_ttl_seconds,
            reconciliation_interval_seconds,
            monitoring_interval_seconds,
        })
    }
}

/// Configuration errors that can occur during loading
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A required environment variable is missing
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(&'static str),

    /// The program ID is not a valid Solana public key
    #[error("Invalid program ID: {0}")]
    InvalidProgramId(String),

    /// The port number is not valid
    #[error("Invalid port number")]
    InvalidPort,

    /// A numeric environment variable has an invalid value
    #[error("Invalid number for {0}")]
    InvalidNumber(&'static str),
}
