use std::str::FromStr;

use solana_sdk::{message::{AccountMeta, Instruction}, pubkey::Pubkey, transaction::Transaction};

use crate::services::AppState;

pub struct CpiManager;

impl CpiManager{
  pub async fn lock_collateral_cpi(
    state: &AppState,
    vault_pubkey : &str,
    authority: &Pubkey,
    amount: u64,
  ) -> Result<String , CPIError>{
    tracing::info!("CPI: Locking {} in vault {}", amount, vault_pubkey);
     if amount == 0 {
            return Err(CPIError::InvalidAmount("Amount must be greater than zero".to_string()));
    }
    let vault_pk = Pubkey::from_str(vault_pubkey).map_err(|e| CPIError::InvalidPubkey(e.to_string()))?;
    let (vault_authority_pda , _bump) = Pubkey::find_program_address(
      &[b"vault_authority" , vault_pk.as_ref()], &state.program_id);
    let lock_ix = build_lock_instruction(
      &state.program_id, 
      &vault_pk, 
      &vault_authority_pda, 
      &authority, 
      amount
    )?;

    let recent_blockhash = state.solana_client.get_latest_blockhash().await.map_err(|e| CPIError::RpcError(e.to_string()))?;
    let transaction = Transaction::new_with_payer(&[lock_ix], None);
    let signature = state.solana_client.send_and_confirm_transaction(&transaction).await.map_err(|e| CPIError::TransactionFailed(e.to_string()))?;
    tracing::info!("CPI: Lock successful, signature: {}", signature);    
    Ok(signature.to_string())
  }

  pub async fn unlock_collateral_cpi(
    state: &AppState,
    vault_pubkey : &str,
    authority: &Pubkey,
    amount : u64
  ) -> Result<String , CPIError>{
    tracing::info!("CPI: Unlocking {} in vault {}", amount, vault_pubkey);
    
    if amount == 0 {
      return Err(CPIError::InvalidAmount("Amount must be greater than zero".to_string()));
    }

    let vault_pk = Pubkey::from_str(vault_pubkey)
    .map_err(|e| CPIError::InvalidPubkey(e.to_string()))?;

    let (vault_authority_pda , _bump) = Pubkey::find_program_address(
      &[b"vault_authority" , vault_pk.as_ref()], &state.program_id);
    let unlock_ix = build_unlock_instruction(
      &state.program_id, 
      &vault_pk, 
      &vault_authority_pda, 
      &authority, 
      amount
    )?;

    let recent_blockhash = state.solana_client.get_latest_blockhash().await.map_err(|e| CPIError::RpcError(e.to_string()))?;
    let mut transaction = Transaction::new_with_payer(
      &[unlock_ix],
      None
    );
    let signature = state.solana_client.send_and_confirm_transaction(&transaction).await.map_err(|e| CPIError::TransactionFailed(e.to_string()))?;
    tracing::info!("CPI: Unlock successful, signature: {}", signature);
        
    Ok(signature.to_string())
  }

  pub async fn transfer_collateral_vault(
    state: &AppState,
    from_vault_pubkey : &str,
    to_vault_pubkey : &str,
    amount : u64,
    authority: &Pubkey
  ) -> Result<String , CPIError>{
    tracing::info!(
            "CPI: Transferring {} from {} to {}",
            amount,
            from_vault_pubkey,
            to_vault_pubkey
        );

    if amount == 0 {
            return Err(CPIError::InvalidAmount("Amount must be greater than zero".to_string()));
    }
    let from_vault_pk = Pubkey::from_str(from_vault_pubkey).map_err(|e| CPIError::InvalidPubkey(e.to_string()))?;
    let to_vault_pk = Pubkey::from_str(to_vault_pubkey).map_err(|e| CPIError::InvalidPubkey(e.to_string()))?;

    let from_vault = state.database.get_vault(from_vault_pubkey).await.map_err(|e| CPIError::DatabaseError(e.to_string()))?.ok_or(CPIError::VaultNotFound("vault not found".to_string()))?;
    let to_vault = state.database.get_vault(to_vault_pubkey).await.map_err(|e| CPIError::DatabaseError(e.to_string()))?.ok_or(CPIError::VaultNotFound("vault not found".to_string()))?;

    let from_token_account = Pubkey::from_str(&from_vault.token_account)
        .map_err(|e| CPIError::InvalidPubkey(e.to_string()))?;
    let to_token_account = Pubkey::from_str(&to_vault.token_account)
        .map_err(|e| CPIError::InvalidPubkey(e.to_string()))?;
        
    let (vault_authority_pda, _bump) = Pubkey::find_program_address(
        &[b"vault_authority", from_vault_pk.as_ref()],
        &state.program_id,
    );

    let transfer_ix = build_transfer_instruction(
      &state.program_id, 
      &from_vault_pk, 
      &to_vault_pk,
      &from_token_account, 
      &to_token_account, 
      &vault_authority_pda, 
      authority, 
      amount
    )?;

    let transaction = Transaction::new_with_payer(
      &[transfer_ix], 
      None
    );

    let signature = state.solana_client.send_and_confirm_transaction(&transaction).await.map_err(|e| CPIError::TransactionFailed(e.to_string()))?;
    tracing::info!("CPI: Transfer successful, signature: {}", signature);
        
    Ok(signature.to_string())
  } 
  pub fn handle_cpi_error(error: &CPIError, operation: &str) {
        match error {
            CPIError::InvalidAmount(_) => {
                tracing::error!("CPI {} failed: Invalid amount - {}", operation, error);
            }
            CPIError::InsufficientBalance { .. } => {
                tracing::error!("CPI {} failed: Insufficient balance - {}", operation, error);
            }
            CPIError::TransactionFailed(msg) => {
                tracing::error!("CPI {} transaction failed: {}", operation, msg);
            }
            _ => {
                tracing::error!("CPI {} error: {}", operation, error);
            }
        }
    }
}

fn build_lock_instruction(
  program_id : &Pubkey,
  vault: &Pubkey,
  vault_authority: &Pubkey,
  authority_program: &Pubkey,
  amount : u64,
) -> Result<Instruction , CPIError>{
  let discriminator: [u8; 8] = [0,1,2,3,4,5,6,7];
  let mut data = Vec::with_capacity(16);
  data.extend_from_slice(&discriminator);
  data.extend_from_slice(&amount.to_le_bytes());

  Ok(
    Instruction { 
      program_id: *program_id , 
      accounts: vec![
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vault_authority,false),
        AccountMeta::new(*authority_program, false),
      ], 
      data
    }
  ) 
}

fn build_unlock_instruction(
  program_id : &Pubkey,
  vault: &Pubkey,
  vault_authority: &Pubkey,
  authority_program: &Pubkey,
  amount : u64
) -> Result<Instruction , CPIError>{
  let discriminator: [u8; 8] = [0,1,2,3,4,5,6,8];
  let mut data = Vec::with_capacity(16);
  data.extend_from_slice(&discriminator);
  data.extend_from_slice(&amount.to_le_bytes());

  Ok(
    Instruction { 
      program_id: *program_id, 
      accounts: vec![
        AccountMeta::new(*vault, false),
        AccountMeta::new(*vault_authority, false),
        AccountMeta::new_readonly(*authority_program, false)
      ], 
      data
    }
  )
}

fn build_transfer_instruction(
  program_id : &Pubkey,
  from_vault : &Pubkey,
  to_vault : &Pubkey,
  from_token_account : &Pubkey,
  to_token_account : &Pubkey,
  vault_authority: &Pubkey,
  authority_program: &Pubkey,
  amount : u64
) -> Result<Instruction , CPIError>{
  let discriminator = [0,1,2,3,4,5,6,9];

  let mut data = Vec::with_capacity(16);
  data.extend_from_slice(&discriminator);
  data.extend_from_slice(&amount.to_le_bytes());

  Ok(
    Instruction { 
      program_id: *program_id,
      accounts: vec![
        AccountMeta::new(*from_vault, false),
        AccountMeta::new(*to_vault, false),
        AccountMeta::new(*from_token_account, false),
        AccountMeta::new(*to_token_account, false),
        AccountMeta::new(*vault_authority, false),
        AccountMeta::new_readonly(*authority_program, false),
        AccountMeta::new_readonly(spl_token::id(), false)
      ], 
      data 
    }
  )
}

#[derive(Debug, thiserror::Error)]
pub enum CPIError {
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    
    #[error("Invalid pubkey: {0}")]
    InvalidPubkey(String),
    
    #[error("Vault not found: {0}")]
    VaultNotFound(String),
    
    #[error("Insufficient balance: available={available}, required={required}")]
    InsufficientBalance { available: u64, required: u64 },
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Program not authorized")]
    Unauthorized,
    
    #[error("Instruction build failed: {0}")]
    InstructionBuildError(String),
}
