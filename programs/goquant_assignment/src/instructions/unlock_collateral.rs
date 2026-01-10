use anchor_lang::prelude::*;

use crate::{
    error::VaultError,
    states::{CollateralVault, UnLockEvent, VaultAuthority},
};

#[derive(Accounts)]
pub struct UnLockCollateral<'info> {
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

pub fn unlock_collateral_handler(ctx: Context<UnLockCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InvalidAmount);
    let authority_account = &ctx.accounts.vault_authority;

    require!(
        authority_account.is_program_authorized(&ctx.accounts.authority_program.key()),
        VaultError::ProgramNotAuthorized
    );

    let vault = &mut ctx.accounts.vault;
    require!(
        vault.locked_balance >= amount,
        VaultError::InsufficientBalance
    );

    vault.locked_balance = vault
        .locked_balance
        .checked_sub(amount)
        .ok_or(VaultError::UnderFlow)?;
    vault.available_balance = vault
        .available_balance
        .checked_add(amount)
        .ok_or(VaultError::OverFlow)?;

    emit!(UnLockEvent {
        vault: vault.key(),
        amount,
        new_available_balance: vault.available_balance,
        new_locked_balance: vault.locked_balance,
        timestamp: Clock::get()?.unix_timestamp
    });

    Ok(())
}
