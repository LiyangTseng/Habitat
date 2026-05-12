# Program ABI Compatibility Contract

This document freezes the ABI contract for `habitat_settlement_program`.

Canonical program id: `BgNjXioQqVNNihH4QCtjthDKAynZLVDixArQgmY7oRM4`

Instructions (discriminator = SHA256("global:" + name)[:8]):
- `initialize_pledge`
  - Accounts: `[payer (signer,writable), pledge (writable, PDA), system_program]`
  - Args: `InitializePledgeArgs { pledge_id: string, oracle_pubkey: pubkey, escrow_amount: u64, deadline_timestamp: i64 }`
- `resolve_success`
  - Accounts: `[oracle (signer), pledge (writable), user (writable), system_program, penalty_pool (writable)]`
  - Args: `resolution_proof: string` (Anchor string)
- `resolve_failure`
  - Accounts: `[oracle (signer), pledge (writable), user (writable), system_program, penalty_pool (writable)]`
  - Args: `resolution_proof: string`
- `claim_timeout`
  - Accounts: `[user (signer,writable), pledge (writable), system_program]`
  - Args: none

Account layouts (Anchor/Borsh):
- `PledgeState`: fields listed in IDL `types.PledgeState` (pledge_id, user_pubkey, oracle_pubkey, escrow_amount:u64, deadline_timestamp:i64, status:u8)

Errors: use numeric codes as in IDL `errors` section; map in Go using `backend/control/solana/abi/errors.go`.

Versioning rules:
- Any change to instruction names, account order, or argument types MUST increment the IDL and be accompanied by a migration and a CI-updated IDL commit.
- Backwards-compatible additive changes (new events, new optional fields) must be documented and reviewed.

CI contract:
- The root CI job `Solana IDL Check` regenerates the IDL and diffs it against the committed copy at `backend/control/solana/idl/habitat_settlement_program.json`.

Signed-off-by: Habitat Team
