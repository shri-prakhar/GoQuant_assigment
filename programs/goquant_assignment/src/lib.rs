use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod states;

use crate::instructions::*;
declare_id!("A9JDc7TrKR5Qyot3W3t6UQaRz4CTgEURemuSUkWfP9hs");

#[program]
pub mod goquant_assignment {
    use super::*;

    pub fn authority_to_add(ctx: Context<AddAuthorizedProgram>, program_id: Pubkey) -> Result<()> {
        add_authorized_program_handler(ctx, program_id)
    }
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize_vault_handler(ctx)
    }
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        deposit_handler(ctx, amount)
    }
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        withdraw_handler(ctx, amount)
    }
    pub fn lock_collateral(ctx: Context<LockCollateral>, amount: u64) -> Result<()> {
        lock_collateral_handler(ctx, amount)
    }
    pub fn unlock_collateral(ctx: Context<UnLockCollateral>, amount: u64) -> Result<()> {
        unlock_collateral_handler(ctx, amount)
    }
    pub fn transfer_collateral(ctx: Context<TransferCollateral>, amount: u64) -> Result<()> {
        transfer_collateral_handler(ctx, amount)
    }
}
