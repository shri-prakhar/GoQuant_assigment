use anchor_lang::prelude::*;

#[account]
pub struct CollateralVault{
  pub owner: Pubkey,
  pub token_account: Pubkey,
  pub total_balance: u64,
  pub locked_balance: u64,
  pub available_balance: u64,
  pub total_deposited : u64,
  pub total_withdrawn : u64,
  pub created_at : i64,
  pub bump : u8,
}

impl CollateralVault {
    pub const LEN : usize = 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1;
}

#[account]
pub struct VaultAuthority{
  pub authorized_programs: Vec<Pubkey>,
  pub bump : u8
}
impl VaultAuthority{
  pub const LEN : usize = 4 + ( 32 * 8 ) + 1; // 4 bytes are the vector length 
}
impl VaultAuthority{
  pub fn is_program_authorized(&self , program: &Pubkey) -> bool{
    self.authorized_programs.iter().any(|p| p == program)
  }
}

#[derive(Copy , Clone , AnchorSerialize , AnchorDeserialize , Debug)]
pub struct TransactionRecord{
  pub vault: Pubkey,
  pub transaction_type: TransactionType,
  pub amount: u64,
  pub timestamp: i64, 
}

#[repr(u8)]
#[derive(Copy , Clone , AnchorSerialize , AnchorDeserialize , Debug)]
pub enum TransactionType {
      Deposit,
      Withdrawal,
      Lock,
      Unlock,
      Transfer
}

