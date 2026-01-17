//! # GoQuant Assignment - Collateral Vault Program
//!
//! A Solana Anchor program that implements a collateral vault system for DeFi protocols.
//! Users can deposit tokens as collateral, lock/unlock portions for lending, and withdraw
//! when needed.
//!
//! ## Features
//!
//! - **Vault Initialization**: Create new collateral vaults for users
//! - **Deposit**: Add tokens to vault as collateral
//! - **Withdraw**: Remove tokens from vault (subject to locking constraints)
//! - **Lock/Unlock**: Temporarily lock collateral for DeFi protocols
//! - **Transfer**: Move collateral between vaults
//! - **Events**: Emit structured events for off-chain processing
//!
//! ## Security Considerations
//!
//! - All operations validate ownership and balances
//! - Locked collateral cannot be withdrawn until unlocked
//! - Authority controls for program upgrades
//! - Comprehensive event logging for transparency
//!
//! ## Program ID
//!
//! `A9JDc7TrKR5Qyot3W3t6UQaRz4CTgEURemuSUkWfP9hs`

use anchor_lang::prelude::*;

pub mod error;
pub mod instructions;
pub mod states;

use crate::instructions::*;

declare_id!("3sTDJpeRCmXSu9pmkkxjFwYrCHTuoDF3NDWRzFUwKrTg");

/// GoQuant Assignment - Collateral Vault Program
///
/// This program provides a secure vault system for collateral management
/// in DeFi applications on Solana.
#[program]
pub mod goquant_assignment {
    use super::*;

    /// Add an authorized program that can interact with vaults
    ///
    /// # Arguments
    /// * `ctx` - Program context with authority signer
    /// * `program_id` - The program ID to authorize
    ///
    /// # Security
    /// Only the program authority can call this function
    pub fn authority_to_add(ctx: Context<AddAuthorizedProgram>, program_id: Pubkey) -> Result<()> {
        add_authorized_program_handler(ctx, program_id)
    }

    /// Initialize a new collateral vault for a user
    ///
    /// Creates a new vault account and associates it with the user's token account.
    ///
    /// # Arguments
    /// * `ctx` - Program context with vault, owner, and token accounts
    ///
    /// # Events
    /// Emits `VaultInitializedEvent` on success
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        instructions::initialize_vault_handler(ctx)
    }

    /// Deposit tokens into a vault as collateral
    ///
    /// Transfers tokens from user's token account to vault's token account
    /// and updates vault balances.
    ///
    /// # Arguments
    /// * `ctx` - Program context with vault and token accounts
    /// * `amount` - Amount of tokens to deposit (in smallest units)
    ///
    /// # Events
    /// Emits `DepositEvent` on success
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        deposit_handler(ctx, amount)
    }

    /// Withdraw tokens from a vault
    ///
    /// Transfers tokens from vault back to user's token account.
    /// Only available balance (not locked) can be withdrawn.
    ///
    /// # Arguments
    /// * `ctx` - Program context with vault and token accounts
    /// * `amount` - Amount of tokens to withdraw (in smallest units)
    ///
    /// # Events
    /// Emits `WithdrawEvent` on success
    ///
    /// # Errors
    /// Returns error if insufficient available balance
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        withdraw_handler(ctx, amount)
    }

    /// Lock collateral for DeFi protocol use
    ///
    /// Moves tokens from available to locked balance.
    /// Locked collateral cannot be withdrawn until unlocked.
    ///
    /// # Arguments
    /// * `ctx` - Program context with vault account
    /// * `amount` - Amount of tokens to lock (in smallest units)
    ///
    /// # Events
    /// Emits `LockEvent` on success
    ///
    /// # Errors
    /// Returns error if insufficient available balance
    pub fn lock_collateral(ctx: Context<LockCollateral>, amount: u64) -> Result<()> {
        lock_collateral_handler(ctx, amount)
    }

    /// Unlock previously locked collateral
    ///
    /// Moves tokens from locked back to available balance.
    ///
    /// # Arguments
    /// * `ctx` - Program context with vault account
    /// * `amount` - Amount of tokens to unlock (in smallest units)
    ///
    /// # Events
    /// Emits `UnlockEvent` on success
    ///
    /// # Errors
    /// Returns error if insufficient locked balance
    pub fn unlock_collateral(ctx: Context<UnLockCollateral>, amount: u64) -> Result<()> {
        unlock_collateral_handler(ctx, amount)
    }

    /// Transfer collateral between vaults
    ///
    /// Moves collateral from one vault to another.
    /// Both vaults must be owned by the signer.
    ///
    /// # Arguments
    /// * `ctx` - Program context with both vault accounts
    /// * `amount` - Amount of tokens to transfer (in smallest units)
    ///
    /// # Events
    /// Emits `TransferEvent` on success
    pub fn transfer_collateral(ctx: Context<TransferCollateral>, amount: u64) -> Result<()> {
        transfer_collateral_handler(ctx, amount)
    }
}
