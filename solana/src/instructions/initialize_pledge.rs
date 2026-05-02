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
}
