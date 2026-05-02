//! Stable program errors with Anchor error code mapping.
//!
//! The #[error_code] macro assigns numeric codes (0, 1, 2, ...) to each variant.
//! The backend should map these into retriable vs terminal categories.

use anchor_lang::prelude::*;

/// Program error codes.
///
/// Each variant is automatically assigned a numeric code by Anchor.
/// This code is exposed in the IDL and used in transaction logs.
/// Order matters: first variant = 6000, second = 6001, etc.
#[derive(PartialEq)]
#[error_code]
pub enum ContractError {
    /// Instruction data is malformed or incomplete.
    #[msg("Invalid instruction")]
    InvalidInstruction,

    /// Account is owned by wrong program (not this contract).
    #[msg("Account ownership mismatch")]
    AccountOwnershipMismatch,

    /// Insufficient funds in escrow or payer account.
    #[msg("Insufficient funds")]
    InsufficientFunds,

    /// Signer is not the authorized oracle.
    #[msg("Unauthorized oracle signer")]
    UnauthorizedOracle,

    /// Signer is not the authorized user/pledge owner.
    #[msg("Unauthorized user signer")]
    UnauthorizedUser,

    /// Deadline has not been reached yet (for oracle resolution).
    #[msg("Deadline not reached")]
    DeadlineNotReached,

    /// Timeout grace period has not been reached (for user timeout claim).
    #[msg("Timeout grace period not reached")]
    TimeoutNotReached,

    /// External provider (oracle, LLM service, etc.) is unavailable.
    /// This error should trigger retry logic on the backend.
    #[msg("Provider unavailable")]
    ProviderUnavailable,

    /// Pledge has already been resolved (immutability constraint).
    #[msg("Pledge already resolved")]
    AlreadyResolved,
}

impl ContractError {
    /// Check if this error should trigger retry logic on the backend.
    pub fn is_retriable(&self) -> bool {
        matches!(self, ContractError::ProviderUnavailable)
    }
}
