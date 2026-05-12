# Habitat

An accountability platform for building lasting habits through community commitments and on-chain settlement.

## Project Overview

**Habitat** enables users to make pledges toward habit completion, stake commitments with real consequences (financial penalties), participate in peer accountability groups, and leverage on-chain settlement for transparent, trustless outcomes.

### Core Philosophy

- **Trustless settlement**: Solana smart contracts handle escrow and resolution—no middleman.
- **Multi-provider extensibility**: Support both traditional payments (Stripe) and on-chain settlement (Solana).
- **Community accountability**: Users form groups, track together, and incentivize each other through shared stakes.

---

## Architecture

The project consists of three main components:

```
habitat/
├── backend/          # Go API service (user, grind, payment, messaging)
├── frontend/         # Next.js web + Chrome extension UI
└── solana/          # Rust + Anchor smart contracts (pledge escrow & settlement)
```

### Component Responsibilities

| Component | Role | Location |
|-----------|------|----------|
| **Backend** | REST API, domain logic, database, payment orchestration | `backend/` |
| **Frontend** | Web UI (Next.js), Chrome extension, client-side state | `frontend/` |
| **Solana** | Pledge escrow, oracle-driven settlement, timeout claims | `solana/` |

---

## Smart Contract Testing & Validation

Habitat uses **Solana smart contracts** (written in Rust with Anchor 1.0.2) to manage pledge escrow and on-chain settlement. This section guides collaborators through validating the smart contract quickly.

### Quick Start: 5-Minute Validation

Run this command from the workspace root to validate the entire smart contract in ~5 minutes:

```bash
# 0. Kill any existing validator and reset
pkill -f solana-test-validator; sleep 1

# 1. Start the local Solana validator (background process)
solana-test-validator --reset > /dev/null 2>&1 &
sleep 3

# 2. Build and deploy the program to localnet
cd solana && anchor build && anchor deploy --provider.cluster localnet

# 3. Run the E2E validation test
cd e2e
npm install
# Ensure the test runner knows which keypair to use
export ANCHOR_WALLET=~/.config/solana/id.json
npm run test
```

**Expected Output:**
```
=== Habitat Settlement Program E2E Test ===

Program ID: BgNjXioQqVNNihH4QCtjthDKAynZLVDixArQgmY7oRM4
Provider RPC: http://127.0.0.1:8899

[Account funding...]
✓ Airdrop complete.

Pledge PDA: [derived address]

--- Step 1: Initialize Pledge ---
✓ Initialize TX: [signature]

Verifying pledge account creation...
✓ Pledge account created
  Owner:      BgNjXioQqVNNihH4QCtjthDKAynZLVDixArQgmY7oRM4
  Lamports:   1000000
  Data size:  [N] bytes

--- Step 2: Resolve Success ---
✓ Resolve Success TX: [signature]

--- Step 3: Verify Final State ---
✓ Pledge account exists after resolution
  Owner:      BgNjXioQqVNNihH4QCtjthDKAynZLVDixArQgmY7oRM4
  Lamports:   1000000
  Data size:  [N] bytes

=== Test Complete ===
✓ All steps executed successfully!
✓ Smart contract is responding to transactions.
```

If you see this output, the smart contract is functioning correctly.

### Testing Architecture

The testing strategy uses three layers:

#### Layer 1: Rust Unit Tests (Fastest)
Tests pure logic without on-chain execution.

```bash
cd solana
anchor test  # Runs via cargo test --nocapture
```

**What it validates:**
- Pledge authorization (oracle & user checks)
- State machine transitions (Pending → ResolvedSuccess/ResolvedFailure)
- Timeout deadline enforcement
- Receipt generation

**Current status:** ✓ 24 unit tests passing

#### Layer 2: E2E Integration Tests (Medium)
Tests the full pledge lifecycle against a real local validator.

```bash
cd e2e
npm install
# Ensure the test runner knows which keypair to use
export ANCHOR_WALLET=~/.config/solana/id.json
npm run test
```

**What it validates:**
- Pledge account creation with correct initial state
- Oracle resolution with state updates
- User timeout claim fallback
- On-chain RPC interactions

**Current status:** ✓ Full E2E flow passing

#### Layer 3: Network Integration (Optional)
Deploy to Solana devnet or testnet for broader validation.

```bash
anchor deploy --provider.cluster devnet
# Then update frontend/src/config/config.tsx with devnet program ID
```

### Pledge Lifecycle Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                  Pledge Lifecycle                          │
└─────────────────────────────────────────────────────────────┘

    ┌──────────────┐
    │   PENDING    │  ← User creates pledge, deposits escrow
    └──────┬───────┘
           │
      ┌────┴─────────────────────────────┐
      │                                   │
      ▼                                   ▼
┌──────────────┐              ┌─────────────────┐
│   TIMEOUT    │              │   ORACLE        │
│   CLAIM      │              │   RESOLVES      │
└──────┬───────┘              └────┬────────┬───┘
       │                           │        │
       │                           ▼        ▼
       │                    ┌──────────────┐ ┌──────────────┐
       │                    │   SUCCESS    │ │   FAILURE    │
       │                    │   (user won) │ │  (user lost) │
       │                    └──────┬───────┘ └──────┬───────┘
       │                           │               │
       └───────────────┬───────────┴───────────────┘
                       │
                       ▼
              ┌──────────────────┐
              │   SETTLED        │
              │ (on-chain final) │
              └──────────────────┘
```

### Key Smart Contract Details

**Program ID:** `BgNjXioQqVNNihH4QCtjthDKAynZLVDixArQgmY7oRM4`

**Framework:** Anchor 1.0.2 (Solana smart contract framework)

**IDL Location:** [solana/target/idl/habitat_settlement_program.json](solana/target/idl/habitat_settlement_program.json)

#### Instructions

| Instruction | Purpose | Signer | Effect |
|-------------|---------|--------|--------|
| `initialize_pledge` | Create pledge escrow account | User (payer) | Sets up pledge PDA with metadata |
| `resolve_success` | Oracle confirms user won | Oracle | Updates pledge status to ResolvedSuccess |
| `resolve_failure` | Oracle confirms user lost | Oracle | Updates pledge status to ResolvedFailure |
| `claim_timeout` | User claims refund after deadline | User | Updates pledge status to ResolvedSuccess if deadline passed |

#### Account Structure

```rust
pub struct PledgeState {
    pledge_id: String,           // Unique identifier for this pledge
    user_pubkey: String,         // User's wallet address
    oracle_pubkey: String,       // Oracle authorized to resolve
    escrow_amount: u64,          // Lamports locked in pledge
    deadline_timestamp: i64,     // Unix timestamp for timeout
    status: PledgeStatus,        // Pending | ResolvedSuccess | ResolvedFailure
}
```

### Development Workflow

When making changes to the smart contract:

1. **Edit Rust code** in `solana/src/`
2. **Run unit tests locally:**
   ```bash
   cd solana && anchor test
   ```
3. **Build for deployment:**
   ```bash
   anchor build
   ```
4. **Deploy to localnet:**
   ```bash
   anchor deploy --provider.cluster localnet
   ```
5. **Run E2E validation:**
   ```bash
   cd solana/e2e && npm run test
   ```
---
