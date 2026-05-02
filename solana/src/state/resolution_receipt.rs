//! Resolution evidence stored for audit and reconciliation.

use anchor_lang::prelude::*;
use crate::types::PledgeStatus;

/// Resolution receipt PDA/account stored on-chain for audit history.
///
/// This is separate from PledgeState because it captures an immutable
/// outcome record rather than the current pledge lifecycle state.
#[account]
pub struct ResolutionReceipt {
    pub pledge_id: String,
    pub resolved_by: String,
    pub resolution: PledgeStatus,
    pub tx_hash: String,
    pub finalized_at_unix: i64,
}

impl ResolutionReceipt {
    /// Serialized payload size for Anchor account allocation.
    ///
    /// Layout breakdown:
    /// - pledge_id: 4 bytes (length) + 32 bytes (content) = 36 bytes
    /// - resolved_by: 4 bytes (length) + 44 bytes (base58 pubkey) = 48 bytes
    /// - resolution: enum discriminator = 1 byte
    /// - tx_hash: 4 bytes (length) + 88 bytes (base58 signature) = 92 bytes
    /// - finalized_at_unix: 8 bytes (i64)
    ///
    /// Total payload: 185 bytes
    pub const LEN: usize = 4 + 32 + 4 + 44 + 1 + 4 + 88 + 8;
}
