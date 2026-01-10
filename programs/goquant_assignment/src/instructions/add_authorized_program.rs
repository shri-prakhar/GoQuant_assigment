use anchor_lang::prelude::*;

use crate::{
    error::VaultError,
    states::{CollateralVault, VaultAuthority},
};

#[derive(Accounts)]
pub struct AddAuthorizedProgram<'info> {
    #[account(
    mut,
    seeds = [b"vault_authority" , vault.key().as_ref()],
    bump
  )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(mut, seeds = [b"vault", admin.key().as_ref()], bump = vault.bump)]
    pub vault: Account<'info, CollateralVault>,

    pub admin: Signer<'info>,
}

//for admin
pub fn add_authorized_program_handler(
    ctx: Context<AddAuthorizedProgram>,
    program_id: Pubkey,
) -> Result<()> {
    let vault_authority = &mut ctx.accounts.vault_authority;

    require!(ctx.accounts.admin.is_signer, VaultError::UnAuthorized);

    if !vault_authority
        .authorized_programs
        .iter()
        .any(|p| p == &program_id)
    {
        vault_authority.authorized_programs.push(program_id);
    }
    Ok(())
}
