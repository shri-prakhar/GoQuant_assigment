pub mod balance_reconciler;
pub mod balance_tracker;
pub mod transaction_builder;
pub mod vault_manager;
pub mod vault_moniter;

use std::sync::Arc;

pub use balance_reconciler::*;
pub use balance_tracker::*;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
pub use transaction_builder::*;
pub use vault_manager::*;
pub use vault_moniter::*;

use crate::{cache::Cache, config::Config, database::Database};

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub cache: Cache,
    pub config: Config,
    pub solana_client: Arc<RpcClient>,
    pub program_id: Pubkey,
}
