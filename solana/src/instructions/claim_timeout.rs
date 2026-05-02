//! Allow user timeout reclaim when oracle resolution is unavailable.
//!
//! Implementation guide:
//! - Verify the user signer before touching state.
//! - Require the deadline plus grace period to have passed.
//! - Keep the pledge pending until the refund transfer succeeds.
//! - Transfer escrow lamports back to the user using Anchor CPI.
//! - Mark the pledge as resolved only after the transfer is complete.
//! - Return a receipt for backend reconciliation.

use crate::{
    config::DEFAULT_TIMEOUT_GRACE_SECONDS,
    error::ContractError,
    instructions::pledge_resolution::{update_pledge_status, build_resolution_receipt},
    state::{pledge_state::PledgeState, resolution_receipt::ResolutionReceipt},
    types::PledgeStatus,
};

pub(crate) fn claim_timeout(
    pledge: &mut PledgeState,
    user_signer: &str,
    now_unix: i64,
    tx_hash: String,
) -> Result<ResolutionReceipt, ContractError> {
    // 1. Confirm the caller is the pledge owner.
    // 2. Confirm the deadline plus grace period has passed.
    // 3. Transfer escrow back to the user.
    // 4. Flip state to the timeout resolution path.
    // 5. Build the timeout receipt.
    validate_timeout_claim(pledge, user_signer, now_unix)?;
    transfer_timeout_refund_stub()?;
    update_pledge_status(pledge, PledgeStatus::ResolvedSuccess);

    Ok(build_resolution_receipt(
        pledge,
        user_signer,
        tx_hash,
        now_unix,
    ))
}

pub(crate) fn timeout_claim_eligibility_timestamp(deadline_timestamp: i64) -> i64 {
    deadline_timestamp + DEFAULT_TIMEOUT_GRACE_SECONDS
}

pub(crate) fn validate_timeout_claim(
    pledge: &PledgeState,
    user_signer: &str,
    now_unix: i64,
) -> Result<(), ContractError> {
    if user_signer != pledge.user_pubkey {
        return Err(ContractError::UnauthorizedUser);
    }
    if pledge.status != PledgeStatus::Pending {
        return Err(ContractError::AlreadyResolved);
    }

    let timeout_at = timeout_claim_eligibility_timestamp(pledge.deadline_timestamp);
    if now_unix < timeout_at {
        return Err(ContractError::TimeoutNotReached);
    }

    Ok(())
}

/// Implementation notes for the timeout refund helper.
///
/// When this becomes real CPI code, it should:
/// 1. Receive the pledge account, the user account, and the system program.
/// 2. Use Anchor's `system_program::transfer` with `CpiContext::with_signer`.
/// 3. Reuse the PDA signer seeds derived in `lib.rs` so the pledge PDA can sign the CPI.
/// 4. Move `pledge.escrow_amount` lamports from the pledge PDA back to the user.
/// 5. Return `ContractError::InsufficientFunds` or another contract error instead of leaking Anchor errors.
///
/// This follows the same refund shape as the success path, but the instruction intent is
/// user-initiated timeout recovery instead of oracle-approved success resolution.
pub(crate) fn transfer_timeout_refund_stub() -> Result<(), ContractError> {
    Ok(())
}
