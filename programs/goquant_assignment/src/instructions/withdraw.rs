use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::{
    error::VaultError,
    states::{CollateralVault, WithdrawEvent},
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
    mut,
    seeds = [b"vault" , user.key().as_ref()],
    bump,
  )]
    pub vault: Account<'info, CollateralVault>,
    //source
    #[account(
    mut,
    constraint = vault_ata.key() == vault.token_account @ VaultError::InvalidAmount
  )]
    pub vault_ata: Account<'info, TokenAccount>,
    //destination
    #[account(
    mut,
    constraint = user_token_account.owner == user.key() @VaultError::InvalidTokenAccount
  )]
    pub user_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn withdraw_handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InvalidAmount);
    let vault = &mut ctx.accounts.vault;

    require!(
        vault.owner == ctx.accounts.user.key(),
        VaultError::UnAuthorized
    );
    require!(
        vault.available_balance >= amount,
        VaultError::InsufficientBalance
    );

    let seeds = &[b"vault", vault.owner.as_ref(), &[vault.bump]];
    let signer: &[&[&[u8]]] = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_ata.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: vault.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();

    transfer(
        CpiContext::new_with_signer(cpi_program, cpi_accounts, signer),
        amount,
    )?;

    vault.total_balance = vault
        .total_balance
        .checked_sub(amount)
        .ok_or(VaultError::OverFlow)?;
    vault.available_balance = vault
        .available_balance
        .checked_sub(amount)
        .ok_or(VaultError::OverFlow)?;
    vault.total_withdrawn = vault
        .total_withdrawn
        .checked_add(amount)
        .ok_or(VaultError::OverFlow)?;

    emit!(WithdrawEvent {
        user: ctx.accounts.user.key(),
        vault: vault.key(),
        amount,
        new_available_balance: vault.available_balance,
        new_total_balance: vault.total_balance,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
