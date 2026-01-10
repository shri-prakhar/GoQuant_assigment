use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{error::VaultError, states::CollateralVault};

#[derive(Accounts)]
pub struct Withdraw<'info>{
  #[account(mut)]
  pub user : Signer<'info>,

  #[account(
    mut,
    seeds = [b"vault" , user.key().as_ref()],
    bump,
  )]
  pub vault : Account<'info , CollateralVault>,
  //source
  #[account(
    mut,
    constraint = vault_ata.key() == vault.token_account @ VaultError::InvalidAmount
  )]
  pub vault_ata : Account<'info , TokenAccount>,
  //destination
  #[account(
    mut,
    constraint = user_token_account.owner == user.key() @VaultError::InvalidTokenAccount
  )]
  pub user_token_account : Account<'info , TokenAccount>,
  pub token_program : Program<'info , Token>
}
