# Habitat Platform Milestones Plan

## Goal
Define a milestone-driven roadmap to evolve Habitat from a LeetCode-focused accountability app into a general habit cultivation platform with API-first contracts and multi-source habit signal ingestion.

## Strategic Decision: Multi-Provider Payment Architecture (April 2026)

**Context:** The system is not deployed to production yet. Stripe is not actively used. This affords a rare opportunity to refactor the payment layer for flexibility and scalability from day one.

**Decision:** Prioritize building a canonical, provider-neutral payment interface that supports **both Stripe and Solana simultaneously** as native first-class citizens, rather than bolting on Solana as an afterthought adapter.

**Statement:** The payment layer should target general payment method support from the start, so the backend, schema, and API can support multiple providers without assuming Stripe-only behavior.

**Rationale:**
- DB schema and API can be reshaped now without legacy burden.
- An abstracted adapter pattern enables future payment providers (PayPal, Wise, off-chain settlements) without rework.
- Both card (Stripe) and on-chain (Solana) settlement models are designed into core architecture, not tacked on.
- Testing framework can be built once and reused across all providers via adapter swap tests.

**Execution Impact:**
- P3.5 is now "Multi-Provider Payment Architecture" (refactor interface + DB schema).
- P3.6 is "Solana Smart Contract & Adapter" (implement on-chain stack against the canonical interface).
- Result: Stripe and Solana coexist seamlessly by Q2 2026; future providers integrate in O(days) not O(weeks).

### Roadmap Status Update (April 2026)

**Implementation note:** We kept provider-neutral contracts and adapter boundaries, but simplified runtime wiring to **single-provider-per-service-instance** for readability and operational clarity.

**What changed in code:**
- Payment service now binds exactly one provider adapter at startup.
- Provider selection remains factory-driven (`BuildForProvider`).
- Multi-provider dispatch inside a single service instance was intentionally removed.

**Roadmap implication:**
- The strategic direction remains provider-neutral and extensible.
- The statement "Stripe and Solana coexist seamlessly" should now be read as:
   - coexistence at architecture level (contracts + adapters + schemas), and
   - one active provider binding per deployed service instance, unless multi-provider runtime routing is reintroduced later.

**Revision marker:**
- Any checklist item that assumes simultaneous multi-provider routing in one service process should be treated as deferred, not completed.

---

## Modification Scope Analysis (Current → Target)

### Existing Backend State (as of March 25, 2026)
**Domain entities implemented:**
- ✅ User, Grind (task-centric), Task (leetcode-only with TODO), Participation, Message, InterviewSession, StripePaymentInfo

**NOT implemented:**
- ❌ CompletionEvent (separate from Task)
- ❌ PartnerGroup (implicit in Grind+Participation, not formalized)
- ❌ HabitType/ProviderType abstraction (hardcoded `TaskType: "leetcode"`)
- ❌ OpenAPI contract
- ❌ Multi-provider ingestion pipeline

**Current constraints limiting Milestones 1-3:**
- Task entity marked with TODO comment: "this is not general enough and only designed for leetcode"
- Completion is synchronous and task-centric only
- Grind represents a program+group conflated (no separation of concerns)
- No external provider webhook or API polling infrastructure
- No idempotency or deduplication logic for external signals

---

## Milestone 1: API-First Contract (OpenAPI)

### Objective
Document the expected API surface independent of current implementation so frontend, backend, extension, and future integrations can align on stable contracts.

### Scope
- Create an OpenAPI 3.1 spec as source of truth.
- Define canonical resource models (User, HabitProgram, HabitTask, CompletionEvent, PartnerGroup, Invitation, InterviewSession, PaymentMethod).
- Versioning policy (`/api/v1`, `/api/v2`, and deprecation rules).
- Standard response envelope and error model.

### Modification Scope for Milestone 1
**Backend changes required:**
1. Add `HabitType` and `ProviderType` enums to domain.
2. Create `CompletionEvent` entity and repository interface (separate from Task).
3. Introduce `PartnerGroup` entity to formalize group semantics (refactor from Grind).
4. Extend `Task` to support multi-habit via `habitType` field (breaking schema change).
5. Add `provider` and `evidenceType` tracking to Task completion record.
6. Create `CompletionEventRepository` interface for ingestion queries.

**Frontend impact:**
- No breaking changes to existing login/grind flows if v1/v2 separation maintained.
- Generated API clients from OpenAPI will match new naming (HabitProgram vs Grind).

**Database migrations required:**
- Add `habit_type`, `provider` columns to task table.
- Create `completion_events` table (separate from tasks).
- Refactor grind schema to distinguish program commitment from group membership.

### Why These Resource Models (and Why They Are Relevant Here)

1. `User`
  - Why propose it: identity, authentication, preferences, and ownership are foundational across all flows.
  - Responsibility: account lifecycle, profile data, auth linkage, timezone/notification preferences.
  - Repo relevance: already present as a core domain entity and used by auth, profile updates, invitation routing, and payment ownership.

2. `Grind`
  - Why propose it: this is the generalized form of the current grind concept, needed to expand beyond LeetCode.
  - Responsibility: commitment window, budget/stakes, participants, rules, current status.
  - Repo relevance: directly maps to existing grind logic (create/list/current/get/quit) and allows migration without breaking business semantics.

3. `Task`
  - Why propose it: daily execution unit for each program; supports both coding and non-coding habits.
  - Responsibility: assignment, due date, completion state, evidence, scoring fields.
  - Repo relevance: already implemented as task workflows (`today`, `get`, `finish`) and is the right abstraction for idempotent completion contracts.

4. `CompletionEvent`
  - Why propose it: external provider signals (Duolingo/others) require a normalized event object independent of UI actions.
  - Responsibility: immutable ingestion record with source, evidence type, confidence, idempotency key, dedupe metadata.
  - Repo relevance: currently completion is task-centric and synchronous; this model enables extension/webhook/provider ingestion and async processing.

5. `PartnerGroup`
  - Why propose it: accountability is inherently multi-user and should be modeled independently of a single task/program row.
  - Responsibility: member roster, roles, participation state, shared policies (penalty, reminders, visibility).
  - Repo relevance: existing participation relationships are central to grinds; this formalizes team accountability for future mixed-habit programs.

6. `Invitation`
  - Why propose it: invitation lifecycle needs a stable contract separate from free-form messages.
  - Responsibility: invite token/state machine (pending/accepted/rejected/expired), sender/receiver, target program/group.
  - Repo relevance: invitation flows already exist in message endpoints; explicit resource modeling reduces coupling and clarifies domain transitions.

7. `InterviewSession` (Optional)
  - Why propose it: coaching/interview interactions are stateful, long-running, and distinct from tasks.
  - Responsibility: session status, transcript history, linked task/program, evaluation outputs.
  - Repo relevance: already implemented with start/webhook/response/end; model should remain first-class as platform moves to broader coaching workflows.

8. `PaymentMethod`
  - Why propose it: payment concerns (stakes, penalties, default card) should be explicit and decoupled from user and program records.
  - Responsibility: stored payment instrument metadata, default selection, provider references, validity status, and settlement capability flags.
  - Repo relevance: Stripe payment endpoints and entities already exist; this model stabilizes billing contracts and future settlement logic.
  - Reserved extensibility notes:
    - Keep payment provider-agnostic contract (`provider`, `methodType`, `capabilities`, `settlementNetwork`).
    - Reserve support for non-card rails: wallet, bank transfer, and on-chain settlement.
    - Reserve smart-contract settlement path (Rust contracts) behind a payment adapter boundary.
    - Keep `PaymentMethod` and `PaymentSettlement` separate so current Stripe flows can coexist with future blockchain-based settlement.

### Boundary Notes for OpenAPI Design
- `Grind` and `Task` should be the public contract names, while backend can internally adapt from existing Grind/Task implementations during transition.
- `Invitation` should be modeled as a dedicated resource even if transport still reuses message infrastructure initially.
- `CompletionEvent` should be append-only in API semantics to preserve auditability and fraud review.

### Initial Endpoint Domains (Target Contract)
- Auth: register, login, verify-token, logout, refresh token.
- User Profile: get/update profile, preference settings (timezone, notification rules).
- Programs (formerly grinds): create/list/get/current/quit/progress.
- Tasks: list by date range, today, get by id, complete (idempotent).
- Invitations & Messaging: invite, accept, reject, mark read, sent/received.
- Habit Integrations: connect provider, callback webhook, list linked providers.
- Completion Events: ingest external completion, validate, dedupe, query timeline.
- Payments: save/select methods, intents, penalties, settlements.
- Interviews/Coaching: start session, append response, end/evaluate.
- Health/Observability: liveness, readiness, service metadata.

### Deliverables
1. `openapi.yaml` draft with core schemas and endpoint definitions.
2. API design rules (naming, pagination, idempotency, status codes).
3. Contract review checklist and sign-off notes.

### Exit Criteria
- OpenAPI spec reviewed by frontend + backend.
- At least one generated client consumed by frontend without manual type patches.
- Breaking changes policy documented.

## Milestone 2: Expand to General Habit Cultivation

### Objective
Broaden platform value beyond coding drills into cross-domain habit building while preserving accountability mechanics.

### Expansion Directions
- Learning habits: Duolingo, Anki, reading streaks.
- Wellness habits: workout logs, meditation, sleep consistency.
- Productivity habits: journaling, inbox-zero sessions, focused work blocks.
- Maker habits: writing, open-source commits, design challenges.

### Product Model Update
(Note: I think these can be used as a longer-term modification direction. We can start simple)
- Introduce `HabitType` and `ProviderType` abstraction.
- Program templates by category (e.g., Language Learning 30-day, Fitness 21-day).
- Flexible completion rules:
  - binary complete/incomplete
  - threshold-based (e.g., 15 minutes/day)
  - score-based (e.g., XP >= X)

### Modification Scope for Milestone 2
**Backend changes required:**
1. Create `ProgramTemplate` entity with `habitType` and `completionRule` fields.
2. Extend GrindService (or consider renaming to HabitProgramService) to accept `habitType` on create.
3. Add `completionThreshold` and `ruleType` to Grind/HabitProgram schema.
4. Refactor Task creation logic to support variable completion evidence based on habit type (not just LeetCode problems).
5. Create `HabitTypeProvider` mapping service (duolingo → duration-based, leetcode → problem-based, etc.).

**Frontend impact:**
- Add habit type selector in program creation UI.
- Conditional task display based on habit type (problem card vs workout log vs language lesson, etc.).

**Database migrations required:**
- Add `habit_type`, `completion_rule_type`, `completion_threshold` to grind table.
- Create `program_templates` table with category and rule presets.
- Extend task schema to include habit-type-specific evidence fields (duration, score, status, etc.)

### Deliverables
1. Habit taxonomy document (v1).
2. Program template schema and seed set.
3. UI/UX concept for mixed-habit dashboard.

### Exit Criteria
- At least 3 non-LeetCode habit categories supported in domain model (Duolingo, weekly exercise challenges, GRE vocab memorizing).
- Program creation supports selecting habit category + completion rule.

## Milestone 3: Daily Job Checked Interception (Duolingo + Others)

### Objective
Enable habit completion tracking through external signals, including extension interception and provider integrations.

### Feasibility Notes
This entry is acceptable and strategically strong, with guardrails:
- Prefer official APIs/webhooks where available.
- Use browser interception only for user-authorized, transparent telemetry.
- Never rely solely on brittle DOM selectors for critical correctness.
- Store provenance (`source`, `confidence`, `raw_payload_hash`).

### Ingestion Strategy
1. Provider Connectors (preferred): OAuth/API polling/webhooks.
2. Browser Extension Signals (fallback/augment):
   - Detect completion-like events from allowed domains.
   - Emit normalized `CompletionEvent` payloads to backend.
3. Manual Check-ins (safety net): user self-report when automation unavailable.

### Normalized Completion Event Contract (proposed)
- `eventId` (idempotency key)
- `userId`
- `provider` (leetcode, duolingo, manual, etc.)
- `habitType`
- `completedAt` (UTC)
- `evidenceType` (api, webhook, extension, manual)
- `confidenceScore` (0-1)
- `externalReference`
- `metadata` (provider-specific)

### Risk & Controls
- Privacy: explicit consent, per-provider scopes, revocation.
- Fraud/false positives: dedupe, confidence thresholds, anomaly checks.
- Reliability: retries with backoff, dead-letter queue for failed ingests.
- Compliance: terms-of-service review for each provider.

### Modification Scope for Milestone 3
**Backend changes required (CRITICAL for this milestone):**
1. Create `CompletionEvent` entity and repository (NEW domain model).
2. Add webhook ingestion endpoint: `POST /api/v1/completion-events/ingest` (authenticated + provider-signed).
3. Implement idempotency key deduplication logic (check `eventId` before insert).
4. Add `ProviderConnector` interface for multi-provider support (abstract OAuth/API polling).
5. Create first provider implementation: `DuolingoConnector` (or similar).
6. Add event-to-task reconciliation service to sync external completion signals to task records.
7. Extend `Task` completion to accept external `CompletionEvent` link (not just direct UI finish).
8. Add `provider`, `externalReference`, `confidenceScore` fields to task completion record.

**Extension changes required:**
1. Add detection logic for non-LeetCode completion events (Duolingo streak, etc.).
2. Emit `CompletionEvent` payload structs to backend webhook.
3. Add provider allowlist and user consent flow for each new provider.

**Frontend impact:**
- Task display shows completion source (linkage to external provider).
- Webhook callback handling if implementing OAuth provider flow.

**Database migrations required:**
- Create `completion_events` table with fields: `event_id`, `user_id`, `provider`, `habit_type`, `completed_at`, `evidence_type`, `confidence_score`, `external_reference`, `metadata`, `dedupe_hash`.
- Add unique index on (`event_id`, `provider`, `user_id`) for idempotency.
- Add `linked_completion_event_id` foreign key to tasks table.
- Create `provider_integrations` table to track user OAuth tokens, API credentials, and consent status.

### Deliverables
1. Provider integration policy doc (what is permitted, what is not).
2. Completion ingestion API contract in OpenAPI.
3. First connector POC (Duolingo or equivalent) + manual fallback.

### Exit Criteria
- One external provider integrated end-to-end.
- Extension signal path tested and observable.
- Completion dedupe + idempotency proven in integration tests.

## Milestone 4: Reliability and Scale Foundations

### Objective
Harden system for growth and multi-provider event volume.

### Scope
- Transaction boundaries for multi-step writes.
- Idempotency keys on completion and payment operations.
- Async event pipeline for completion ingestion and AI evaluation.
- Observability: structured logs, metrics, traces.
- Readiness/liveness probes with dependency checks.

### Deliverables
1. Event processing architecture decision record.
2. Reliability dashboard (error rate, ingest lag, dedupe rate).
3. SLO draft for completion processing latency and correctness.
4. Payment abstraction ADR covering single-provider runtime binding, provider-neutral interfaces, and settlement fallbacks (with optional future multi-provider runtime routing).

## Architecture Design: Solana Commitment Contract (Habit Cultivation)

### 1. System Overview
This system enforces habit cultivation through decentralized financial escrows. Users pledge cryptocurrency (USDC/SOL) that is locked in a smart contract. An off-chain backend evaluates habit completion and acts as an Oracle to trigger either a refund (success) or a penalty transfer (failure).

### 2. High-Level Architecture (The Oracle Model)
The system relies on a hybrid on-chain/off-chain architecture to bridge physical-world actions (app activity) with blockchain state.

#### 2.1 On-Chain Components (Solana Program)
* **The Vault (PDA):** Program Derived Addresses are used to create unique, program-controlled escrow accounts for every new habit pledge.
* **State Account:** Stores minimal metadata: `user_pubkey`, `escrow_amount`, `deadline_timestamp`, `status` (Pending/Resolved), and `oracle_pubkey`.
* **Functions:** `initialize_pledge`, `resolve_success`, `resolve_failure`.

#### Key Roles (Why `user_pubkey` and `oracle_pubkey` are both required)

- **`user_pubkey` (owner/beneficiary identity):** identifies whose funds are locked and who is eligible to reclaim funds in user-driven paths (for example timeout claims). This key ties the pledge state to a specific user wallet and prevents other users from claiming unrelated escrows.
- **`oracle_pubkey` (resolver authority):** identifies the trusted backend signer allowed to submit outcome decisions (`resolve_success` or `resolve_failure`). This separates payout decision authority from user ownership authority.
- **Why they must be different:**
   - If only `user_pubkey` existed, users could self-approve outcomes and bypass accountability.
   - If only `oracle_pubkey` existed, user ownership checks for refunds/timeouts would be ambiguous and less safe.
   - Using both keys enforces separation of duties: user owns stake identity, oracle owns resolution authority.

#### 2.2 Off-Chain Components (Go Backend / Oracle Operator)
* **Activity Tracker (Go backend):** Existing backend services track daily user behavior and completion evidence.
* **The Cron/Evaluator (Go backend):** A scheduled job checks deadlines against backend activity logs.
* **The Signer (Go backend infra):** Holds the `oracle_pubkey` private key in secure infrastructure and signs resolution transactions sent to Solana.
* **Rust scope clarification:** Rust is used for the on-chain Solana program; off-chain orchestration and oracle execution live in the Go backend in this repository.

#### 2.3 Transaction Flow
1.  **Pledge:** Client calls `initialize_pledge`. Funds move from User Wallet -> PDA.
2.  **Tracking:** User interacts with the mobile/web app off-chain.
3.  **Resolution:** On deadline, Backend evaluates data.
    * *If Success:* Backend signs `resolve_success`. PDA unlocks, funds return to User.
    * *If Failure:* Backend signs `resolve_failure`. PDA unlocks, funds transfer to Penalty Pool.

### 3. Technical Depth & Best Practices
To ensure a secure and highly optimized Rust implementation, the following standards must be adhered to:

* **Anchor Framework:** All on-chain code must be written using Anchor to utilize secure routing, macro-based validation, and automatic IDL (Interface Definition Language) generation.
* **PDA Security:** Strict `#[account(mut, seeds = [...], bump)]` macros must be used. The `oracle_pubkey` must be hardcoded or strictly initialized so only our trusted backend can sign resolution functions.
* **Precise Space Allocation:** Avoid arbitrary padding. Calculate exact byte sizes for the State Account (e.g., `8 bytes (discriminator) + 32 bytes (Pubkey) + 8 bytes (i64 timestamp) = 48 bytes`) to minimize Rent costs.
* **Graceful Degradation (Timeout):** Implement a `claim_timeout` function. If the Oracle backend crashes and fails to resolve a pledge within X days past the deadline, the user can manually trigger a transaction to reclaim their funds.

## Rust Smart Contract Interface Plan

### Objective
Build the Solana program as the enforcement layer for habit accountability: the contract holds pledged funds, records who owns the pledge, records who is allowed to resolve it, and enforces the outcomes that the current Go backend decides from habit-tracking evidence.

This section is intentionally written for a first-day Rust developer. The goal is not to memorize Solana terminology first. The goal is to understand the data flow, what each on-chain component protects, and how the Anchor program should fit the existing backend without forcing a rewrite.

### Plain-English Model

The product premise is simple:

- A user commits to a habit.
- The user locks a stake as a pledge.
- The current backend tracks whether the user actually followed through.
- The Solana program makes the commitment real by holding the stake and only allowing valid outcomes.
- Social accountability comes from the fact that the stake can be returned on success, reclaimed on timeout, or routed to a penalty path on failure.

In other words, the Rust program is not the habit tracker itself. It is the enforcement layer that makes the habit tracker meaningful.

### Why We Need Each On-Chain Piece

1. `initialize_pledge`
   - Creates a pledge account for one habit commitment.
   - Locks the user’s stake into the program-controlled account.
   - Writes the minimum on-chain facts needed to later prove what this pledge was.

2. `resolve_success`
   - Lets the trusted backend confirm that the user completed the habit.
   - Releases the pledged funds according to the success rule.
   - Prevents the user from self-approving their own outcome.

3. `resolve_failure`
   - Lets the trusted backend mark the pledge as failed when the habit was not completed.
   - Routes the pledge according to the failure rule.
   - Prevents the same pledge from being resolved twice.

4. `claim_timeout`
   - Gives the user a fallback if the backend or oracle path is unavailable.
   - Prevents funds from getting stuck forever.
   - Acts as the safety valve that keeps the system fair.

5. `PledgeState`
   - Stores the canonical pledge record on chain.
   - Tells us who owns the pledge, who can resolve it, how much is locked, when it expires, and what state it is in.

6. `ResolutionReceipt`
   - Stores the outcome record for audit and debugging.
   - Makes it easier for the backend, tests, and future indexers to reconcile what happened.

### How This Fits the Habit-Tracking Premise

The current Go backend already knows how to track behavior off chain. It can tell whether a task was completed, whether a habit signal was observed, and whether an outcome should be considered successful or failed.

The Rust program should not duplicate that logic. Instead, it should:

- accept a pledge from the user,
- remember the pledge facts on chain,
- trust only the authorized backend signer for success/failure decisions,
- and protect the user with a timeout path if the backend is unavailable.

That means the backend stays responsible for habit reasoning, while Solana stays responsible for value enforcement and outcome finality.

### Data Flow Overview

1. The user starts a pledge from the app.
2. The backend prepares the pledge details and the user signs the transaction.
3. `initialize_pledge` creates the pledge account and locks the stake.
4. The backend continues to observe habit progress in the normal Go stack.
5. When the habit is judged complete or failed, the backend submits `resolve_success` or `resolve_failure`.
6. If the backend does not resolve the pledge in time, the user submits `claim_timeout`.
7. The contract emits events and stores state so the backend can reconcile the result.

### Architecture Goals

- Keep the on-chain program boundary narrow and explicit so Go backend integration remains stable.
- Enforce separation of duties between `user_pubkey` (ownership) and `oracle_pubkey` (resolution authority).
- Model the pledge lifecycle in on-chain state (`Pending`, `ResolvedSuccess`, `ResolvedFailure`) with deterministic account derivation.
- Support graceful degradation via `claim_timeout` when backend/oracle resolution is unavailable.
- Keep instruction logic, on-chain state, shared types, and error mapping isolated so each piece stays easy to reason about and test.

### Proposed Directory Structure

```text
solana/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── config.rs
    ├── error.rs
    ├── events.rs
    ├── types.rs
    ├── instructions/
    │   ├── mod.rs
    │   ├── initialize_pledge.rs
    │   ├── resolve_success.rs
    │   ├── resolve_failure.rs
    │   └── claim_timeout.rs
    └── state/
        ├── mod.rs
        ├── pledge_state.rs
        └── resolution_receipt.rs
```

### Module Responsibilities

1. `lib.rs`
   - Crate root and Anchor entrypoint.
   - Owns the account contexts and instruction handlers that glue the program together.
   - Is the place where instruction arguments, PDA seeds, signer checks, and CPI calls meet.

2. `config.rs`
   - Program-level constants such as timeout grace periods, finality thresholds, and network-specific limits.
   - Keeps timing and chain values out of the instruction handlers.

3. `error.rs`
   - Stable program error types.
   - Lets the backend classify failures as validation problems, authorization failures, retryable issues, or terminal settlement failures.

4. `types.rs`
   - Shared request data for pledge instructions.
   - Holds the instruction inputs that are safe to send from the client/backend, such as `pledge_id`, `oracle_pubkey`, `escrow_amount`, and `deadline_timestamp`.
   - This file is the boundary between user intent and on-chain validation.

5. `events.rs`
   - Event payload definitions emitted by the program.
   - Gives the backend and indexers a readable trail of what happened without reading raw account data.

6. `instructions/mod.rs`
   - Instruction entrypoint registry.
   - Central place to expose the supported pledge lifecycle actions.

7. `instructions/initialize_pledge.rs`
   - Validates pledge input and builds the initial pledge state.
   - Derives the deterministic pledge PDA so retries cannot create duplicate pledges.
   - Produces the on-chain pledge record that the handler later stores.

8. `instructions/resolve_success.rs`
   - Checks that the trusted backend signer is allowed to resolve the pledge.
   - Moves the pledge from `Pending` to `ResolvedSuccess`.
   - Returns the success receipt and prepares the escrow return flow.

9. `instructions/resolve_failure.rs`
   - Checks that the trusted backend signer is allowed to resolve the pledge.
   - Moves the pledge from `Pending` to `ResolvedFailure`.
   - Returns the failure receipt and prepares the penalty transfer flow.

10. `instructions/claim_timeout.rs`
    - Checks that the pledge owner is the signer and that the grace period has passed.
    - Moves the pledge into the timeout path when the backend did not resolve it in time.
    - Protects the user from a stuck pledge.

11. `state/mod.rs`
    - Shared state exports for the on-chain accounts used by the program.

12. `state/pledge_state.rs`
    - Stores the canonical pledge record.
    - Should remain small, stable, and explicit so the backend and tests can reliably decode it.
    - Must preserve the ownership key, oracle key, amount, deadline, and status.

13. `state/resolution_receipt.rs`
    - Stores the outcome/audit record.
    - Helps the backend reconcile on-chain decisions with off-chain habit evidence.

### Expected Behavior by Instruction

`initialize_pledge` should:
- reject malformed input,
- derive the same pledge PDA every time for the same user and pledge id,
- create the pledge account once,
- store the pledge facts,
- and leave the pledge in `Pending`.

### Backend Compatibility Rules

- Go backend is the oracle operator for off-chain evaluation and transaction submission.
- Oracle-only instructions (`resolve_success`, `resolve_failure`) must reject non-oracle signers.
- User-only timeout recovery (`claim_timeout`) must reject non-owner signers.
- Pledge account derivation must be deterministic and stable so retries do not create duplicate pledge state.
- Any new on-chain status values must be mirrored in backend mapping before release.

## Settlement Lifecycle Definition (Architecture Clarity)

To prevent confusion about adapter boundaries and settlement operations, this section documents the canonical settlement lifecycle that all payment providers must implement.

### Phases of the Settlement Lifecycle

**Phase 1: Collection (Payment Method Preparation)**
- **Method:** `CreateCollectionIntent(amount, currency)`
- **Purpose:** Prepare the user to make a payment (e.g., generate a Stripe Payment Intent for the frontend, or initialize a Solana pledge account for the backend)
- **Provider-agnostic:** Both card and wallet methods need a collection phase
- **Output:** Client secret (for frontend) or pledge reference (for on-chain)
- **Status:** Pending (user has not confirmed payment yet)

**Phase 2: Settlement (Charge/Move Funds)**
- **Method:** `CreateSettlementIntent(customerId, paymentMethodId, amount, currency)`
- **Purpose:** Actually charge or move funds using a saved payment method
- **Provider-specific execution:**
  - Stripe: off-session charge (customer already saved a card)
  - Solana: pledge account already initialized, settlement is on-chain resolution
- **Output:** Settlement reference and initial status
- **Status:** Pending (charge submitted, not yet confirmed)

**Phase 3: Resolution (Confirm Outcome)**
- **Method:** `ResolveSettlement(reference, resolution: "success" | "failure" | "refunded")`
- **Purpose:** Mark the settlement as succeeded, failed, or refunded
- **Provider-specific execution:**
  - Stripe: confirm charge or initiate refund
  - Solana: oracle submits resolve_success or resolve_failure instruction
- **Status:** Success, Failed, or Refunded

**Phase 4: Status Query (Real-time Sync)**
- **Method:** `QuerySettlementStatus(reference)`
- **Purpose:** Check the current status of a settlement without modifying it
- **Use cases:** Reconciliation, webhook validation, polling for finality
- **Status:** Returns canonical status (Pending, Authorized, Captured, Failed, Refunded, SettledOnChain)

**Phase 5: Disbursement (Send Funds)**
- **Method:** `CreateDisbursement(destinationReference, amount, currency)`
- **Purpose:** Send funds to a beneficiary account (e.g., penalty pool, user refund)
- **Provider-specific execution:**
  - Stripe: transfer to connected account
  - Solana: on-chain SPL token transfer
- **Status:** Completed (same as phase it originated from)

### Adapter Responsibility Separation

**SettlementAdapter (provider-neutral, all providers must implement)**
- `CreateCollectionIntent`: Initialize payment preparation
- `CreateSettlementIntent`: Charge using saved method
- `ResolveSettlement`: Confirm outcome
- `QuerySettlementStatus`: Check real-time status
- `CreateDisbursement`: Send funds

**CardMethodAdapter (card onboarding only, Stripe-specific)**
- `CreatePaymentMethodSetupIntent`: Prepare to save a card
- `EnsurePayerProfile`: Create/retrieve customer profile
- `GetPaymentMethodDetails`: Fetch saved card metadata
- `LinkPaymentMethodToPayer`: Attach card to customer

**WalletMethodAdapter (wallet onboarding only, Solana-specific)**
- `ValidateWalletOwnership`: Verify user owns the wallet
- `NormalizeWalletMethod`: Create payment method record

### Key Principle: PaymentService Should Call Only SettlementAdapter Methods

The payment service (`payment_service.go`) should depend only on `SettlementAdapter` (through `PaymentGatewayAdapter`). Onboarding is handled by separate capability adapters injected from the factory. This ensures:
- Settlement logic is provider-agnostic
- No card-specific or wallet-specific code in the service layer
- Adding a new payment provider requires only a new adapter, not service changes

### Go Payment Adapter Compatibility Bridge Plan

The current Solana pledge contract model is not a 1:1 match with the existing Go payment adapter interface, so a translation layer is required.

#### Interface Gap Summary

- Go adapter expects card-like methods (`CreatePaymentIntent`, `Charge`, `PayBack`, etc.).
- Solana contract exposes pledge lifecycle instructions (`initialize_pledge`, `resolve_success`, `resolve_failure`, `claim_timeout`).
- Backend settlement records expect provider references and canonical settlement statuses.

#### Translation Strategy (Multi-Provider Ready from Day One)

**Key Decision:** Since the system is not deployed yet, Stripe is not in production use. We can refactor the payment layer now to be provider-agnostic and extensible, supporting both Stripe and Solana natively without future rework.

1. Refactor `PaymentGatewayAdapter` interface to be provider-neutral (not card-centric).
2. Keep a single canonical settlement lifecycle that both Stripe and Solana can map into.
3. Use dependency injection to route to Stripe adapter or Solana adapter based on payment method type.
4. Keep mapping rules explicit and testable in one place (no hidden provider-specific conversions in controllers).
5. Design DB schema to support both card metadata and on-chain reference fields natively (not as afterthought).

#### Provider-Neutral Interface (Refactored)

Instead of card-centric methods, use a canonical settlement lifecycle:

1. `AddPaymentMethod(methodType: "card" | "solana_wallet", metadata)`
   - Stripe: stores Stripe token and customer ID.
   - Solana: stores wallet pubkey and pledge account derivation seed.

2. `CreateSettlementIntent(amount, reason, settlementNetwork)`
   - Creates an intent to collect or return funds.
   - For Stripe: initiates a charge or refund via Stripe API.
   - For Solana: prepares deterministic pledge account and `initialize_pledge` transaction context.

3. `ResolveSettlement(settlementId, resolution: "authorized" | "failed" | "refunded")`
   - Marks settlement as resolved and persists outcome.
   - For Stripe: calls Stripe API to confirm/refund charge.
   - For Solana: submits `resolve_success`, `resolve_failure`, or timeout claim instruction.

4. `QuerySettlementStatus(settlementId, networkSpecific)`
   - Returns canonical status (`pending`, `captured`, `failed`, `settled_onchain`) plus provider-specific metadata.

**Provider-Specific Implementation Details:**

- **Stripe Adapter**: implements above interface by calling Stripe API; persists `stripeChargeId`, `stripeCustomerId`.
- **Solana Adapter**: implements above interface by building/signing Solana transactions; persists `solanaVault`, `solanaTransactionHash`, `pledgeStatus`.
- **Controllers**: only call the provider-neutral interface; no provider-aware logic in handlers.

#### Canonical Settlement Status Model

**Unified Status Enum** (provider-independent):
- `pending`: intent created, awaiting capture.
- `authorized`: payment method verified and hold placed (card-like only).
- `captured`: funds deducted from source.
- `failed`: settlement attempt failed.
- `refunded`: previously captured funds returned.
- `settled_onchain`: transaction finalized on blockchain (Solana-specific milestone).

**Provider Mapping:**

| Provider | pending | authorized | captured | failed | refunded | settled_onchain |
|----------|---------|------------|----------|--------|----------|-----------------|
| Stripe | intent created | charge authorized | charge captured | charge declined | refund created | N/A |
| Solana | account initialized | N/A | pledge resolved success, waiting confirmation | pledge resolve failed | timeout claim | transaction confirmed (K+ slots) |

**Reference Persistence:**
- Stripe: `providerReference` = `stripeChargeId`, `settlementProof` = Stripe invoice/receipt.
- Solana: `providerReference` = transaction hash, `settlementProof` = finalized blockhash + signature.

#### Operational Safety Requirements

- All oracle-driven transitions must be idempotent with deterministic pledge identifiers.
- Transaction submission must separate stages: sign, broadcast, confirm, then persist final status.
- Failed broadcasts must not be treated as successful settlement until confirmation exists.

### How We Should Build P3.6 A1-A6

This is the practical build order for the Rust program and the backend bridge. The intent is to keep each phase small enough for a Python/C++ developer learning Rust to verify quickly.

#### A1: Anchor Bootstrapping

Goal: make the Rust program look and behave like an Anchor program before adding deeper business rules.

- Define the program entrypoint in `lib.rs`.
- Define the account structs that Anchor needs to compile and validate instructions.
- Define the `PledgeState` and `ResolutionReceipt` accounts with explicit sizes.
- Define stable error codes and events early so the backend can integrate against them.

Why this matters:
- The backend must know the account shapes and instruction names before it can drive the contract.
- You want compile-time structure first, business logic second.
- This keeps the program understandable before any CPI or payment movement is added.

#### A2: Core Instruction Behavior

Goal: make each instruction enforce one clear rule.

- `initialize_pledge` should create the pledge and lock the stake.
- `resolve_success` should let only the authorized backend finalize success.
- `resolve_failure` should let only the authorized backend finalize failure.
- `claim_timeout` should protect the user if the backend never resolves the pledge.

Why this matters:
- Habit tracking is only useful if the outcome cannot be changed by the wrong signer.
- The contract should be boring and predictable: one instruction, one responsibility.

#### A3: Security and Invariant Tests

Goal: prove the contract rejects the wrong signer, wrong state, and repeated resolution.

- Unauthorized signer tests.
- Duplicate resolution tests.
- PDA mismatch tests.
- Amount and deadline edge cases.

Why this matters:
- This is where the trust model gets checked.
- If A3 is weak, the habit-enforcement promise is weak.

#### A4: IDL and Go Bridge

Goal: make the backend speak the same contract language as the Rust program.

- Generate the Anchor IDL.
- Mirror instruction arguments in Go.
- Mirror PDA derivation in Go.
- Freeze the ABI contract.

Why this matters:
- The backend already owns habit decisions.
- It now needs a stable way to turn those decisions into signed Solana transactions.

#### A5: Go Adapter Implementation

Goal: let the current backend drive Solana the same way it already drives payment flows.

- Map wallet/payment method intent into pledge intent.
- Build the initialize / resolve / timeout transactions.
- Submit transactions and track confirmation.
- Persist proofs and final statuses in the existing backend.

Why this matters:
- The product stays one system: habit tracking in Go, enforcement in Solana.
- The backend remains the source of truth for user experience and reconciliation.

#### A6: Integration, Observability, and Runbooks

Goal: make the system operable, debuggable, and safe to run.

- Run local validator integration tests.
- Add adapter swap tests.
- Add logs and metrics for sign/broadcast/confirm.
- Document what to do when a transaction is stuck.

Why this matters:
- A decentralized enforcement layer is only useful if the team can operate it safely.
- This is where product reliability meets on-chain reality.

### Implementation Order (Multi-Provider Priority)

**Phase 1: Refactor Payment Layer for Multi-Provider Support**
1. Redesign `PaymentGatewayAdapter` interface to be provider-neutral (canonical settlement lifecycle, not card-centric).
2. Update DB schema to support both `card` and `solana_wallet` method types natively.
3. Add `methodType`, `settlementNetwork`, `providerReference`, `settlementProof` columns to payments schema.
4. Refactor Stripe adapter implementation to use new interface (backward compatible at API level).
5. Write adapter contract tests to ensure both old Stripe and new Solana adapters can be swapped via DI.

**Phase 2: Implement Solana Adapter Against Canonical Interface**
1. Create the Rust crate skeleton and shared types.
2. Add the error model and event payloads.
3. Implement `initialize_pledge` and pledge state account.
4. Implement `resolve_success` and `resolve_failure` with oracle signer checks.
5. Implement `claim_timeout` with user signer + deadline checks.
6. Build the Solana adapter in Go that translates canonical interface calls into Solana instructions.

**Phase 3: Integration & Testing**
1. Add deterministic PDA derivation tests.
2. Add signer authorization tests (oracle-only, user-only, timeout paths).
3. Add adapter swap tests (prove same test suite passes with Stripe adapter and Solana adapter).
4. Add end-to-end retry/idempotency tests for both adapters.
5. Document the adapter pattern and onboarding for future providers.

## Milestone 5: Product Intelligence Layer

### Objective
Turn tracked habits into personalized coaching and retention loops.

### Scope
- Streak health scoring and relapse prediction.
- Adaptive daily suggestions based on completion confidence and behavior trends.
- Accountability nudges and partner escalation policies.

### Deliverables
1. Habit analytics schema and event marts.
2. Weekly coaching summary endpoint.
3. Recommendation rule engine v1.

## Cross-Cutting Decisions Needed
1. OpenAPI tooling choice (Swagger Editor + CI validation, or Stoplight).
2. Event bus choice (Redis Streams, Kafka, or managed queue).
3. Provider priority order after LeetCode (Duolingo first recommended).
4. Consent and data-retention policy for external activity ingestion.
5. **Payment provider strategy: Multi-provider-ready from day one (DECIDED: Yes; refactor now while unfettered by production constraints).**
   - Canonical settlement interface to support Stripe and Solana simultaneously.
   - DB schema designed for both card and on-chain settlement natively.
   - Adapter pattern enables future payment providers without core rework.
6. Smart-contract stack decision (Rust target chain/framework, custody model, and settlement finality policy).
   - Anchor framework with IDL generation for production-ready on-chain code.
   - Solana as initial network (extensible to other EVM/non-EVM blockchains via adapter).
   - Settlement finality: K-slot confirmation threshold for operational certainty.

## Suggested Execution Order (Next 4-6 Weeks)
1. Week 1: Draft `openapi.yaml` and approve core schemas.
2. Week 2: Add completion ingestion endpoints + idempotency model.
3. Week 3: Implement first provider connector + extension fallback path.
4. Week 4: Add observability + retry/dedupe controls.
5. Week 5-6: Launch mixed-habit templates and dashboard updates.

## Definition of Done for This Planning Phase
- API-first direction documented.
- Non-LeetCode platform expansion direction documented.
- Daily job checked interception strategy documented with risks and controls.
- Clear milestone order and deliverables established.

## Status Snapshot

### Successfully Implemented So Far
- Go-native unit tests for domain and service layers are passing.
- Repository integration tests run against real PostgreSQL via Testcontainers.
- API integration tests use `httptest` and are passing.
- `openapi.yaml` exists and covers the current API surface at a useful draft level.
- Docker-based local development support exists in `deploy/docker-compose.yml`.
- The test harness can discover `.env` files more reliably than before.

### Still Gaps in the Current Codebase
- The migration helper is intentionally lean now, but there is no manual migration CLI for ad hoc rollback/status checks.
- CompletionEvent ingestion and HabitType/ProviderType abstractions are still not implemented in backend code.

## Roadmap

Last updated: 2026-04-22
- Overall status: P0-P3 completed; P3.5 completed; P3.6 started (skeleton aligned), implementation pending.

### P0: Immediate Stabilization
- [x] Extract a shared router registration function so production and tests cannot drift.
- [x] Replace `AutoMigrate()` with versioned migrations for production-safe schema control.
- [x] Expand CI to run `go fmt`, `golangci-lint`, `go test -race`, integration tests, and OpenAPI validation.
- [x] Add a contract check for key auth, grind, task, payment, and message responses.

### P1: Contract and Boundary Cleanup
- [x] Align `openapi.yaml` with every implemented runtime route, including `/api/v2` auth endpoints.
- [x] Normalize error responses so handlers emit one consistent JSON error shape.
- [x] Introduce or finalize a shared API bootstrap path for both `main.go` and the test harness.
- [x] Review the repository interfaces and service boundaries for any accidental infrastructure leakage.

### P2: Payment Contract and Adapter Foundation
- [x] Refactor Stripe logic behind the adapter so controllers/services are provider-neutral.
- [x] Replace Stripe-shaped payment storage with a canonical provider-neutral repo schema.
- [x] Extend `openapi.yaml` payment schemas for provider-discriminated responses (`card` and `on_chain`).
- [x] Introduce canonical settlement statuses and references (provider-neutral lifecycle model).
- [x] Add a Solana adapter stub (feature-flagged, no mainnet dependency yet).

### P3: Settlement Reliability and Rust-Ready Backbone
- [x] Add idempotency keys for payment intent, method selection, charge, and settlement flows.
- [x] Implement reconciliation workflow for pending/failed/duplicate settlements.
- [x] Persist on-chain readiness fields (`provider`, `network`, `txHash`, `contractAddress`, `finalizedAt`, `settlementProof`).
- [x] Add adapter-level integration tests for settlement lifecycle (`authorized`, `failed`, `settled`, `retried`).
- [x] Document Rust smart-contract interface boundary (instructions/events/error codes expected by backend adapter).

### P3.5: Multi-Provider Payment Architecture (Payment Layer Refactoring)
- [x] Refactor `PaymentGatewayAdapter` interface to be provider-neutral (canonical settlement lifecycle, not card-centric).
- [x] Update DB schema to support `methodType` ("card" | "solana_wallet"), `settlementNetwork`, `providerReference`, `settlementProof` natively.
- [x] Implement new Stripe adapter against canonical interface (prove backward compatibility at API level).
- [x] Design canonical settlement status enum (pending, authorized, captured, failed, refunded, settled_onchain).
- [x] Add dependency injection pattern for provider adapter binding at startup (single provider per service instance).
- [ ] (Optional future) Reintroduce in-service multi-provider runtime dispatch by payment method type if product requirements need simultaneous provider routing.
- [x] Write adapter contract tests to prove both Stripe and Solana adapters can be swapped via same test suite.

### P3.6: Solana Smart Contract & Adapter Implementation
- [x] Complete Phase A1 (Anchor bootstrapping) in the detailed P3.6-A checklist below.
- [x] Complete Phase A2 (core instruction implementation) in the detailed P3.6-A checklist below.
- [x] Complete Phase A3 (security and invariants tests) in the detailed P3.6-A checklist below.
- [x] Complete Phase A4 (IDL + Go contract bridge) in the detailed P3.6-A checklist below.
- [x] Complete Phase A5 (Go Solana adapter implementation) in the detailed P3.6-A checklist below.
- [ ] Complete Phase A6 (integration, observability, runbooks) in the detailed P3.6-A checklist below.

#### P3.6-A: Rust + Anchor Learning-to-Delivery Backlog (Detailed)

Goal: deliver production-safe Solana settlement support in Go backend while learning Rust/Anchor incrementally through implementation.

**Phase A0 - Tooling + Local Environment (1-2 days)**
- [x] Install and pin toolchain versions in docs: rustc 1.94.0, solana-cli 3.1.14, anchor-cli 1.0.1, Node v25.0.0, yarn 1.22.22 (pnpm optional when using yarn).
- [x] Add `solana/Anchor.toml` and workspace config (`programs.localnet`, `provider.cluster`, `provider.wallet`, test script).
- [x] Add solana-folder-level `Makefile` targets for `build`, `test`, `deploy-local`, and `idl` generation.
- [x] Add local validator bootstrap script with deterministic test keypairs and airdrop helpers.
- [x] Add a short "Rust and Anchor quickstart" section to README for first-time contributors.

**Phase A1 - Anchor Program Bootstrapping (2-3 days)**
- [x] Convert `solana/src/lib.rs` to Anchor `#[program]` entrypoint with explicit instruction handlers.
- [x] Add Anchor account structs with `#[derive(Accounts)]` for each instruction context.
- [x] Add `#[account]` state struct for pledge with deterministic `LEN` sizing constant.
- [x] Add `#[account]` state struct for resolution receipt with deterministic `LEN` sizing constant.
- [x] Add canonical custom errors with `#[error_code]` and stable numeric mapping documented for Go.
- [x] Add `#[event]` emissions for `PledgeInitialized`, `PledgeResolved`, and `PledgeTimeoutClaimed`.

What A1 should give you in practice:
- A compilable program skeleton.
- Clear account shapes.
- A stable ABI for the backend to target.
- Enough structure that the next steps are about behavior, not chasing compiler errors.

**Phase A2 - Core Instruction Implementation (3-5 days)**
- [x] Implement `initialize_pledge` with PDA derivation, vault funding, deadline validation, and duplicate prevention.
- [x] Add A2 foundation scaffolding interfaces in instruction modules (`resolve_success`, `resolve_failure`, `claim_timeout`) with no-op transfer hooks for later CPI implementation.
- [x] Implement `resolve_success` with oracle signer verification and escrow return to user.
- [x] Implement `resolve_failure` with oracle signer verification and transfer to penalty destination.
- [x] Implement `claim_timeout` with owner-only checks, grace-period checks, and single-resolution enforcement.
- [x] Ensure all state transitions are one-way and reject invalid transition graphs.

What A2 should give you in practice:
- A pledge that can be created once and only once.
- A path for the backend to settle outcomes.
- A fallback path for the user if the backend never responds.
- No ambiguity about who is allowed to sign each action.

**Phase A3 - Program Security + Invariant Tests (3-4 days)**
- [x] Add negative tests for unauthorized signers across all resolve paths.
- [x] Add replay/double-resolve tests to enforce idempotent settlement behavior.
- [x] Add PDA seed collision tests and bump mismatch tests.
- [x] Add amount/rent edge case tests (zero amount, insufficient lamports, stale accounts) - harness stub skeleton created.
- [x] Add timeout boundary tests (`deadline - 1`, `deadline`, `deadline + grace`).

**Phase A4 - IDL + Go Contract Bridge (2-3 days)**
- [x] Generate Anchor IDL and commit versioned artifact under `solana/target/idl` (or checked-in copy under backend-owned path).
- [x] Create a Go-side typed mapping package for instruction args, account metas, and error code translation.
- [x] Add deterministic PDA derivation helper in Go mirroring Anchor seed scheme exactly.
- [x] Define and freeze a "Program ABI Compatibility Contract" doc (instruction names, account order, argument encoding).
- [x] Add CI check that fails when Rust instruction signatures drift without IDL refresh.

What A4 should give you in practice:
- The Go backend can generate the exact instruction payloads the program expects.
- The backend and the program stop drifting apart.
- Frontend/backend assumptions can be checked against the same contract.

**Phase A5 - Go Solana Adapter Implementation + Payment Method Onboarding Refactoring (3-5 days)**

**A5 Priority 1: Payment Method Onboarding Refactor (COMPLETED - 2026-05-15)**
- [x] Remove Stripe-only `SaveCard` endpoint and method from service interface.
- [x] Remove `CreateSaveCardIntent` endpoint (Stripe setup intent specific).
- [x] Add unified `AddPaymentMethod(methodType: "card" | "solana_wallet", payload)` to service interface.
- [x] Implement internal dispatch logic in service layer by methodType discriminator.
- [x] Create `addStripeCardMethod()` private handler (calls Stripe adapter, creates payer profile, links method, persists to repo).
- [x] Create `addSolanaWalletMethod()` private handler (validates wallet address/network, creates payment method info, persists to repo).
- [x] Create `AddPaymentMethodDTO` with discriminator fields (methodType, cardPaymentMethodID, walletAddress, network, programID).
- [x] Update payment controller to expose unified `POST /api/v1/payments/methods` endpoint.
- [x] Update `openapi.yaml` to remove `/payments/save-card-intent` and `/payments/save-card`, replace with unified `/payments/methods` POST endpoint with method-type examples.
- [x] Consolidate GET `/payments/methods` (list available) and POST `/payments/methods` (add new) under same endpoint path.
- [x] Update router to wire new endpoint and remove old routes.
- [x] Verify all tests pass (6/6 adapter contract tests passing).

Rationale: With no production deployment yet, removing SaveCard eliminates backward-compatibility burden and prepares the API contract for clean multi-provider onboarding. Both card and wallet methods now flow through the same endpoint with discriminated payloads, making future provider expansion seamless.

**A5 Priority 2: Go Solana Adapter Implementation (PARTIALLY COMPLETED)**
- [x] Implement `AddPaymentMethod` mapping for `solana_wallet` metadata validation and wallet ownership checks.
- [x] Implement `CreateSettlementIntent` to prepare pledge/account context and persist canonical pending settlement row.
- [x] Implement `ResolveSettlement` to execute sign -> broadcast -> confirm pipeline with retry-safe idempotency key (non-custodial sign-then-submit flow implemented).
- [x] Implement `QuerySettlementStatus` with canonical status mapping (`pending`, `failed`, `settled_onchain`, etc.).
- [x] Add provider-specific proof persistence (`signature`, `slot`, `finalized_blockhash`, explorer URL).

Current implementation status:
- The adapter and service now provide a non-custodial sign-then-submit flow: the Solana adapter builds an unsigned pledge transaction (`CreateCollectionIntent`), the service persists a canonical pending `PaymentSettlement`, and the `SubmitSolanaSignedTransaction` path broadcasts, confirms (with retry) and persists `TxHash`/`SettlementProof`/`FinalizedAtUnix`.
- `CreateSettlementIntent` and the pending-settlement persistence are implemented in the service and adapter.
- `ResolveSettlement` semantics are supported via the sign-then-submit pipeline (the adapter exposes a `ResolveSettlement` method, and `SubmitSolanaSignedTransaction` updates settlement status and proof after on-chain confirmation).
- `QuerySettlementStatus` is the remaining work: the adapter currently returns a static "settled_onchain" placeholder; it needs RPC-driven confirmation, slot/finality mapping, and richer proof metadata (slot, finalized blockhash, explorer URL) for canonical status queries and reconciliation.

What A5 should give you in practice:
- The existing Go backend can act as the oracle operator.
- The app can create a pledge from the same service that already understands user identity and habit state.
- Solana becomes one more settlement rail, not a separate product.

**Phase A6 - Integration, Observability, and Runbooks (2-4 days)**
- [ ] Add local integration tests running Go adapter against local validator + deployed local Anchor program.
- [ ] Add adapter swap integration suite (same test contract passes for Stripe and Solana adapters).
- [ ] Add structured logs and metrics around sign/broadcast/confirm latency and failure categories.
- [ ] Add operator runbook for stuck transactions, RPC failover, and manual reconciliation procedure.
- [ ] Add staged rollout checklist: localnet -> devnet -> production gate criteria.

Current checkpoint status:
- Local validator is running.
- Local Anchor program deployment is complete.
- Backend startup still needed a Solana env wiring fix; once that is in place, run the integration tests against localnet.

What A6 should give you in practice:
- Confidence that the system behaves the same under retries and failures.
- Visibility into the on-chain transaction lifecycle.
- A realistic operational path for running the product.

**Learning Outcomes to Track While Implementing**
- [ ] Rust fundamentals used in this repo: ownership/borrowing, error handling, enums, serialization.
- [ ] Solana runtime fundamentals: account model, rent, PDAs, signer model, CPI boundaries.
- [ ] Anchor fundamentals: macros, account constraints, IDL, test harness, deployment model.
- [ ] Backend integration fundamentals: deterministic derivation parity, confirmation/finality policy, idempotent retries.

### P4: Domain Expansion
- [ ] Introduce `HabitType` and `ProviderType` in the domain model.
- [ ] Add `CompletionEvent` as a first-class entity with a repository and ingestion service.
- [ ] Refactor `Grind` toward a more general habit-program model without breaking current flows.
- [ ] Add provider-agnostic completion evidence fields so non-LeetCode habits can be tracked cleanly.

### P5: Product Growth
- [ ] Implement the first external provider connector and ingestion path.
- [ ] Add mixed-habit templates and program creation support.
- [ ] Build observability and deduplication controls for completion events.
- [ ] Expand end-to-end coverage to the full happy path: register -> login -> create grind -> get task -> finish task.

### API Endpoint Misalignment Summary

The following discrepancies were identified between the `openapi.yaml` specification and the implemented backend routes:

1. **Endpoint: `/grinds` (DELETE)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `DELETE /api/v1/grinds/delete-all` exists in the backend.
   - **Actions**: should add this in the openapi spec.

2. **Endpoint: `/interviews/llm` (POST)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `POST /api/v1/interviews/llm` exists in the backend.
   - **Actions**: we'll review this later

3. **Endpoint: `/interviews/{id}/response` (POST)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `POST /api/v1/interviews/:id/response` exists in the backend.
   - **Actions**: we'll review this later

4. **Endpoint: `/payments/save-card-intent` (POST)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `POST /api/v1/payments/save-card-intent` exists in the backend.

5. **Endpoint: `/payments/save-card` (POST)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `POST /api/v1/payments/save-card` exists in the backend.

6. **Endpoint: `/payments/force-charging` (POST)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `POST /api/v1/payments/force-charging` exists in the backend.

7. **Endpoint: `/payments/methods/select-default` (POST)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `POST /api/v1/payments/methods/select-default` exists in the backend.

8. **Endpoint: `/users/exists` (GET)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `GET /api/v1/users/exists` exists in the backend.
   - **Actions**: should add this in the openapi spec.

9. **Endpoint: `/messages/{id}/read` (POST)**
   - **Spec:** Not defined in `openapi.yaml`.
   - **Implementation:** `POST /api/v1/messages/:id/read` exists in the backend.
   - **Actions**: should add this in the openapi spec.

10. **Endpoint: `/login` (POST)**
    - **Spec:** Defined for `/api/v1/login`.
    - **Implementation:** `/api/v2/login` also exists but is not documented in `openapi.yaml`.
    - **Actions**: should add the missing parts in the openapi spec.

11. **Endpoint: `/verify-token` (GET)**
    - **Spec:** Defined for `/api/v1/verify-token`.
    - **Implementation:** `/api/v2/verify-token` also exists but is not documented in `openapi.yaml`.
    - **Actions**: should add the missing parts in the openapi spec.

### Recommendations
- Update `openapi.yaml` to include undocumented endpoints or remove unused routes from the backend.
- Ensure versioning consistency between `/api/v1` and `/api/v2` endpoints.

### Solana Smart Contract Integration (See P3.5 and P3.6)

For detailed implementation guidance on integrating Solana pledge contracts with the backend, see:
- **P3.5: Multi-Provider Payment Architecture** – refactoring the payment layer to support both Stripe and Solana as first-class adapter implementations.
- **P3.6: Solana Smart Contract & Adapter Implementation** – building the Rust on-chain program and Go-side adapter that implements the canonical settlement interface.

The pledge/oracle architecture is documented under "Architecture Design: Solana Commitment Contract" earlier in this plan.

---

## API Endpoints Configuration Analysis

### Backend API Structure (`/api/v1`)

The backend implements 32 endpoints across 6 functional domains using the Gin web framework with PostgreSQL:

**Auth & Users (5 endpoints)**
- `POST /register` - Register new user
- `POST /login` - Login user
- `GET /verify-token` - Verify JWT token
- `PATCH /users/update-profile` - Update user profile
- `GET /users/exists` - Check if user exists

**Grinds (6 endpoints)**
- `POST /grinds` - Create new grind
- `GET /grinds` - List all user grinds
- `GET /grinds/current` - Get current ongoing grind
- `GET /grinds/{grindId}` - Get grind by ID
- `DELETE /grinds/{grindId}` - Quit grind
- `GET /grinds/{grindId}/progress` - Get grind progress and leaderboard

**Tasks (3 endpoints)**
- `POST /tasks/finish` - Mark task as completed
- `GET /tasks/today` - Get today's task
- `GET /tasks/{taskId}` - Get task by ID with optional query parameter `setProblem`

**Interviews (4 endpoints)**
- `POST /interviews/start` - Start interview session
- `POST /interviews/{sessionId}/end` - End interview and trigger evaluation
- `POST /interviews/{sessionId}/response` - Save agent response
- `POST /interviews/llm` - LLM webhook endpoint

**Payments (7 endpoints)**
- `POST /payments/payment-intent` - Create payment intent
- `POST /payments/save-card-intent` - Create save card intent
- `POST /payments/save-card` - Save card
- `POST /payments/force-charging` - Force charge overdue penalties
- `GET /payments/methods` - List saved payment methods
- `POST /payments/methods/select-default` - Select default payment method

**Messages & Invitations (6 endpoints)**
- `GET /messages` - Get received messages
- `GET /messages/sent` - Get sent messages
- `POST /messages/invitation` - Create invitation
- `POST /messages/{messageId}/invitation/accept` - Accept invitation
- `POST /messages/{messageId}/invitation/reject` - Reject invitation
- `POST /messages/{messageId}/read` - Mark message as read

**Health (1 endpoint)**
- `GET /ping` - Health check

### Versioning Strategy

- **`/api/v1`**: Single-grind focused endpoints (current production)
- **`/api/v2`**: Multi-habit support (future)
  - `POST /login` - V2 login with multi-grind support
  - `GET /verify-token` - V2 token verification

### Implementation Architecture

**Technology Stack:**
- Framework: Gin (Go web framework)
- Database: PostgreSQL with pgx driver
- Authentication: JWT Bearer tokens
- CORS: Enabled for cross-origin requests

**Dependency Injection:**
- Service layer: Business logic orchestration
- Repository pattern: Data access abstraction
- Controller layer: HTTP handler adaptation
- Middleware: CORS, request validation, error handling

### API Testing Status

**Deprecating JavaScript Dredd - Switching to Go-Based Approach:**

**Why not JavaScript Dredd for a Go backend?**
- Language mismatch: JavaScript tools add dependency outside Go ecosystem
- Maintenance burden: Different testing patterns across projects
- Performance: JavaScript interpreter overhead
- Integration: Better to test via Go standard library (`testing`, `httptest`)

**Go-Based Alternative (Recommended):**
1. **Table-Driven Unit Tests**: Domain logic in `*_test.go` files
2. **Integration Tests**: Full stack with `httptest.NewRecorder()`
3. **Go Libraries**:
   - `testify/assert` - Clean assertions
   - `testify/suite` - Test suites with setup/teardown
   - `github.com/getkin/kin-openapi` - OpenAPI spec validation
   - `gotest.tools/assert` - Alternative assertion library

**Proper Testing Workflow (AI-TDD in Go):**
1. Write failing domain tests first
2. Implement domain logic
3. Write repository tests with test database
4. Implement repository layer
5. Write integration tests with full HTTP stack
6. Validate responses against OpenAPI schema

**Backend Testing Commands:**
```bash
# Run all tests
make test

# Run unit tests only (domain + service)
make test-unit

# Run repository integration tests (repository + HTTP)
make test-repo-integration

# Run end-to-end tests
make test-e2e

# Generate coverage report
make test-coverage

# Validate OpenAPI spec
make validate-openapi
```

**Coverage Target:**
- Domain layer: ≥90%
- Service layer: ≥80%
- Repository layer: ≥80%
- Handler layer: ≥70% (integration tests)

**Backend Health Check Verification:**
- Backend server running at `http://localhost:8080`
- Health endpoint accessible: `GET /api/v1/ping` returns `{"message":"pong"}`

### Next Steps for API Testing

1. Create integration test suite in Go for core workflows
2. Document authentication flow and JWT token generation
3. Add Swagger UI generation from OpenAPI spec
4. Implement API versioning strategy for v2 endpoints
5. Add request/response logging and monitoring

### OpenAPI Spec Compliance Notes

**Current Status:**
- OpenAPI 3.0.3 format (downgraded from 3.1.0 for broader tool compatibility)
- Path parameters annotated with examples for test data generation
- Request/response schemas defined for all 32 endpoints
- Authentication via BearerAuth JWT scheme
- Standard error response schemas defined

**Known Issues:**
- `tags` field warnings (not errors) - informational only
- `nullable` deprecation warnings (valid in OpenAPI 3.0)
- `allOf` usage in `GrindWithTodayTask` schema (valid composition pattern)
- `additionalProperties` in metadata objects (valid for flexible property storage)

**Resolved:**
- Path parameter examples added (grind_123, task_456, msg_789, session_123)
- Query parameter naming corrected (setProblem instead of set-problem)
- Removed unsupported bearerFormat from security scheme
- Added comprehensive examples to all schema objects

---

## Decision Log: Integration Testing Strategy (DDD + OpenAPI)

### Context
- We need confidence that implemented APIs match business behavior and OpenAPI contracts.
- We currently have a Go backend with Gin + PostgreSQL and existing integration harness foundations under `backend/test`.
- We attempted Dredd for endpoint contract testing and observed compatibility friction with parser expectations and OpenAPI feature support.

### Decision
Use **Go-native integration testing** as the primary strategy, and treat OpenAPI as the contract source of truth validated from Go tests.

**Primary stack:**
1. `go test` + `testing` + `httptest` for endpoint integration tests.
2. Real PostgreSQL-backed repository/integration tests.
3. Optional OpenAPI response-schema checks using `kin-openapi` from Go tests.

**Not selected as primary:** Dredd/JS-based contract runner.

### Why This Decision
1. **Language consistency**: one toolchain (Go) for app + tests + CI.
2. **Lower maintenance**: avoids Node runtime/version drift and dual-ecosystem debugging.
3. **DDD alignment**: test boundaries by layer (domain, application, infrastructure, transport) while preserving dependency direction.
4. **Behavior-first coverage**: easier to assert HTTP behavior plus DB side effects in a single test flow.
5. **CI reliability**: deterministic setup with migrations/fixtures and no external parser blockers.

### Alternatives Considered
1. **Dredd as primary API test runner**
   - Pros: direct spec-driven execution.
   - Cons: parser compatibility issues in practice, extra JS toolchain complexity, lower leverage versus existing Go harness.
2. **Manual-only testing (Postman/curl)**
   - Pros: quick spot checks.
   - Cons: weak repeatability, no regression guard in CI.

### Testing Levels for This DDD Backend

1. **Domain Unit Tests (most tests, fastest)**
   - Scope: entities/value objects/invariants.
   - No DB/HTTP/framework imports.
   - Goal: business rule correctness.

2. **Application/Service Tests**
   - Scope: use-case orchestration and policy branching.
   - Repositories can be fakes/mocks where appropriate.
   - Goal: workflow correctness independent of transport.

3. **Repository Integration Tests (DB contract tests)**
   - Scope: repository methods against real PostgreSQL.
   - Run migrations + fixtures, assert persisted/query behavior.
   - Goal: catch SQL/schema/scan/mapping issues.

4. **HTTP API Integration Tests (black-box within backend boundary)**
   - Scope: request through Gin router, service layer, repository, DB.
   - Assert status code, response payload shape, and DB side effects.
   - Goal: endpoint behavior matches intended contract.

5. **OpenAPI Contract Assertions (from Go tests)**
   - Scope: validate selected responses against OpenAPI schemas.
   - Start with high-risk endpoints (`/login`, `/verify-token`, `/tasks/finish`, payment endpoints).
   - Goal: prevent contract drift and frontend breakage.

6. **Minimal E2E Smoke Tests (optional)**
   - Scope: app boot + a few critical user journeys.
   - Goal: deployment confidence; keep count low.

### Endpoint Coverage Policy

For each endpoint, include at least:
1. Happy path.
2. Validation failure.
3. Auth failure (if protected).
4. Not-found or forbidden branch.
5. Idempotency/concurrency path where relevant.

### Data Lifecycle Policy for Integration Tests
1. Suite setup: run migrations to latest.
2. Test setup: seed only minimal fixture set per case.
3. Isolation: cleanup by transaction rollback or deterministic truncation.
4. Avoid `t.Parallel()` for DB-coupled tests until isolation strategy is hardened.

### CI Gate Proposal
1. PR gate A: domain + application tests.
2. PR gate B: repository + API integration tests with test DB.
3. Optional gate C: OpenAPI schema assertion subset for critical endpoints.
4. Nightly: broader integration matrix (all core endpoints).

### Immediate Execution Plan
1. Make `backend/test/integration` compile cleanly and remove placeholder-only tests.
2. Implement first vertical slice tests:
   - register -> login -> create grind -> get today task -> finish task.
3. Add payment flow integration tests:
   - payment-intent, methods list, select-default.
4. Add OpenAPI schema assertions for auth/task/payment responses.
5. Expand to invitations/interviews flows in a second batch.

### Exit Criteria for This Testing Track
1. All core v1 endpoints have at least one passing integration test.
2. Critical endpoints have both happy and failure-path coverage.
3. CI blocks merge on integration-test failures.
4. OpenAPI schema checks run for selected critical responses.

### Future Scope Note: Multi-Habit Expansion (Beyond LeetCode)
1. Current entity and endpoint behavior is still primarily LeetCode-centric (`TaskType: "leetcode"`).
2. Future milestones must extend domain models, services, and API contracts to support non-LeetCode habits (for example language learning, wellness, productivity, and custom habit providers).
3. Test strategy must evolve with this migration by adding provider-agnostic task/event fixtures and cross-habit integration scenarios, so coverage does not assume coding-only workflows.
