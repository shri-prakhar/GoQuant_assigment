use anchor_lang::prelude::*;

pub mod instructions;
pub mod states;
pub mod error;


declare_id!("A9JDc7TrKR5Qyot3W3t6UQaRz4CTgEURemuSUkWfP9hs");

#[program]
pub mod goquant_assignment {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
