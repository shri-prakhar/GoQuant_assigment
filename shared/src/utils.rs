use crate::{VaultError, VaultResult};

pub fn validate_pubkey(pubkey : &str) -> VaultResult<()>{
  if pubkey.len() < 32 || pubkey.len() > 44 {
    return Err(VaultError::InvalidPubkey("Pubkey length must be between 32-44 characters".to_string()));
  }

  bs58::decode(pubkey).into_vec().map_err(|e| VaultError::InvalidPubkey(format!("Invalid base58: {}" , e)))?;
  Ok(())
}

pub fn validate_signature(signature: &str) -> VaultResult<()> {
    if signature.len() < 86 || signature.len() > 88 {
        return Err(VaultError::InvalidPubkey(
            "Signature length must be 86-88 characters".to_string()
        ));
    }
    
    bs58::decode(signature)
        .into_vec()
        .map_err(|e| VaultError::InvalidPubkey(format!("Invalid signature: {}", e)))?;
    
    Ok(())
}

pub fn validate_amount(amount: i64) -> VaultResult<i64> {
  if amount <=0 {
    return Err(VaultError::InvalidAmount(
       "Amount must be greater than zero".to_string()
    ));
  }

  Ok(amount)
}

pub fn checked_add(a: i64, b: i64) -> VaultResult<i64> {
    a.checked_add(b).ok_or(VaultError::Overflow)
}

pub fn checked_sub(a: i64, b: i64) -> VaultResult<i64> {
    a.checked_sub(b).ok_or(VaultError::Underflow)
}

pub fn checked_mul(a: i64, b: i64) -> VaultResult<i64> {
    a.checked_mul(b).ok_or(VaultError::Overflow)
}

pub fn base_units_to_usdt(amount: i64) -> f64 {
  amount as f64 / 1_000_000.0
}

pub fn usdt_to_base_units(amount : f64) -> i64 {
  (amount * 1_000_000.0) as i64
}

pub fn format_usdt(amount: i64) -> String {
  format!("{:.6} USDT" , base_units_to_usdt(amount))
}
