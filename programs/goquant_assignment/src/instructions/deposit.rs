use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer, transfer};

use crate::{error::VaultError, states::{CollateralVault, DepositEvent}};

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

pub fn deposit_handler(ctx: Context<Deposit> , amount : u64) -> Result<()>{
  require!(amount > 0 , VaultError::InvalidAmount);

  let cpi_accounts = Transfer{
    from: ctx.accounts.user_token_account.to_account_info(),
    to: ctx.accounts.vault_ata.to_account_info(),
    authority: ctx.accounts.user.to_account_info()
  };

  let cpi_program = ctx.accounts.token_program.to_account_info();
  transfer(CpiContext::new(cpi_program, cpi_accounts), amount)?;

  let vault = &mut ctx.accounts.vault;
  vault.total_balance = vault.total_balance.checked_add(amount).ok_or(VaultError::OverFlow)?;
  vault.available_balance = vault.available_balance.checked_add(amount).ok_or(VaultError::OverFlow)?;
  vault.total_deposited = vault.total_deposited.checked_add(amount).ok_or(VaultError::OverFlow)?;

  emit!(DepositEvent{
    user: ctx.accounts.user.key(),
    vault: vault.key(),
    amount,
    new_available_balance : vault.available_balance,
    new_total_balance: vault.total_balance,
    timestamp: Clock::get()?.unix_timestamp,
  });

  Ok(())
}