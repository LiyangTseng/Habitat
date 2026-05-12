//! Resolve a pledge as failure using oracle authority.

use anchor_lang::{
    prelude::{Account, SystemAccount},
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
    oracle_signer: &str,
    tx_hash: String,
    finalized_at_unix: i64,
) -> Result<ResolutionReceipt, ContractError> {
    apply_resolution(
        pledge,
        penalty_pool,
        oracle_signer,
        PledgeStatus::ResolvedFailure,
        tx_hash,
        finalized_at_unix,
    )
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
            key, is_signer, is_writable, lamports, data, owner, executable,
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

    fn sample_penalty_pool() -> &'static AccountInfo<'static> {
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

    #[test]
    fn resolve_failure_rejects_unauthorized_oracle_before_transfer() {
        let pledge_info = sample_pledge_account(PledgeStatus::Pending);
        let penalty_pool_info = sample_penalty_pool();
        let mut pledge = Account::<PledgeState>::try_from_unchecked(pledge_info).expect("build pledge account");
        let penalty_pool = SystemAccount::try_from(penalty_pool_info).expect("build penalty pool");

        let result = resolve_failure(
            &mut pledge,
            &penalty_pool,
            "not-the-oracle",
            "tx-1".to_string(),
            1_800_000_001,
        );

        assert!(matches!(result, Err(ContractError::UnauthorizedOracle)));
    }

    #[test]
    fn resolve_failure_rejects_already_resolved_pledge() {
        let pledge_info = sample_pledge_account(PledgeStatus::ResolvedFailure);
        let penalty_pool_info = sample_penalty_pool();
        let mut pledge = Account::<PledgeState>::try_from_unchecked(pledge_info).expect("build pledge account");
        let penalty_pool = SystemAccount::try_from(penalty_pool_info).expect("build penalty pool");

        let result = resolve_failure(
            &mut pledge,
            &penalty_pool,
            "oracle-pubkey",
            "tx-2".to_string(),
            1_800_000_002,
        );

        assert!(matches!(result, Err(ContractError::AlreadyResolved)));
    }
}
