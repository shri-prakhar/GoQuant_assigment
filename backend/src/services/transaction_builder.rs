use solana_sdk::{hash::Hash, pubkey::Pubkey, transaction::Transaction};
use spl_token::instruction as token_instruction;

pub struct TransactionBuilder;

impl TransactionBuilder {
    pub fn build_deposit_tx(
        user_pubkey: &Pubkey,
        user_token_account: &Pubkey,
        vault_token_account: &Pubkey,
        amount: u64,
        recent_blockhash: Hash,
    ) -> Result<Transaction, BuilderError> {
        let transfer_ix = token_instruction::transfer(
            &spl_token::id(),
            user_token_account,
            vault_token_account,
            user_pubkey,
            &[],
            amount,
        )
        .map_err(|e| BuilderError::BuildFailed(e.to_string()))?;

        let mut transaction = Transaction::new_with_payer(&[transfer_ix], Some(user_pubkey));
        transaction.message.recent_blockhash = recent_blockhash;
        Ok(transaction)
    }
    pub fn build_withdraw_tx(
        user_pubkey: &Pubkey,
        vault_pubkey: &Pubkey,
        vault_token_account: &Pubkey,
        user_token_account: &Pubkey,
        amount: u64,
        recent_blockhash: solana_sdk::hash::Hash,
    ) -> Result<Transaction, BuilderError> {
        // For withdrawal, the vault PDA must sign
        // This is typically done through the program
        let transfer_ix = token_instruction::transfer(
            &spl_token::id(),
            vault_token_account,
            user_token_account,
            vault_pubkey, // Vault PDA is authority
            &[],
            amount,
        )
        .map_err(|e| BuilderError::BuildFailed(e.to_string()))?;

        let mut transaction = Transaction::new_with_payer(&[transfer_ix], Some(user_pubkey));

        transaction.message.recent_blockhash = recent_blockhash;

        Ok(transaction)
    }

    pub fn add_compute_budget(
        transaction: &mut Transaction,
        compute_units: u32,
    ) -> Result<(), BuilderError> {
        // TODO: Add compute budget instruction
        // solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(compute_units)
        Ok(())
    }

    pub fn estimate_fee(transaction: &Transaction, lamports_per_signature: u64) -> u64 {
        let num_signatures = transaction.message.header.num_required_signatures as u64;
        num_signatures * lamports_per_signature
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Invalid pubkey")]
    InvalidPubkey,

    #[error("Transaction build failed: {0}")]
    BuildFailed(String),
}
