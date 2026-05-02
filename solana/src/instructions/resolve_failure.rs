//! Resolve a pledge as failure using oracle authority.

use anchor_lang::{
    prelude::{Account, Program, System, SystemAccount},
};
use crate::{
    error::ContractError,
    instructions::pledge_resolution::{apply_resolution},
    state::{pledge_state::PledgeState, resolution_receipt::ResolutionReceipt},
    types::PledgeStatus,
};

pub(crate) fn resolve_failure<'info>(
    pledge: &mut Account<'info, PledgeState>,
    penalty_pool: &SystemAccount<'info>,
    system_program: &Program<'info, System>,
    signer_seeds: &[&[&[u8]]],
    oracle_signer: &str,
    tx_hash: String,
    finalized_at_unix: i64,
) -> Result<ResolutionReceipt, ContractError> {
    apply_resolution(
        pledge,
        penalty_pool,
        system_program,
        signer_seeds,
        oracle_signer,
        PledgeStatus::ResolvedFailure,
        tx_hash,
        finalized_at_unix,
    )
}
