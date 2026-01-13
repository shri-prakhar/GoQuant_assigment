use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub solana_rpc_url: String,
    pub program_id: Pubkey,
    pub max_db_connections: u32,
    pub cache_ttl_seconds: u32,
    pub reconciliation_interval_seconds: u64,
    pub monitoring_interval_seconds: u64,
}

impl Config {
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

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(&'static str),

    #[error("Invalid program ID: {0}")]
    InvalidProgramId(String),

    #[error("Invalid port number")]
    InvalidPort,

    #[error("Invalid number for {0}")]
    InvalidNumber(&'static str),
}
