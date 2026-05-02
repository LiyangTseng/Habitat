//! Habitat Solana commitment contract.
//!
//! This crate provides on-chain escrow settlement via a commitment oracle.
//! Instructions: initialize_pledge, resolve_success, resolve_failure, claim_timeout.

// Temporary until Anchor internals migrate from AccountInfo::realloc to AccountInfo::resize.
#![allow(deprecated)]

pub mod config;
pub mod error;
pub mod events;
pub mod instructions;
pub mod state;
pub mod types;

pub use error::ContractError;
pub use state::pledge_state::PledgeState;
pub use types::{InitializePledgeArgs, PledgeStatus};

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, system_instruction};
use instructions::{
    claim_timeout, initialize_pledge, resolve_failure, resolve_success,
};

fn derive_pledge_signer_seeds(pledge_id: &str, user: &Pubkey, program_id: &Pubkey) -> (Vec<Vec<u8>>, u8) {
    let seed_prefix = b"pledge".to_vec();
    let user_bytes = user.to_bytes().to_vec();
    let pledge_bytes = pledge_id.as_bytes().to_vec();
    let seeds_refs: Vec<&[u8]> = vec![seed_prefix.as_slice(), user_bytes.as_slice(), pledge_bytes.as_slice()];
    let (_pda, bump) = Pubkey::find_program_address(&seeds_refs, program_id);
    (vec![seed_prefix, user_bytes, pledge_bytes], bump)
}

struct OracleResolutionPrep {
    oracle_pubkey_str: String,
    pledge_id: String,
    finalized_at_unix: i64,
    seed_vecs: Vec<Vec<u8>>,
    bump_arr: [u8; 1],
}

fn prepare_oracle_resolution(ctx: &Context<ResolvePledge>) -> Result<OracleResolutionPrep> {
    let oracle_pubkey_str = ctx.accounts.oracle.key().to_string();
    let pledge_id = ctx.accounts.pledge.pledge_id.clone();
    let finalized_at_unix = Clock::get()?.unix_timestamp;

    let (seed_vecs, bump) = derive_pledge_signer_seeds(
        &pledge_id,
        &ctx.accounts.user.key(),
        &ctx.program_id,
    );

    Ok(OracleResolutionPrep {
        oracle_pubkey_str,
        pledge_id,
        finalized_at_unix,
        seed_vecs,
        bump_arr: [bump],
    })
}

// ============================================================================
// PROGRAM ID DECLARATION
// ============================================================================

// The public key that identifies this program on-chain.
// Derived from solana/.localnet/keypairs/habitat-settlement-program-keypair.json
declare_id!("GyDwfPjDJ61P8wxQ7EMFyARuoh5Yeuyn58aCSTb2zx28");

// ============================================================================
// ANCHOR PROGRAM ENTRY POINT
// ============================================================================

/// The #[program] attribute marks this module as the Anchor program entrypoint.
/// All public functions inside become RPC-callable instructions.
#[program]
mod habitat_settlement_program {
    use super::*;

    /// Initialize a new pledge escrow with user, oracle, and deadline.
    ///
    /// # Arguments
    /// * `ctx` - Account context containing payer, pledge account, and system program
    /// * `args` - InitializePledgeArgs with pledge_id, amounts, and deadline
    ///
    /// # Returns
    /// Ok(()) on success, Err(ContractError) on validation failure
    pub fn initialize_pledge(
        ctx: Context<InitializePledge>,
        args: InitializePledgeArgs,
    ) -> Result<()> {
        let clock = Clock::get()?;
        initialize_pledge::validate_initialize_pledge_deadline(
            clock.unix_timestamp,
            args.deadline_timestamp,
        )?;

        let payer_key = ctx.accounts.payer.key();
        let (pledge_pda, _pledge_bump) = initialize_pledge::derive_pledge_pda(
            &args.pledge_id,
            &payer_key,
            &ctx.program_id,
        );

        require_keys_eq!(ctx.accounts.pledge.key(), pledge_pda, ContractError::InvalidInstruction);

        // The pledge PDA is the escrow vault for now: initialize it, then fund it.
        let pledge_state = initialize_pledge::initialize_pledge(args, payer_key)?;

        let transfer_ix = system_instruction::transfer(
            &ctx.accounts.payer.key(),
            &ctx.accounts.pledge.key(),
            pledge_state.escrow_amount,
        );
        invoke(
            &transfer_ix,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.pledge.to_account_info(),
            ],
        )?;

        // Store the pledge state in the pledge account (PDA / escrow vault)
        let pledge = &mut ctx.accounts.pledge;
        pledge.pledge_id = pledge_state.pledge_id;
        pledge.user_pubkey = pledge_state.user_pubkey;
        pledge.oracle_pubkey = pledge_state.oracle_pubkey;
        pledge.escrow_amount = pledge_state.escrow_amount;
        pledge.deadline_timestamp = pledge_state.deadline_timestamp;
        pledge.status = pledge_state.status;

        // Emit initialization event for off-chain indexing
        emit!(events::PledgeInitialized {
            pledge_id: pledge.pledge_id.clone(),
            user_pubkey: pledge.user_pubkey.clone(),
            oracle_pubkey: pledge.oracle_pubkey.clone(),
            escrow_amount: pledge.escrow_amount,
            deadline_timestamp: pledge.deadline_timestamp,
        });

        Ok(())
    }

    /// Oracle resolves pledge as successful (user won, oracle confirms).
    ///
    /// # Constraints
    /// * Signer must be the oracle (checked by #[signer] in context)
    /// * Pledge must be in Pending status
    pub fn resolve_success(
        ctx: Context<ResolvePledge>,
        resolution_proof: String,
    ) -> Result<()> {
        let prep = prepare_oracle_resolution(&ctx)?;
        let signer_seeds: &[&[&[u8]]] = &[&[
            prep.seed_vecs[0].as_slice(),
            prep.seed_vecs[1].as_slice(),
            prep.seed_vecs[2].as_slice(),
            &prep.bump_arr,
        ]];

        // Execute resolution logic
        let _receipt = resolve_success::resolve_success(
            &mut ctx.accounts.pledge,
            &ctx.accounts.user,
            &ctx.accounts.system_program,
            signer_seeds,
            &prep.oracle_pubkey_str,
            resolution_proof.clone(),
            prep.finalized_at_unix,
        )?;

        // Emit resolution event
        emit!(events::PledgeResolved {
            pledge_id: prep.pledge_id,
            status: "success".to_string(),
            resolution_proof,
        });

        Ok(())
    }

    /// Oracle resolves pledge as failed (user did not win, oracle confirms).
    ///
    /// # Constraints
    /// * Signer must be the oracle
    /// * Pledge must be in Pending status
    pub fn resolve_failure(
        ctx: Context<ResolvePledge>,
        resolution_proof: String,
    ) -> Result<()> {
        let prep = prepare_oracle_resolution(&ctx)?;
        let signer_seeds: &[&[&[u8]]] = &[&[
            prep.seed_vecs[0].as_slice(),
            prep.seed_vecs[1].as_slice(),
            prep.seed_vecs[2].as_slice(),
            &prep.bump_arr,
        ]];

        // Execute resolution logic
        let _receipt = resolve_failure::resolve_failure(
            &mut ctx.accounts.pledge,
            &ctx.accounts.penalty_pool,
            &ctx.accounts.system_program,
            signer_seeds,
            &prep.oracle_pubkey_str,
            resolution_proof.clone(),
            prep.finalized_at_unix,
        )?;

        // Emit resolution event
        emit!(events::PledgeResolved {
            pledge_id: prep.pledge_id,
            status: "failure".to_string(),
            resolution_proof,
        });

        Ok(())
    }

    /// User claims timeout refund after deadline has passed without oracle resolution.
    ///
    /// # Constraints
    /// * Signer must be the user (owner of pledge)
    /// * Current timestamp must be >= deadline_timestamp
    /// * Pledge must still be in Pending status
    pub fn claim_timeout(ctx: Context<ClaimTimeout>) -> Result<()> {
        let pledge = &mut ctx.accounts.pledge;

        // Validate pledge state and deadline
        if pledge.status != PledgeStatus::Pending {
            return Err(ContractError::AlreadyResolved.into());
        }

        let clock = Clock::get()?;
        if clock.unix_timestamp < pledge.deadline_timestamp {
            return Err(ContractError::TimeoutNotReached.into());
        }

        // Verify user signer
        let user_pubkey_str = ctx.accounts.user.key().to_string();
        require!(
            user_pubkey_str == pledge.user_pubkey,
            ContractError::UnauthorizedUser
        );

        // Execute timeout claim logic
        let _receipt = claim_timeout::claim_timeout(
            pledge,
            &user_pubkey_str,
            clock.unix_timestamp,
            format!("{:?}", clock), // placeholder tx_hash for local dev
        )?;

        Ok(())
    }
}

// ============================================================================
// ACCOUNT CONTEXT STRUCTS
// ============================================================================

/// Context for initializing a new pledge.
///
/// This struct defines:
/// 1. Which accounts are needed
/// 2. Who must sign the transaction (payer)
/// 3. What constraints apply (mut, init, seeds, etc.)
#[derive(Accounts)]
#[instruction(args: InitializePledgeArgs)]
pub struct InitializePledge<'info> {
    /// The transaction payer (usually the user, but could be anyone).
    /// #[signer] means this account must have signed the transaction.
    /// mut (mutable) because Solana deducts rent from it.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The pledge account PDA (Program Derived Address).
    /// #[account(init, ...)] means Anchor will create this account for us.
    /// space = 8 + ... calculates rent based on serialized size.
    /// payer = payer sets who pays for account creation.
    /// seeds/bump = deterministic address derivation from pledge_id.
    #[account(
        init,
        payer = payer,
        space = 8 + PledgeState::LEN,
        seeds = [b"pledge", payer.key().as_ref(), args.pledge_id.as_bytes()],
        bump
    )]
    pub pledge: Account<'info, PledgeState>,

    /// Anchor's built-in system program.
    /// Required for account creation (#[account(init, ...)]).
    pub system_program: Program<'info, System>,
}

/// Context for oracle to resolve a pledge (success or failure).
///
/// Shared by both resolve_success and resolve_failure instructions.
#[derive(Accounts)]
pub struct ResolvePledge<'info> {
    /// The oracle account that resolves pledges.
    /// #[signer] = must sign the transaction.
    #[account(mut)]
    pub oracle: Signer<'info>,

    /// The pledge account to resolve.
    /// mut because we update its status field.
    /// constraint checks oracle_pubkey matches oracle signer.
    #[account(mut)]
    pub pledge: Account<'info, PledgeState>,

    /// The user's wallet (receives funds on success)
    /// - mut: we're sending lamports TO this account (modifies its lamport balance)
    /// - address = constraint: validates this is the correct user from the pledge
    #[account(mut, address = pledge.user_pubkey.parse().unwrap())]
    pub user: SystemAccount<'info>,

    /// Required for CPI transfers
    pub system_program: Program<'info, System>,

    /// The penalty destination for failed oracle resolutions.
    #[account(mut)]
    pub penalty_pool: SystemAccount<'info>,
}

/// Context for user to claim timeout refund.
#[derive(Accounts)]
pub struct ClaimTimeout<'info> {
    /// The user (pledge owner) claiming the timeout.
    /// #[signer] = must sign.
    #[account(mut)]
    pub user: Signer<'info>,

    /// The pledge account.
    /// mut because we update its status.
    #[account(mut)]
    pub pledge: Account<'info, PledgeState>,
}