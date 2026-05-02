//! Event payloads emitted by the commitment contract.
//!
//! Anchor's #[event] macro automatically:
//! - Serializes events into transaction logs
//! - Adds them to the IDL for off-chain indexing
//! - Provides event subscriptions for clients

use anchor_lang::prelude::*;

/// Emitted when a new pledge is initialized.
///
/// Clients listen to this event to know when a pledge enters the system.
/// All fields are derived from InitializePledgeArgs input.
#[event]
pub struct PledgeInitialized {
    pub pledge_id: String,
    pub user_pubkey: String,
    pub oracle_pubkey: String,
    pub escrow_amount: u64,
    pub deadline_timestamp: i64,
}

/// Emitted when an oracle resolves a pledge (success or failure).
///
/// The status field indicates "success" or "failure".
/// Clients use this to update off-chain database and notify users.
#[event]
pub struct PledgeResolved {
    pub pledge_id: String,
    pub status: String,      // "success" or "failure"
    pub resolution_proof: String,
}

/// Emitted when a user claims timeout refund (pledge unresolved past deadline).
///
/// Indicates the pledge reverted to user without oracle involvement.
#[event]
pub struct PledgeTimeoutClaimed {
    pub pledge_id: String,
    pub user_pubkey: String,
    pub escrow_amount: u64,
}

/// Legacy event (deprecated in favor of PledgeInitialized + PledgeResolved).
/// Kept for backwards compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PledgeResolvedEvent {
    pub pledge_id: String,
    pub user_pubkey: String,
    pub oracle_pubkey: String,
    pub status: String,
    pub tx_hash: String,
    pub finalized_at_unix: i64,
}

impl PledgeResolvedEvent {
    pub fn new(
        pledge_id: String,
        user_pubkey: String,
        oracle_pubkey: String,
        status: String,
        tx_hash: String,
        finalized_at_unix: i64,
    ) -> Self {
        Self {
            pledge_id,
            user_pubkey,
            oracle_pubkey,
            status,
            tx_hash,
            finalized_at_unix,
        }
    }
}
