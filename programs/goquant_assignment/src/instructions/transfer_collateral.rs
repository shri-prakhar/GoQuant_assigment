use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{
    error::VaultError,
    states::{CollateralVault, VaultAuthority},
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
