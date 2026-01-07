use anchor_lang::prelude::*;

#[event]
pub struct VaultInitializeEvent{
  pub user : Pubkey , 
  pub vault : Pubkey , 
  pub token_account : Pubkey ,
  pub timestamp : i64
}

#[event]
pub struct DepositEvent{
  pub user : Pubkey , 
  pub vault : Pubkey ,
  pub amount : u64 ,
  pub new_total_balance : u64,
  pub new_available_balance : u64,
  pub timestamp : i64
}

#[event]
pub struct LockEvent{
  pub vault : Pubkey,
  pub amount : u64,
  pub total_locked_balance : u64,
  pub total_available_balance : u64,
  pub timestamp : i64 
}

#[event]
pub struct UnLockEvent{
  pub vault : Pubkey,
  pub amount : u64,
  pub new_locked_balance : u64,
  pub new_available_balance : u64,
  pub timestamp : i64,
}

#[event]
pub struct TransferEvent{
  pub from_vault : Pubkey,
  pub to_vault : Pubkey,
  pub amount : u64,
  pub timestamp : i64
}
