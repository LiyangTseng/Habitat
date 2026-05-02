//! Canonical pledge account stored on chain.

use anchor_lang::prelude::*;
use crate::types::PledgeStatus;

/// Pledge account PDA stored on-chain.
///
/// The #[account] attribute tells Anchor to:
/// 1. Implement serialization/deserialization (borsh)
/// 2. Add 8-byte discriminator to prevent account type confusion
/// 3. Include this type in the IDL for clients
///
/// This struct is serialized into the account's data buffer and
/// persists until the account is closed.
#[derive(Debug)]
#[account]
pub struct PledgeState {
    pub pledge_id: String,
    pub user_pubkey: String,
    pub oracle_pubkey: String,
    pub escrow_amount: u64,
    pub deadline_timestamp: i64,
    pub status: PledgeStatus,
}

impl PledgeState {
    /// Calculate the serialized size of PledgeState for Anchor account space allocation.
    ///
    /// Layout breakdown:
    /// - pledge_id: 4 bytes (length) + 32 bytes (content) = 36 bytes
    /// - user_pubkey: 4 bytes (length) + 44 bytes (base58 pubkey) = 48 bytes
    /// - oracle_pubkey: 4 bytes (length) + 44 bytes (base58 pubkey) = 48 bytes
    /// - escrow_amount: 8 bytes (u64)
    /// - deadline_timestamp: 8 bytes (i64)
    /// - status: enum with 1 byte discriminator
    ///
    /// Total: 157 bytes
    pub const LEN: usize = 4 + 32 + 4 + 44 + 4 + 44 + 8 + 8 + 1;
}
