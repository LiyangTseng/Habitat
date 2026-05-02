//! Resolve a pledge as success using oracle authority.

use anchor_lang::{
    prelude::{Account, Program, System, SystemAccount},
};
use crate::{
    error::ContractError,
    instructions::pledge_resolution::{apply_resolution},
    state::{pledge_state::PledgeState, resolution_receipt::ResolutionReceipt},
    types::PledgeStatus,
};

pub(crate) fn resolve_success<'info>(
    pledge: &mut Account<'info, PledgeState>,
    user: &SystemAccount<'info>,
    system_program: &Program<'info, System>,
    signer_seeds: &[&[&[u8]]],
    oracle_signer: &str,
    tx_hash: String,
    finalized_at_unix: i64,
) -> Result<ResolutionReceipt, ContractError> {
    apply_resolution(
        pledge,
        user,
        system_program,
        signer_seeds,
        oracle_signer,
        PledgeStatus::ResolvedSuccess,
        tx_hash,
        finalized_at_unix,
    )
}
