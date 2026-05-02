//! Shared helpers for pledge resolution outcomes: validation, status updates, receipts, and escrow transfers.

use anchor_lang::{
    prelude::{Account, CpiContext, Program, System},
    system_program::{transfer, Transfer},
    ToAccountInfo,
};

use crate::{
    error::ContractError,
    state::{pledge_state::PledgeState, resolution_receipt::ResolutionReceipt},
    types::PledgeStatus,
};

/// Validate that the trusted oracle signer is allowed to resolve this pledge now.
///
/// This is shared by both success and failure resolution paths because the same
/// preconditions apply before either branch can continue.
pub(crate) fn validate_oracle_resolution(
    pledge: &PledgeState,
    oracle_signer: &str,
) -> Result<(), ContractError> {
    if oracle_signer != pledge.oracle_pubkey {
        return Err(ContractError::UnauthorizedOracle);
    }
    if pledge.status != PledgeStatus::Pending {
        return Err(ContractError::AlreadyResolved);
    }

    Ok(())
}

/// Update the pledge status to reflect a resolution outcome.
///
/// Called after transfer succeeds to mark the pledge as resolved.
pub(crate) fn update_pledge_status(pledge: &mut PledgeState, status: PledgeStatus) {
    pledge.status = status;
}

/// Build a resolution receipt for success, failure, or timeout paths.
///
/// Reads the resolution status from the pledge's current state.
/// Caller must ensure the pledge status has been updated before calling this.
pub(crate) fn build_resolution_receipt(
    pledge: &PledgeState,
    resolved_by: &str,
    tx_hash: String,
    finalized_at_unix: i64,
) -> ResolutionReceipt {
    ResolutionReceipt {
        pledge_id: pledge.pledge_id.clone(),
        resolved_by: resolved_by.to_string(),
        resolution: pledge.status.clone(),
        tx_hash,
        finalized_at_unix,
    }
}

/// Transfer escrow lamports from the pledge PDA to a destination account.
pub(crate) fn transfer_escrow<'info, D>(
    pledge: &Account<'info, PledgeState>,
    destination: &D,
    system_program: &Program<'info, System>,
    signer_seeds: &[&[&[u8]]],
) -> Result<(), ContractError>
where
    D: ToAccountInfo<'info>,
{
    let cpi_accounts = Transfer {
        from: pledge.to_account_info(),
        to: destination.to_account_info(),
    };

    let cpi_context = CpiContext::new(system_program.to_account_info(), cpi_accounts)
        .with_signer(signer_seeds);

    transfer(cpi_context, pledge.escrow_amount)
        .map_err(|_| ContractError::InsufficientFunds)?;
    Ok(())
}

/// Apply a full resolution flow: validate, transfer, update status, and build receipt.
pub(crate) fn apply_resolution<'info, D>(
    pledge: &mut Account<'info, PledgeState>,
    destination: &D,
    system_program: &Program<'info, System>,
    signer_seeds: &[&[&[u8]]],
    resolved_by: &str,
    status: PledgeStatus,
    tx_hash: String,
    finalized_at_unix: i64,
) -> Result<ResolutionReceipt, ContractError>
where
    D: ToAccountInfo<'info>,
{
    validate_oracle_resolution(pledge, resolved_by)?;
    transfer_escrow(pledge, destination, system_program, signer_seeds)?;
    update_pledge_status(pledge, status);

    Ok(build_resolution_receipt(
        pledge,
        resolved_by,
        tx_hash,
        finalized_at_unix,
    ))
}
