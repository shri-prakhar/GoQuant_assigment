use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::states::{CollateralVault, VaultAuthority, VaultInitializeEvent};

#[derive(Accounts)]

pub struct InitializeVault<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
    init,
    payer = user ,
    space = 8 + CollateralVault::LEN,
    seeds = [b"vault" , user.key().as_ref()],
    bump
  )]
    pub vault: Account<'info, CollateralVault>,
    pub mint: Account<'info, Mint>,
    //vault_ata
    #[account(
    init,
    payer = user,
    associated_token::mint = mint,
    associated_token::authority = vault,
  )]
    pub vault_ata: Account<'info, TokenAccount>,
    #[account(
    init,
    payer = user,
    space = 8 + VaultAuthority::LEN,
    seeds = [b"vault_authority" , vault.key().as_ref()],
    bump,
  )]
    pub vault_authority: Account<'info, VaultAuthority>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn initialize_vault_handler(ctx: Context<InitializeVault>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let clock = Clock::get()?;
    vault.owner = ctx.accounts.user.key();
    vault.token_account = ctx.accounts.vault_ata.key();
    vault.total_balance = 0;
    vault.locked_balance = 0;
    vault.available_balance = 0;
    vault.total_deposited = 0;
    vault.total_withdrawn = 0;
    vault.created_at = clock.unix_timestamp;
    vault.bump = ctx.bumps.vault;

    {
        let va = &mut ctx.accounts.vault_authority;
        va.bump = ctx.bumps.vault_authority;
        va.authorized_programs = Vec::new();
    }
    emit!(VaultInitializeEvent {
        user: vault.owner,
        vault: vault.key(),
        token_account: vault.token_account,
        timestamp: clock.unix_timestamp
    });

    Ok(())
}
