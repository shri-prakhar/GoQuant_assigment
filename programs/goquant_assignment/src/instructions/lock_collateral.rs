use anchor_lang::prelude::*;

use crate::{
    error::VaultError,
    states::{CollateralVault, LockEvent, VaultAuthority},
};

#[derive(Accounts)]
pub struct LockCollateral<'info> {
    #[account(mut)]
    pub vault: Account<'info, CollateralVault>,

    #[account(
    mut,
    seeds = [b"vault_authority" , vault.key().as_ref()],
    bump
  )]
    pub vault_authority: Account<'info, VaultAuthority>,

    ///CHECK: will be check later
    pub authority_program: UncheckedAccount<'info>,
}

pub fn lock_collateral_handler(ctx: Context<LockCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InvalidAmount);

    let authorized_accounts = &ctx.accounts.vault_authority;
    //let caller_program_id = ctx.program_id;

    require!(
        authorized_accounts.is_program_authorized(&ctx.accounts.authority_program.key()),
        VaultError::ProgramNotAuthorized
    );

    let vault = &mut ctx.accounts.vault;
    require!(
        vault.available_balance >= amount,
        VaultError::InsufficientBalance
    );
    vault.locked_balance = vault
        .locked_balance
        .checked_add(amount)
        .ok_or(VaultError::OverFlow)?;
    vault.available_balance = vault
        .available_balance
        .checked_sub(amount)
        .ok_or(VaultError::UnderFlow)?;

    emit!(LockEvent {
        vault: vault.key(),
        amount,
        total_locked_balance: vault.locked_balance,
        total_available_balance: vault.available_balance,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
