use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{
    error::VaultError,
    states::{CollateralVault, TransferEvent, VaultAuthority},
};

#[derive(Accounts)]
pub struct TransferCollateral<'info> {
    #[account(mut)]
    pub from_vault: Account<'info, CollateralVault>,
    #[account(mut)]
    pub to_vault: Account<'info, CollateralVault>,
    #[account(
    mut,
    constraint = from_vault_ata.key() == from_vault.token_account @VaultError::InvalidTokenAccount
  )]
    pub from_vault_ata: Account<'info, TokenAccount>,
    #[account(
    mut,
    constraint = to_vault_ata.key() == to_vault.token_account @VaultError::InvalidTokenAccount
  )]
    pub to_vault_ata: Account<'info, TokenAccount>,
    #[account(
    mut,
    seeds = [b"vault_authority" , from_vault.key().as_ref()],
    bump
  )]
    pub vault_authority: Account<'info, VaultAuthority>,

    ///CHECK: will be check later
    pub authority_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn transfer_collateral_handler(ctx: Context<TransferCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InvalidAmount);
    let authority_account = &ctx.accounts.vault_authority;
    require!(
        authority_account.is_program_authorized(&ctx.accounts.authority_program.key()),
        VaultError::ProgramNotAuthorized
    );

    let from_vault = &mut ctx.accounts.from_vault;
    let to_vault = &mut ctx.accounts.to_vault;

    require!(
        from_vault.available_balance >= amount,
        VaultError::InsufficientBalance
    );

    from_vault.total_balance = from_vault
        .total_balance
        .checked_sub(amount)
        .ok_or(VaultError::UnderFlow)?;
    from_vault.available_balance = from_vault
        .available_balance
        .checked_sub(amount)
        .ok_or(VaultError::UnderFlow)?;
    to_vault.total_balance = to_vault
        .total_balance
        .checked_add(amount)
        .ok_or(VaultError::OverFlow)?;
    to_vault.available_balance = to_vault
        .available_balance
        .checked_add(amount)
        .ok_or(VaultError::OverFlow)?;

    let seeds = &[b"vault", from_vault.owner.as_ref(), &[from_vault.bump]];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.from_vault_ata.to_account_info(),
        to: ctx.accounts.to_vault_ata.to_account_info(),
        authority: from_vault.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();

    transfer(
        CpiContext::new_with_signer(cpi_program, cpi_accounts, signer),
        amount,
    )?;

    emit!(TransferEvent {
        from_vault: from_vault.key(),
        to_vault: to_vault.key(),
        amount,
        timestamp: Clock::get()?.unix_timestamp
    });

    Ok(())
}
