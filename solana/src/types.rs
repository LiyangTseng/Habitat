//! Shared request and response types for pledge instructions.

use anchor_lang::prelude::*;

/// Arguments for initializing a new pledge.
/// Sent by the client in the initialize_pledge instruction.
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct InitializePledgeArgs {
    pub pledge_id: String,
    pub oracle_pubkey: Pubkey,
    pub escrow_amount: u64,
    pub deadline_timestamp: i64,
}

/// Pledge lifecycle status enum.
/// AnchorSerialize/AnchorDeserialize required for use in #[account] structs.
#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub enum PledgeStatus {
    Pending,
    ResolvedSuccess,
    ResolvedFailure,
}

impl PledgeStatus {
    /// Convert enum variant to backend-compatible string representation.
    /// Used when syncing state to the Go backend.
    pub fn as_backend_value(&self) -> &'static str {
        match self {
            PledgeStatus::Pending => "pending",
            PledgeStatus::ResolvedSuccess => "resolved_success",
            PledgeStatus::ResolvedFailure => "resolved_failure",
        }
    }
}

/// Audit trail receipt for pledge resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionReceiptData {
    pub pledge_id: String,
    pub resolved_by: String,
    pub resolution: PledgeStatus,
    pub tx_hash: String,
    pub finalized_at_unix: i64,
}
