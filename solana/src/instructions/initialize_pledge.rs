//! Initialize pledge state for a user's escrow commitment.

use anchor_lang::prelude::*;
use crate::{
    error::ContractError,
    state::pledge_state::PledgeState,
    types::{InitializePledgeArgs, PledgeStatus},
};

pub(crate) fn initialize_pledge(
    args: InitializePledgeArgs,
    owner_pubkey: Pubkey,
) -> std::result::Result<PledgeState, ContractError> {
    validate_initialize_pledge_args(&args)?;

    Ok(build_pending_pledge_state(args, owner_pubkey))
}

fn validate_initialize_pledge_args(args: &InitializePledgeArgs) -> std::result::Result<(), ContractError> {
    if args.pledge_id.is_empty() {
        return Err(ContractError::InvalidInstruction);
    }

    // oracle_pubkey must be set (non-default)
    if args.oracle_pubkey == anchor_lang::solana_program::pubkey::Pubkey::default() {
        return Err(ContractError::InvalidInstruction);
    }

    if args.escrow_amount == 0 {
        return Err(ContractError::InvalidInstruction);
    }

    Ok(())
}

pub(crate) fn validate_initialize_pledge_deadline(
    now_unix: i64,
    deadline_timestamp: i64,
) -> std::result::Result<(), ContractError> {
    // Deadline must be strictly in the future relative to now
    if deadline_timestamp <= now_unix {
        return Err(ContractError::InvalidInstruction);
    }

    Ok(())
}

pub(crate) fn derive_pledge_pda(
    pledge_id: &str,
    signer_pubkey: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let seeds: &[&[u8]] = &[
        b"pledge",
        signer_pubkey.as_ref(),
        pledge_id.as_bytes(),
    ];
    Pubkey::find_program_address(seeds, program_id)
}

fn build_pending_pledge_state(args: InitializePledgeArgs, owner_pubkey: Pubkey) -> PledgeState {
    PledgeState {
        pledge_id: args.pledge_id,
        user_pubkey: owner_pubkey.to_string(),
        oracle_pubkey: args.oracle_pubkey.to_string(),
        escrow_amount: args.escrow_amount,
        deadline_timestamp: args.deadline_timestamp,
        status: PledgeStatus::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_args() -> InitializePledgeArgs {
        InitializePledgeArgs {
            pledge_id: "pledge-1".to_string(),
            oracle_pubkey: anchor_lang::prelude::Pubkey::new_unique(),
            escrow_amount: 1,
            deadline_timestamp: 1_800_000_000,
        }
    }

    fn sample_owner() -> Pubkey {
        anchor_lang::prelude::Pubkey::new_unique()
    }

    #[test]
    fn initialize_pledge_starts_in_pending() {
        let state = initialize_pledge(sample_args(), sample_owner()).expect("should initialize");
        assert_eq!(state.status, PledgeStatus::Pending);
        assert!(!state.user_pubkey.is_empty());
    }

    #[test]
    fn initialize_pledge_rejects_empty_required_fields() {
        let mut args = sample_args();
        args.pledge_id = String::new();
        let err = initialize_pledge(args, sample_owner()).expect_err("should fail");
        assert_eq!(err, ContractError::InvalidInstruction);
    }

    #[test]
    fn initialize_pledge_rejects_zero_escrow() {
        let mut args = sample_args();
        args.escrow_amount = 0;
        let err = initialize_pledge(args, sample_owner()).expect_err("should fail");
        assert_eq!(err, ContractError::InvalidInstruction);
    }

    #[test]
    fn validate_initialize_pledge_deadline_accepts_future_deadline() {
        assert!(validate_initialize_pledge_deadline(1_800_000_000, 1_800_000_001).is_ok());
    }

    #[test]
    fn validate_initialize_pledge_deadline_rejects_current_or_past_deadline() {
        assert_eq!(
            validate_initialize_pledge_deadline(1_800_000_000, 1_800_000_000),
            Err(ContractError::InvalidInstruction)
        );
        assert_eq!(
            validate_initialize_pledge_deadline(1_800_000_000, 1_799_999_999),
            Err(ContractError::InvalidInstruction)
        );
    }

    #[test]
    fn derive_pledge_pda_changes_with_seed_inputs() {
        let owner = sample_owner();
        let program_id = anchor_lang::prelude::Pubkey::new_unique();

        let (pda_a, _) = derive_pledge_pda("pledge-1", &owner, &program_id);
        let (pda_b, _) = derive_pledge_pda("pledge-2", &owner, &program_id);
        let (pda_c, _) = derive_pledge_pda("pledge-1", &anchor_lang::prelude::Pubkey::new_unique(), &program_id);

        assert_ne!(pda_a, pda_b);
        assert_ne!(pda_a, pda_c);
    }

    #[test]
    fn derive_pledge_pda_is_stable_for_same_inputs() {
        let owner = sample_owner();
        let program_id = anchor_lang::prelude::Pubkey::new_unique();

        let (pda_1, bump_1) = derive_pledge_pda("pledge-1", &owner, &program_id);
        let (pda_2, bump_2) = derive_pledge_pda("pledge-1", &owner, &program_id);

        assert_eq!(pda_1, pda_2);
        assert_eq!(bump_1, bump_2);
    }

    #[test]
    fn derive_pledge_pda_bump_matches_program_address() {
        let owner = sample_owner();
        let program_id = anchor_lang::prelude::Pubkey::new_unique();

        let (derived_pda, derived_bump) = derive_pledge_pda("pledge-1", &owner, &program_id);
        let seeds: &[&[u8]] = &[b"pledge", owner.as_ref(), b"pledge-1"];
        let (expected_pda, expected_bump) = anchor_lang::prelude::Pubkey::find_program_address(seeds, &program_id);

        assert_eq!(derived_pda, expected_pda);
        assert_eq!(derived_bump, expected_bump);
    }
}
