use anchor_lang::prelude::*;

use crate::states::{CollateralVault, VaultAuthority};

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
