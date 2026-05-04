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

/// Validate that a pledge hasn't been resolved and the signer matches the expected key for this pledge action.
pub(crate) fn validate_pledge_authorized(
    status: &PledgeStatus,
    signer: &str,
    expected_signer: &str,
    error: ContractError,
) -> Result<(), ContractError> {
    if status != &PledgeStatus::Pending {
        return Err(ContractError::AlreadyResolved);
    }

    if signer != expected_signer {
        return Err(error);
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
    oracle_signer: &str,
    status: PledgeStatus,
    tx_hash: String,
    finalized_at_unix: i64,
) -> Result<ResolutionReceipt, ContractError>
where
    D: ToAccountInfo<'info>,
{
    validate_pledge_authorized(
        &pledge.status,
        oracle_signer,
        &pledge.oracle_pubkey,
        ContractError::UnauthorizedOracle
    )?;
    transfer_escrow(pledge, destination, system_program, signer_seeds)?;
    update_pledge_status(pledge, status);

    Ok(build_resolution_receipt(
        pledge,
        oracle_signer,
        tx_hash,
        finalized_at_unix,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pledge(status: PledgeStatus) -> PledgeState {
        PledgeState {
            pledge_id: "pledge-1".to_string(),
            user_pubkey: "user-pubkey".to_string(),
            oracle_pubkey: "oracle-pubkey".to_string(),
            escrow_amount: 42,
            deadline_timestamp: 1_800_000_000,
            status,
        }
    }

    #[test]
    fn validate_pledge_authorized_allows_pending_matching_signer() {
        let pledge = sample_pledge(PledgeStatus::Pending);

        let result = validate_pledge_authorized(
            &pledge.status,
            "oracle-pubkey",
            &pledge.oracle_pubkey,
            ContractError::UnauthorizedOracle,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn validate_pledge_authorized_rejects_wrong_signer() {
        let pledge = sample_pledge(PledgeStatus::Pending);

        let result = validate_pledge_authorized(
            &pledge.status,
            "someone-else",
            &pledge.oracle_pubkey,
            ContractError::UnauthorizedOracle,
        );

        assert_eq!(result, Err(ContractError::UnauthorizedOracle));
    }

    #[test]
    fn validate_pledge_authorized_rejects_resolved_pledge() {
        let pledge = sample_pledge(PledgeStatus::ResolvedSuccess);

        let result = validate_pledge_authorized(
            &pledge.status,
            "oracle-pubkey",
            &pledge.oracle_pubkey,
            ContractError::UnauthorizedOracle,
        );

        assert_eq!(result, Err(ContractError::AlreadyResolved));
    }

    #[test]
    fn update_pledge_status_sets_new_value() {
        let mut pledge = sample_pledge(PledgeStatus::Pending);

        update_pledge_status(&mut pledge, PledgeStatus::ResolvedFailure);

        assert_eq!(pledge.status, PledgeStatus::ResolvedFailure);
    }

    #[test]
    fn build_resolution_receipt_uses_current_pledge_status() {
        let pledge = sample_pledge(PledgeStatus::ResolvedFailure);

        let receipt = build_resolution_receipt(
            &pledge,
            "oracle-pubkey",
            "tx-123".to_string(),
            1_800_000_123,
        );

        assert_eq!(receipt.pledge_id, "pledge-1");
        assert_eq!(receipt.resolved_by, "oracle-pubkey");
        assert_eq!(receipt.resolution, PledgeStatus::ResolvedFailure);
        assert_eq!(receipt.tx_hash, "tx-123");
        assert_eq!(receipt.finalized_at_unix, 1_800_000_123);
    }
}
