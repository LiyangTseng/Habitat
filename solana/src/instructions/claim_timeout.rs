//! Allow user timeout reclaim when oracle resolution is unavailable.
//!
//! Implementation guide:
//! - Verify the user signer before touching state.
//! - Require the deadline plus grace period to have passed.
//! - Keep the pledge pending until the refund transfer succeeds.
//! - Transfer escrow lamports back to the user using Anchor CPI.
//! - Mark the pledge as resolved only after the transfer is complete.
//! - Return a receipt for backend reconciliation.

use anchor_lang::prelude::{Account, Program, System, ToAccountInfo};



use crate::{
    config::DEFAULT_TIMEOUT_GRACE_SECONDS,
    error::ContractError,
    instructions::pledge_resolution::{build_resolution_receipt, transfer_escrow, update_pledge_status, validate_pledge_authorized},
    state::{pledge_state::PledgeState, resolution_receipt::ResolutionReceipt},
    types::PledgeStatus,
};

pub(crate) fn claim_timeout<'info, D>(
    pledge: &mut Account<'info, PledgeState>,
    user: &D,
    system_program: &Program<'info, System>,
    signer_seeds: &[&[&[u8]]],
    user_signer: &str,
    tx_hash: String,
    finalized_at_unix: i64,
) -> Result<ResolutionReceipt, ContractError>
where
    D: ToAccountInfo<'info>,
{
    // 1. Confirm the caller is the pledge owner.
    // 2. Confirm the deadline plus grace period has passed.
    // 3. Transfer escrow back to the user.
    // 4. Flip state to the timeout resolution path.
    // 5. Build the timeout receipt.
    let timeout_at = timeout_claim_eligibility_timestamp(pledge.deadline_timestamp);
    if finalized_at_unix < timeout_at {
        return Err(ContractError::TimeoutNotReached);
    }

    validate_pledge_authorized(
        &pledge.status,
        user_signer,
        &pledge.user_pubkey,
        ContractError::UnauthorizedUser
    )?;
    transfer_escrow(pledge, user, system_program, signer_seeds)?;
    update_pledge_status(pledge, PledgeStatus::ResolvedSuccess);

    Ok(build_resolution_receipt(
        pledge,
        user_signer,
        tx_hash,
        finalized_at_unix,
    ))
}

pub(crate) fn timeout_claim_eligibility_timestamp(deadline_timestamp: i64) -> i64 {
    deadline_timestamp + DEFAULT_TIMEOUT_GRACE_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::{prelude::*, solana_program::{account_info::AccountInfo, system_program}};

    fn build_account_info(
        key: Pubkey,
        owner: Pubkey,
        executable: bool,
        is_signer: bool,
        is_writable: bool,
        data: Vec<u8>,
        lamports: u64,
    ) -> &'static AccountInfo<'static> {
        let key = Box::leak(Box::new(key));
        let owner = Box::leak(Box::new(owner));
        let lamports = Box::leak(Box::new(lamports));
        let data = Box::leak(data.into_boxed_slice());
        Box::leak(Box::new(AccountInfo::new(
            key, is_signer, is_writable, lamports, data, owner, executable, 0,
        )))
    }

    fn sample_pledge_account(status: PledgeStatus) -> &'static AccountInfo<'static> {
        let pledge = PledgeState {
            pledge_id: "pledge-1".to_string(),
            user_pubkey: Pubkey::new_unique().to_string(),
            oracle_pubkey: Pubkey::new_unique().to_string(),
            escrow_amount: 42,
            deadline_timestamp: 1_800_000_000,
            status,
        };
        let mut data = Vec::new();
        pledge.try_serialize(&mut data).expect("serialize pledge");
        build_account_info(Pubkey::new_unique(), crate::id(), false, false, false, data, 1)
    }

    fn sample_user_account() -> &'static AccountInfo<'static> {
        build_account_info(
            Pubkey::new_unique(),
            system_program::ID,
            false,
            false,
            true,
            Vec::new(),
            1,
        )
    }

    fn sample_system_program() -> &'static AccountInfo<'static> {
        build_account_info(
            system_program::ID,
            system_program::ID,
            true,
            false,
            false,
            Vec::new(),
            1,
        )
    }

    #[test]
    fn claim_timeout_rejects_before_grace_period() {
        let pledge_info = sample_pledge_account(PledgeStatus::Pending);
        let user_info = sample_user_account();
        let system_program_info = sample_system_program();
        let mut pledge = Account::<PledgeState>::try_from_unchecked(pledge_info).expect("build pledge account");
        let user = SystemAccount::try_from(user_info).expect("build user account");
        let system_program = Program::try_from(system_program_info).expect("build system program");
        let user_signer = pledge.user_pubkey.clone();
        let deadline_before_grace = timeout_claim_eligibility_timestamp(pledge.deadline_timestamp) - 1;
        let signer_seed = [b"seed".as_ref(), &[1u8]];
        let signer_seeds: &[&[&[u8]]] = &[&signer_seed];

        let result = claim_timeout(
            &mut pledge,
            &user,
            &system_program,
            signer_seeds,
            &user_signer,
            "tx-1".to_string(),
            deadline_before_grace,
        );

        assert!(matches!(result, Err(ContractError::TimeoutNotReached)));
    }

    #[test]
    fn claim_timeout_rejects_already_resolved_pledge() {
        let pledge_info = sample_pledge_account(PledgeStatus::ResolvedFailure);
        let user_info = sample_user_account();
        let system_program_info = sample_system_program();
        let mut pledge = Account::<PledgeState>::try_from_unchecked(pledge_info).expect("build pledge account");
        let user = SystemAccount::try_from(user_info).expect("build user account");
        let system_program = Program::try_from(system_program_info).expect("build system program");
        let user_signer = pledge.user_pubkey.clone();
        let signer_seed = [b"seed".as_ref(), &[1u8]];
        let signer_seeds: &[&[&[u8]]] = &[&signer_seed];
        let timeout_at = timeout_claim_eligibility_timestamp(pledge.deadline_timestamp);

        let result = claim_timeout(
            &mut pledge,
            &user,
            &system_program,
            signer_seeds,
            &user_signer,
            "tx-2".to_string(),
            timeout_at,
        );

        assert!(matches!(result, Err(ContractError::AlreadyResolved)));
    }

    #[test]
    fn claim_timeout_rejects_unauthorized_user_after_grace_period() {
        let pledge_info = sample_pledge_account(PledgeStatus::Pending);
        let user_info = sample_user_account();
        let system_program_info = sample_system_program();
        let mut pledge = Account::<PledgeState>::try_from_unchecked(pledge_info).expect("build pledge account");
        let user = SystemAccount::try_from(user_info).expect("build user account");
        let system_program = Program::try_from(system_program_info).expect("build system program");
        let wrong_signer = "wrong-user".to_string();
        let signer_seed = [b"seed".as_ref(), &[1u8]];
        let signer_seeds: &[&[&[u8]]] = &[&signer_seed];
        let timeout_at = timeout_claim_eligibility_timestamp(pledge.deadline_timestamp);

        let result = claim_timeout(
            &mut pledge,
            &user,
            &system_program,
            signer_seeds,
            &wrong_signer,
            "tx-2".to_string(),
            timeout_at,
        );

        assert!(matches!(result, Err(ContractError::UnauthorizedUser)));
    }

    #[test]
    fn claim_timeout_accepts_deadline_boundary_without_timeout_error() {
        let pledge_info = sample_pledge_account(PledgeStatus::Pending);
        let user_info = sample_user_account();
        let system_program_info = sample_system_program();
        let mut pledge = Account::<PledgeState>::try_from_unchecked(pledge_info).expect("build pledge account");
        let user = SystemAccount::try_from(user_info).expect("build user account");
        let system_program = Program::try_from(system_program_info).expect("build system program");
        let user_signer = pledge.user_pubkey.clone();
        let signer_seed = [b"seed".as_ref(), &[1u8]];
        let signer_seeds: &[&[&[u8]]] = &[&signer_seed];
        let timeout_at = timeout_claim_eligibility_timestamp(pledge.deadline_timestamp);

        let result = claim_timeout(
            &mut pledge,
            &user,
            &system_program,
            signer_seeds,
            &user_signer,
            "tx-3".to_string(),
            timeout_at,
        );

        assert!(!matches!(result, Err(ContractError::TimeoutNotReached)));
    }

    #[test]
    fn claim_timeout_accepts_grace_boundary_without_timeout_error() {
        let pledge_info = sample_pledge_account(PledgeStatus::Pending);
        let user_info = sample_user_account();
        let system_program_info = sample_system_program();
        let mut pledge = Account::<PledgeState>::try_from_unchecked(pledge_info).expect("build pledge account");
        let user = SystemAccount::try_from(user_info).expect("build user account");
        let system_program = Program::try_from(system_program_info).expect("build system program");
        let user_signer = pledge.user_pubkey.clone();
        let signer_seed = [b"seed".as_ref(), &[1u8]];
        let signer_seeds: &[&[&[u8]]] = &[&signer_seed];
        let timeout_after_grace = timeout_claim_eligibility_timestamp(pledge.deadline_timestamp) + 1;

        let result = claim_timeout(
            &mut pledge,
            &user,
            &system_program,
            signer_seeds,
            &user_signer,
            "tx-4".to_string(),
            timeout_after_grace,
        );

        assert!(!matches!(result, Err(ContractError::TimeoutNotReached)));
    }

    #[test]
    fn timeout_claim_eligibility_timestamp_adds_grace_period() {
        let deadline_timestamp = 1_800_000_000;

        assert_eq!(
            timeout_claim_eligibility_timestamp(deadline_timestamp),
            deadline_timestamp + DEFAULT_TIMEOUT_GRACE_SECONDS
        );
    }
}
