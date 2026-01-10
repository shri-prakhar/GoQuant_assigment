use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{error::VaultError, states::CollateralVault};

#[derive(Accounts)]
pub struct Deposit<'info>{ 
  #[account(mut)]
  pub user : Signer<'info>,
  #[account(
    mut , 
    seeds = [b"vault" , user.key().as_ref()],
    bump,
    has_one = owner @ VaultError::UnAuthorized
  )]
  pub vault : Account<'info , CollateralVault>,
  // USers USDT tokenAccount (source for funds)
  #[account(
    mut,
    constraint = user_token_account.owner == user.key() @VaultError::InvalidTokenAccount
  )]
  pub user_token_account : Account<'info , TokenAccount>,
  //Vault USDT tokenAccount (destination for funds)
  #[account(
    mut,
    constraint = vault_ata.key() == vault.token_account @VaultError::InvalidTokenAccount
  )]
  pub vault_ata : Account<'info , TokenAccount>,

  pub token_program : Program<'info , Token>,
  ///CHECK: This is Validated by the has_one constraint 
  pub owner: UncheckedAccount<'info>
}