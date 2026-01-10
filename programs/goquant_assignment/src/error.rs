use anchor_lang::prelude::*;

#[error_code]

pub enum VaultError {
    #[msg("Invalid Amount: Must be Greater than Zero")]
    InvalidAmount,
    #[msg("Insufficient Balance For this Operation")]
    InsufficientBalance,
    #[msg("Insufficient Locked Balance")]
    InsufficientLockedBalance,
    #[msg("Unauthorized: you don't have permission for this operation")]
    UnAuthorized,
    #[msg("Invalid Token Account")]
    InvalidTokenAccount,
    #[msg("Arithmetic Overflow")]
    OverFlow,
    #[msg("Arithmetic Underflow")]
    UnderFlow,
    #[msg("Program Not Authorized to perform this Operation")]
    ProgramNotAuthorized,
    #[msg("Bump Not Found")]
    BumpNotFound,
    #[msg("Vault has Open Positions - cannot withdraw locked collateral")]
    HasOpenPositions,
}
