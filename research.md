# Terriyaki Project — Deep Dive Research Report

**Date:** March 25, 2026 (UPDATED)
**Previous Date:** March 5, 2025
**Scope:** Full codebase analysis — backend, frontend, Chrome extension, infrastructure
**Status:** Active development with significant API completions

---

## Executive Summary

**Terriyaki** is a **LeetCode grind companion app** that helps users commit to daily coding challenges with accountability partners. It combines:

- **Web app** (Next.js 16 + React 19 + MUI)
- **Go backend** (Gin, GORM, PostgreSQL + Redis + Stripe)
- **Chrome extension** for LeetCode integration (auto-detect solutions, badge countdown)
- **AI-powered mock interviews** (ElevenLabs voice agent + Gemini evaluation)

### Current Implementation Status (as of March 2026)

**✅ COMPLETED FEATURES:**
- Full CRUD endpoints for grinds, tasks, messages, interviews
- Dual auth endpoints: `/api/v1/login` (single grind) and `/api/v2/login` (multiple grinds)
- Payment pipeline: Stripe intent creation, card management, charging
- User profile management (update username/avatar)
- Interview flow: start → webhook → response → end with Gemini evaluation
- Chrome extension badge/notification system wired to backend
- CI/CD workflow (GitHub Actions)
- Redis integration for payment service
- Quit grind functionality (controller implemented but route NOT wired)

**⚠️ IN PROGRESS / INCOMPLETE:**
- No database transactions for multi-step operations (CreateGrind, AcceptInvitation)
- No idempotency keys for task submissions
- Health checks do NOT verify database connectivity
- Progress endpoint mentioned by frontend but NOT wired
- Async webhook processing still synchronous
- No rate limiting on LLM webhook or auth endpoints
- No explicit caching layer (Redis available but not utilized for hot data)

The system follows a DDD/Clean Architecture backend with domain entities, application services, infrastructure repositories, and HTTP controllers. The frontend uses Zustand for state and calls REST APIs. There is no real-time layer (WebSockets, SSE) or message queue.

---

## 1. Architecture Overview

### 1.1 Project Structure

```
terriyaki/
├── backend/                    # Go (Gin, GORM, PostgreSQL)
│   ├── internal/
│   │   ├── cmd/api_server/     # Entry point (main.go)
│   │   ├── domain/             # Entities + repository interfaces
│   │   ├── application/        # Services, DTOs, mappers
│   │   ├── infrastructure/     # Postgres implementations
│   │   ├── interface/api/      # Gin controllers
│   │   ├── config/             # CORS, error codes
│   │   ├── migrate/            # GORM AutoMigrate
│   │   └── utils/              # Auth, LeetCode helpers
│   ├── deploy/                 # Docker Compose
│   └── test/                   # API tests
│
├── frontend/                   # Next.js 16 + React 19
│   ├── src/
│   │   ├── app/                # App Router pages & components
│   │   ├── lib/                # Services, Zustand stores
│   │   ├── types/              # TypeScript types
│   │   └── config/             # API base, error codes
│   └── chrome-extension/       # LeetCode task tracker
```

### 1.2 Backend Layers (DDD / Clean Architecture)

| Layer | Path | Responsibility |
|-------|------|----------------|
| **Domain** | `internal/domain/` | Entities (User, Grind, Task, Participation, Message, InterviewSession) and repository interfaces |
| **Application** | `internal/application/` | Services, DTOs, mappers — business logic |
| **Infrastructure** | `internal/infrastructure/db/postgres/` | GORM repository implementations |
| **Interface** | `internal/interface/api/` | Gin HTTP controllers |

### 1.3 Data Flow

```
HTTP Request → Controller → Service → Repository → PostgreSQL
                    ↓
              DTO / Mapper
```

- **No transactions** across multi-step operations (e.g., CreateGrind + CreateParticipation + CreateTasks + CreateInvitationMessages).
- **No caching** (Redis, in-memory).
- **Single database** — no read replicas, no sharding.
- **Synchronous** — no background jobs, no queues.

---

## 2. Domain Model & Business Logic

### 2.1 Core Entities

| Entity | Purpose |
|--------|---------|
| **User** | id, username, email, avatar, hashed_password |
| **Grind** | id, duration (days), budget, start_date — a commitment period |
| **Task** | LeetCode task: user_id, grind_id, date, completed, problem_title, problem_url, code, code_language |
| **Participation** | user_id, grind_id, missed_days, total_penalty, quitted, quitted_at |
| **Message** | sender_id, receiver_id, content, type (invitation, invitation_accepted, invitation_rejected), invitation_grind_id |
| **InterviewSession** | user_id, task_id, status, conversation_history (JSONB), started_at, ended_at |

### 2.2 Key Business Rules

1. **Grind lifecycle**
   - Created with duration, budget, start date.
   - Creator gets participation + N tasks (one per day).
   - Invitations sent to participants by email.
   - Ongoing = `start_date + duration > now` and user has not quit.

2. **Task assignment**
   - Tasks from NeetCode 250 (CSV).
   - Problem assigned lazily on first `GetTask` with `set-problem=true`.
   - Today’s task: `date >= today 00:00 UTC` and `date < tomorrow 00:00 UTC`.

3. **Invitation flow**
   - Create message → Accept adds participation + tasks → Update message status → Create acceptance message.
   - Reject updates message and creates rejection message.

4. **Interview flow**
   - Start → Create session → ElevenLabs agent uses LLM webhook.
   - Webhook appends user transcript, checks 4-user-message limit → auto-end.
   - End → Gemini evaluation (score, feedback, strengths, improvements).

### 2.3 External Integrations

- **ElevenLabs**: Voice agent (TTS/STT), webhook for transcribed text.
- **Gemini**: Interview evaluation from conversation + task + code.
- **LeetCode API** (via `leetcode-api-go`): Problem metadata (used in utils, not in task assignment — NeetCode CSV is used instead).
- **Stripe**: Frontend only; backend payment routes not wired.

---

## 3. API Design

### 3.1 Base URL

`/api/v1`

### 3.2 Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| **Auth** | | |
| POST | `/register` | Register |
| POST | `/login` | Login |
| POST | `/logout` | Logout |
| GET | `/verify-token` | Verify JWT |
| **Grinds** | | |
| POST | `/grinds` | Create grind |
| GET | `/grinds` | List user grinds |
| GET | `/grinds/current` | Current ongoing grind |
| GET | `/grinds/:id` | Get grind by ID |
| DELETE | `/grinds/delete-all` | Delete all (dev) |
| **Tasks** | | |
| GET | `/tasks/today` | Today’s task |
| GET | `/tasks/:id` | Get task (optional `?set-problem=true`) |
| POST | `/tasks/finish` | Submit solution |
| **Messages** | | |
| GET | `/messages` | Get messages |
| POST | `/messages/:id/read` | Mark read |
| POST | `/messages/:id/invitation/create` | Create invitation |
| POST | `/messages/:id/invitation/accept` | Accept invitation |
| POST | `/messages/:id/invitation/reject` | Reject invitation |
| **Interviews** | | |
| POST | `/interviews/start` | Start interview |
| POST | `/interviews/llm` | ElevenLabs webhook |
| POST | `/interviews/:id/response` | Save agent response |
| POST | `/interviews/:id/end` | End interview |

### 3.3 Issues Fixed vs. Remaining (Updated March 2026)

**✅ FIXED (from original research):**
1. API v2 endpoints now exist: `/api/v2/login` and `/api/v2/verify-token`
2. Frontend auth should no longer 404 if using v2 endpoints
3. `taskToday` field **IS** included in `GroupGrindDTO` as `TodayTask`
4. Login handles no-grind gracefully in v2 (returns 200 with empty grinds array)
5. Payment backend fully implemented with Stripe integration
6. Chrome extension badge should now work (taskToday available)
7. Multiple controllers implemented: payment, profile, interview, message
8. CI/CD workflow in place (GitHub Actions with go test)

**⚠️ REMAINING CRITICAL ISSUES:**
1. **Missing routes:**
   - `POST /grinds/:id/quit` — controller QuitGrindAPI exists but NOT wired in main.go
   - `GET /grinds/:id/progress` — frontend calls but NOT implemented

2. **Database transactions:**
   - CreateGrind + AddParticipation + CreateTasks + InvitationMessages: NOT atomic
   - AcceptInvitation: Race condition on concurrent accepts (no transaction)
   - No SELECT...FOR UPDATE locks

3. **Idempotency:**
   - Task finish endpoint: Can be called multiple times; last write wins
   - No idempotency keys stored

4. **Health & Observability:**
   - `/ping` only checks if server is up, NOT database connectivity
   - No readiness probe for Kubernetes
   - No structured logging, metrics, or tracing

5. **Async Processing:**
   - ElevenLabs webhook: Still synchronous
   - Gemini evaluation: Blocks request until complete
   - Should be queued for async processing

6. **Rate Limiting & Security:**
   - No rate limiting on LLM webhook (DoS risk)
   - No rate limiting on auth endpoints
   - No request timeout configuration

7. **API Contract:**
   - v1 LoginAPI returns `grind` (single); v2 returns `grinds` (array)
   - Frontend must use v2 for multi-grind support to work correctly

---

## 4. Chrome Extension

### 4.1 Manifest V3

- **Permissions**: storage, activeTab, tabs, alarms, cookies
- **Hosts**: leetcode.com, localhost, terriyaki.com
- **Content script**: `content.js` on `https://leetcode.com/problems/*`

### 4.2 Behavior

1. **Content script**
   - Monitors for “Accepted” + runtime info.
   - Extracts code from Monaco editor.
   - Sends `solutionDetected` to background.

2. **Background**
   - Badge: countdown or ✓ when task completed.
   - Uses `/api/v1/grinds/current` for `taskToday`.
   - Alarms: 5 min (urgent), 15 min, 30 min, 60 min.

3. **Popup**
   - Shows today’s task, leaderboard, interview link.
   - Imports token from cookies (localhost, terriyaki.com).
   - Submits solution via `/api/v1/tasks/finish`.

### 4.3 Token Sync

- `chrome.storage.sync` for `apiUrl` and `authToken`.
- Cookie import from `localhost:3000`, `terriyaki.com`.

---

## 5. Implementation Details (March 2026 Status)

### 5.1 Backend Architecture Progress

**Core Layers Implemented:**
- ✅ Domain: Entities (User, Grind, Task, Participation, Message, InterviewSession, StripePaymentInfo)
- ✅ Application: Services with DTOs and mappers for all entities
- ✅ Infrastructure: GORM-based PostgreSQL repositories
- ✅ Interface: 8 controllers (User, Grind, Task, Message, Interview, Payment, Profile, Health)
- ✅ Config: CORS, error codes, auth utilities

**Service Layer Completeness:**
- UserService: Create, GetUser, GetUserByEmail, UpdateUser, ✅ Complete
- GrindService: Create, Get, GetAll, GetOngoing, Quit, DeleteAll, ✅ Complete
- TaskService: Get, GetToday, Finish, ✅ Complete
- MessageService: Create, Get, GetSent, Read, InvitationAccept, InvitationReject, ✅ Complete
- InterviewService: Start, GetSession, UpdateSession, SaveResponse, GetConversation, ✅ Complete
- StripePaymentService: Create payment intents, save cards, charge, payback, find dued payments, ✅ Complete
- ElevenLabsService: Integrated (see interview controller)

### 5.2 Dependency Stack (Go)

**ORM & Database:**
- GORM v1.31.1
- PostgreSQL driver (pgx v5.7.6)
- AutoMigrate for schema (no explicit migrations)

**External Services:**
- Stripe SDK v84.1.0 (payments)
- ElevenLabs (voice agent, webhook-based)
- Gemini LLM via google.golang.org/genai (evaluation)
- Redis v9.17.2 (in use by StripePaymentService)

**Framework & HTTP:**
- Gin v1.11.0 (web framework)
- CORS middleware (gin-contrib/cors v1.7.6)
- JWT v5.3.0 (authentication)

**Observability (NOT YET USED):**
- LeetCode API wrapper (`leetcode-api-go`)
- No structured logging (zap/zerolog)
- No metrics (prometheus)

### 5.3 Routing & Controllers

**Main routes registered in `cmd/api_server/main.go`:**
- v1: All core endpoints (grinds, tasks, messages, interviews, payments, profile, auth)
- v2: Auth only (login, verify-token) with multi-grind support
- Notable: `/grinds/:id/quit` and `/grinds/:id/progress` NOT registered

**Controllers instantiated with dependencies:**
```go
// 8 controllers fully implemented
grindCtrl, userCtrl, healthCtrl, taskCtrl, messageCtrl
interviewCtrl, paymentCtrl, profileCtrl
```

## 6. Database & Persistence

### 6.1 Schema (GORM AutoMigrate)

- **users**, **grinds**, **tasks**, **participation**, **message**, **interview_sessions**, **stripe_payment_infos**
- No explicit SQL migrations; schema changes via AutoMigrate
- Supports: PostgreSQL, MySQL, SQLite (GORM abstraction)

### 6.2 Connection

- Single `gorm.DB` connection
- No explicit connection pooling configuration
- No read replicas

### 6.3 Transaction Handling

- **NO explicit transactions** in services
- Multi-step flows (CreateGrind + participations + tasks + invitations) are NOT atomic
- Potential for data inconsistency on partial failures
- **Action:** Wrap CreateGrind, AcceptInvitation, QuitGrind in db.Transaction()

---

## 7. Authentication & Authorization

- **JWT**: HS256, 24h expiry, `sub` = user ID
- **Password**: bcrypt
- **Storage**: `js-cookie` (token) in frontend
- **Protected endpoints**: `Authorization: Bearer <token>`; `VerifyUserAccess` extracts user ID
- **Two auth versions**: v1 (single grind), v2 (multi-grind)

---

## 8. Current Stack Summary

| Component | Tech | Status |
|-----------|------|--------|
| Web Framework | Gin 1.11 | ✅ |
| ORM | GORM 1.31 | ✅ |
| Database | PostgreSQL | ✅ |
| Authentication | JWT + bcrypt | ✅ |
| Payments | Stripe v84 | ✅ |
| Interview AI | ElevenLabs + Gemini | ✅ (sync) |
| Cache | Redis 9.17 | ✅ (available) |
| Testing | Go testing | ⚠️ (basic) |
| CI/CD | GitHub Actions | ✅ |
| Logging | fmt.Println | ⚠️ (not structured) |
| Metrics | None | ❌ |
| Tracing | None | ❌ |
| Rate Limiting | None | ❌ |

---

## 9. Immediate Action Items (From Current Code Review)

### Critical Fixes (Must Do Before 100 Users)

| # | Item | File | Status | Impact |
|----|------|------|--------|--------|
| 1 | Wire `/grinds/:id/quit` route | `cmd/api_server/main.go` | ❌ Missing | Users cannot quit grinds; frontend calls fail |
| 2 | Add DB check to health endpoint | `interface/api/health_check_controller.go` | ❌ Missing | No readiness probe for K8s; cannot detect DB down |
| 3 | Wrap CreateGrind in transaction | `application/services/grind_service.go` | ❌ Missing | Partial grind creates leave orphaned data |
| 4 | Idempotency key for task finish | `interface/api/task_controller.go` | ❌ Missing | Double submissions count twice |
| 5 | Request timeout on Gemini calls | `interface/api/interview_controller.go` | ❌ Missing | Webhook timeouts on slow LLM |

### P1 Important (Next Sprint)

| # | Item | File | Status | Impact |
|----|------|------|--------|--------|
| 6 | Rate limiting on `/interviews/llm` | `main.go` middleware | ❌ Missing | LLM cost explosion on abuse |
| 7 | Structured logging | All controllers | ⚠️ Using fmt | Cannot debug production issues |
| 8 | Transaction for AcceptInvitation | `application/services/message_service.go` | ❌ Missing | Race on concurrent accepts |
| 9 | Wire `/grinds/:id/progress` route | `cmd/api_server/main.go` | ❌ Missing | Leaderboard/stats view broken |
| 10 | Async Gemini evaluation | `application/services/interview_service.go` | ⚠️ Sync | Interview end blocks on LLM latency |

---

## 10. Code Quality Observations

### What's Well Done ✅
- Clean DDD separation: domain has zero external deps
- Service layer properly abstracts business logic
- DTOs for all entities (type-safe API contracts)
- Proper error handling (mostly)
- Repository interface pattern for testability

### What Needs Improvement ⚠️
- **Error handling**: Many `fmt.Println` instead of structured logging
- **Testing**: Only basic integration tests; no unit tests for services
- **Controller validation**: Minimal input validation (e.g., duration > 0?)
- **Comments**: Some TODO markers, no field-level documentation
- **Naming**: Some inconsistency (e.g., `participationRepo` vs `participation_repo`)
- **Documentation**: No OpenAPI/Swagger specs

---

## 5. Database & Persistence

### 5.1 Schema (GORM AutoMigrate)

- **users**, **grinds**, **tasks**, **participation**, **message**, **interview_sessions**
- No explicit migrations; schema changes via AutoMigrate.

### 5.2 Connection

- Single `gorm.DB` connection.
- No connection pooling configuration visible.
- No read replicas.

### 5.3 Transaction Handling

- **No explicit transactions** in services.
- Multi-step flows (e.g., CreateGrind + participations + tasks + invitations) are not atomic.
- Partial failures can leave inconsistent state (e.g., grind created but invitations failed).

---

## 6. Authentication & Authorization

- **JWT**: HS256, 24h expiry, `sub` = user ID.
- **Password**: bcrypt.
- **Storage**: `js-cookie` (token) in frontend.
- **Protected endpoints**: `Authorization: Bearer <token>`; `VerifyUserAccess` extracts user ID.

---

## 7. Specificities & Edge Cases

### 7.1 Time Zones

- Task “today” uses UTC: `time.Now().UTC().Truncate(24 * time.Hour)`.
- Users in other time zones may see incorrect “today” boundaries.

### 7.2 Multiple Ongoing Grinds

- `FindLatestByUserID` returns the most recent grind.
- Task controller uses that single grind for “today’s task”.
- TODO in code: “might have multiple grinds in GetOngoingGrindByUserID”.

### 7.3 Create Grind Bug

- In `CreateGrindAPI`, on participant-not-found, `DeleteGrindDTO` uses `userDTO.ID` (creator) instead of `grindDTO.ID` (grind to delete).

### 7.4 Message Controller Response

- `GetMessageAPI` returns `"message": messageDTOs` (array) in each item instead of the single message for that row.

### 7.5 Finish Task

- `FinishTodayTaskAPI` does not validate that the task belongs to the authenticated user beyond “today’s task for current grind”.
- No idempotency: multiple submissions overwrite.

---

## 8. Short-, Mid-, and Long-Term Goals

### 8.1 Short-Term (1–3 months)

| Goal | Rationale |
|------|------------|
| Fix API mismatches | Align frontend auth (v1/v2, grinds vs grind), add `taskToday` to `/grinds/current` |
| Add missing routes | `quit`, `progress` for grinds |
| Fix CreateGrind rollback | Use `grindDTO.ID` in DeleteGrind, wrap in transaction |
| Handle no-grind in Login/Verify | Return 200 with `grind: null` instead of 500 |
| Add database transactions | Wrap multi-entity operations (create grind, accept invitation) in `db.Transaction()` |
| Wire payment backend | Stripe webhooks, payment method storage |

### 8.2 Mid-Term (3–6 months)

| Goal | Rationale |
|------|------------|
| Time zone support | User preference, correct “today” per user |
| Multiple grinds | Support multiple ongoing grinds, grind selector in UI |
| Idempotent task finish | Prevent duplicate submissions, idempotency keys |
| Background jobs | Invitation emails, reminder notifications |
| Observability | Structured logging, metrics, tracing |
| Rate limiting | Protect LLM webhook, auth endpoints |

### 8.3 Long-Term (6–12+ months)

| Goal | Rationale |
|------|------------|
| Real-time updates | WebSockets/SSE for grind progress, live leaderboard |
| Mobile app | React Native or PWA |
| Advanced analytics | Streaks, completion trends, recommendations |
| Multi-tenancy / teams | Organizations, team grinds |
| Scale-out | Read replicas, caching, horizontal scaling |

---

## 9. Distributed Systems: Consistency, Availability, Partition Tolerance

### 9.1 Current State

- **Single region**: One backend, one DB.
- **No replication**: No read replicas, no multi-AZ.
- **Synchronous**: No queues, no eventual consistency.
- **Stateless API**: No server-side session; JWT is stateless.

### 9.2 Consistency (Concurrency)

**Current risks**

1. **Race on invitation accept**: Two users accepting the same invitation; `AddParticipation` checks existence but not under a lock.
2. **Double task finish**: Same task submitted twice; last write wins.
3. **Non-atomic multi-step flows**: Create grind + participations + tasks + invitations can partially fail.

**Mitigations**

1. **Database transactions**
   - Wrap CreateGrind, AddParticipation, AcceptInvitation in `db.Transaction()`.
   - Use `SELECT ... FOR UPDATE` for participation checks.

2. **Optimistic locking**
   - Add `version` or `updated_at` to Task; reject finish if version changed.

3. **Idempotency**
   - `X-Idempotency-Key` on task finish; store key → result; return cached result on replay.

4. **Unique constraints**
   - `UNIQUE (user_id, grind_id)` on participation to prevent duplicates at DB level.

### 9.3 Availability

**Current risks**

1. **Single point of failure**: One DB, one backend process.
2. **No health checks**: `/ping` exists but no DB connectivity check.
3. **LLM dependency**: ElevenLabs/Gemini outages affect interviews.
4. **No retries**: External calls (ElevenLabs, Gemini) have no retry/backoff.

**Mitigations**

1. **Redundancy**
   - Multiple backend replicas behind a load balancer.
   - PostgreSQL with streaming replication, failover.

2. **Health checks**
   - Liveness: process alive.
   - Readiness: DB connected, critical deps reachable.

3. **Circuit breakers**
   - Wrap ElevenLabs/Gemini calls; fail fast when downstream is down.

4. **Graceful degradation**
   - Interviews: queue evaluation, return “evaluation pending” instead of blocking.

5. **Connection pooling**
   - Configure GORM/DB pool (max open, max idle, max lifetime).

### 9.4 Partition Tolerance (Network Partitions)

**Current risks**

1. **Client–server partition**: User loses connection mid-request; no retry strategy.
2. **Backend–DB partition**: DB unreachable; all requests fail.
3. **Backend–ElevenLabs partition**: Webhook timeouts, no retries.

**Mitigations**

1. **Timeouts**
   - HTTP client timeouts for outbound calls.
   - Request timeouts in Gin middleware.

2. **Retries with backoff**
   - Idempotent operations (e.g., webhook processing) retried with exponential backoff.

3. **Async processing**
   - Webhook: accept request, return 200, process in background; store in queue (e.g., Redis, SQS).

4. **Eventual consistency**
   - For non-critical paths (e.g., badge update), accept temporary inconsistency; sync when partition heals.

5. **CAP trade-off**
   - In a partition, prefer **availability** for reads (cached/stale data) and **consistency** for writes (reject or queue).
   - For Terriyaki: consistency is more important for grinds/tasks; availability for reads (e.g., badge) can be relaxed.

---

## 10. Recommendations Summary

| Area | Priority | Action |
|------|----------|--------|
| API contract | High | Fix v1/v2, grinds vs grind, taskToday in current grind |
| Transactions | High | Wrap create/accept flows in DB transactions |
| Missing routes | High | Add quit, progress endpoints |
| Login/Verify | High | Return 200 with null grind when no ongoing grind |
| Idempotency | Medium | Add idempotency keys for task finish |
| Health checks | Medium | Add DB and dependency checks |
| Retries | Medium | Retry external API calls with backoff |
| Caching | Low | Cache current grind for badge (short TTL) |
| Observability | Medium | Structured logs, metrics, tracing |

---

## 12. Follow-Up Plan: Scalability, Testability, Consistency, Robustness

A phased plan to evolve Terriyaki into a production-grade, AI-enabled system aligned with modern distributed systems and ML operations practices.

### 12.1 Phase 1: Foundation (Weeks 1–4)

| Pillar | Actions |
|--------|---------|
| **Consistency** | Wrap CreateGrind, AddParticipation, AcceptInvitation in `db.Transaction()`; add `UNIQUE(user_id, grind_id)` on participation; fix CreateGrind rollback bug |
| **API contract** | Align frontend auth (v1), add `taskToday` + `participants` to `/grinds/current`; implement missing `quit`, `progress` routes |
| **Testability** | Add unit tests for services (mocked repos); integration tests for critical flows (create grind, accept invitation); API contract tests (e.g., Pact or OpenAPI validation) |
| **CI/CD** | Extend `.github/workflows/ci.yml` with: `go test -race -cover`, lint (golangci-lint), build Docker image, push to registry; add frontend build + test stage |

### 12.2 Phase 2: Observability & Resilience (Weeks 5–8)

| Pillar | Actions |
|--------|---------|
| **Observability** | Structured logging (zerolog/zap); Prometheus metrics (request latency, error rate, DB pool); Grafana dashboards for API and interview flows |
| **Health checks** | Liveness (`/health/live`), readiness (`/health/ready` with DB + ElevenLabs connectivity); Kubernetes probes |
| **Resilience** | Retries with exponential backoff for ElevenLabs/Gemini; circuit breakers; request timeouts in Gin middleware |
| **Async webhooks** | Accept ElevenLabs webhook → enqueue to Redis/Kafka → process in worker; return 200 immediately to avoid timeouts |

### 12.3 Phase 3: Scalability (Weeks 9–14)

| Pillar | Actions |
|--------|---------|
| **Horizontal scaling** | Stateless API; session affinity not required (JWT); run multiple replicas behind load balancer |
| **Database** | Connection pooling (GORM); read replicas for `/grinds`, `/messages`; write path stays on primary |
| **Caching** | Redis for `getCurrentGrind` (short TTL, invalidate on grind/task updates); reduce DB load for badge and popup |
| **Data pipeline** | Kafka topics for events (grind_created, task_completed, interview_ended); ClickHouse/Elasticsearch for analytics (completion trends, streaks, leaderboards) |

### 12.4 Phase 4: AI/ML & Agentic Workflows (Weeks 15–20)

| Pillar | Actions |
|--------|---------|
| **Interview pipeline** | Decouple LLM evaluation: webhook → event → async worker → Gemini → store result; support batch evaluation for cost optimization |
| **RAG (future)** | Embed problem descriptions + solutions; semantic search for “similar problems” or “practice recommendations” |
| **MCP integration** | Expose Terriyaki as MCP server (grinds, tasks, progress) for agentic workflows (e.g., “Create a grind for next week” via natural language) |
| **Model observability** | Log Gemini/ElevenLabs latency, token usage, failure rate; alert on degradation |

### 12.5 Phase 5: Production Hardening (Ongoing)

| Pillar | Actions |
|--------|---------|
| **Containerization** | Multi-stage Dockerfile (already present); Kubernetes manifests (Deployment, Service, Ingress, HPA) |
| **Secrets** | External secret management (Vault, cloud provider); no secrets in env files |
| **Rate limiting** | Per-user limits on LLM webhook, task finish; protect against abuse |
| **Data retention** | Policy for old interview sessions, completed grinds; archive to cold storage |

---

## 13. Future Bottlenecks If Issues Are Not Addressed

### 13.1 Immediate Bottlenecks (0–6 months)

| Issue | Consequence | Trigger |
|------|-------------|---------|
| **No transactions** | Partial state on create/accept; manual cleanup; user confusion | Concurrent invites, network blips during create |
| **API mismatches** | Auth fails (v2 404); Chrome extension broken (no taskToday); frontend errors | Every login, extension use |
| **Login returns 500 when no grind** | Users without active grind cannot log in | New users, users between grinds |
| **Single DB, no pooling** | Connection exhaustion under moderate load | 50+ concurrent users |
| **Synchronous LLM webhook** | ElevenLabs timeout; interview drops mid-session | Slow Gemini, network latency |

### 13.2 Scaling Bottlenecks (6–18 months)

| Issue | Consequence | Trigger |
|------|-------------|---------|
| **No read replicas** | Primary overloaded by reads (grinds, tasks, messages) | 500+ DAU |
| **No caching** | Repeated `getCurrentGrind` hits DB; badge/popup latency | High extension usage |
| **No async processing** | Webhook blocks; interview evaluation blocks request | Peak interview hours |
| **No observability** | Incidents discovered by users; slow root-cause analysis | Any outage |
| **No rate limiting** | LLM cost explosion; abuse (spam invites, fake tasks) | Malicious or buggy clients |

### 13.3 Long-Term Bottlenecks (18+ months)

| Issue | Consequence | Trigger |
|------|-------------|---------|
| **Monolithic API** | Cannot scale interview vs. core API independently; shared failure domain | 10K+ users |
| **No event-driven architecture** | Tight coupling; hard to add features (notifications, analytics, ML) without touching core | New product features |
| **Single-region deployment** | High latency for global users; no disaster recovery | International expansion |
| **No model versioning** | Gemini prompt changes break evaluations; no A/B testing | Model upgrades, prompt tuning |
| **Technical debt** | Slower feature velocity; more bugs; harder onboarding | Sustained growth |

### 13.4 Risk Matrix

| Severity | Likelihood | Mitigation Priority |
|----------|------------|----------------------|
| Auth/API broken (users can’t log in) | High | **P0** — Fix in Phase 1 |
| Data inconsistency (partial grinds) | Medium | **P0** — Transactions in Phase 1 |
| LLM webhook timeout | Medium | **P1** — Async in Phase 2 |
| DB connection exhaustion | Low (initially) | **P1** — Pooling + replicas in Phase 3 |
| No visibility into failures | High | **P1** — Observability in Phase 2 |
| Cost explosion (LLM abuse) | Low | **P2** — Rate limiting in Phase 5 |

---

## 14. Conclusion

Terriyaki is a **well-structured LeetCode accountability app with solid DDD architecture** and **mostly-complete API endpoints** (as of March 2026). The implementation has progressed significantly from the original research date.

### What's ✅ IMPLEMENTED
- Full layered architecture: domain → application → infrastructure → interface
- All major controllers: grind, user, task, message, interview, payment, profile
- Dual auth: `/api/v1/login` (single grind) & `/api/v2/login` (multiple grinds)
- Stripe payment pipeline: intents, card save, charging
- Interview flow: start → ElevenLabs webhook → Gemini eval
- Chrome extension support: taskToday field in responses
- CI/CD: GitHub Actions with go test

### What's ⚠️ PARTIALLY DONE / NEEDS FIXES
- Database transactions: No atomic multi-step operations (CreateGrind, AcceptInvitation)
- Route wiring: `/grinds/:id/quit` controller exists but NOT registered in main.go
- Health checks: `/ping` only checks server alive, NOT database connectivity
- Idempotency: Task finish can be called multiple times
- Async processing: ElevenLabs webhook and Gemini evaluation are still synchronous

### What's ❌ MISSING
- Rate limiting (LLM webhook DoS risk)
- Structured logging & observability (only fmt.Println)
- Metrics & tracing
- `/grinds/:id/progress` endpoint
- Request timeouts on external API calls

### Priority Next Steps (P0-P1)
1. **Wire `/grinds/:id/quit` route** (15 min) — QuitGrindAPI exists but missing from main.go
2. **Wrap CreateGrind in transaction** (1 hr) — Prevent inconsistent state on failures
3. **Add DB health check** (30 min) — Readiness probe requirement
4. **Implement idempotency keys** (1 hr) — Prevent double-counted tasks
5. **Add rate limiting** (1.5 hr) — Protect LLM webhook & auth endpoints

### Verdict
With these fixes, **Terriyaki will be production-ready for 100–500 DAU**. Beyond that, focus shifts to observability, caching, and horizontal scaling.

---

## 15. Solana + Anchor + Rust Deep Dive for Habitat Payment Integration (April 2026)

This section is a practical, implementation-oriented deep dive for building Solana payment support in this repository with Rust and Anchor, while preserving the provider-neutral Go payment architecture.

### 15.1 Why Anchor for This Project

Anchor is the right fit for this codebase because:

1. It enforces safer account validation patterns than manual Solana SDK boilerplate.
2. It generates an IDL, which gives the Go backend a stable ABI-like contract to integrate against.
3. It makes signer/account constraints explicit and auditable.
4. It accelerates local testing and reduces footguns around account serialization and discriminators.

### 15.2 Solana Runtime Fundamentals You Must Internalize

#### 15.2.1 Account model (not contract-storage model)
- Solana programs are stateless executables.
- State lives in accounts passed into each instruction.
- Every instruction must declare all accounts it will read/write.

Implication for Habitat:
- Pledge and settlement data must be stored in explicit state accounts.
- Escrow funds should be held in a deterministic vault account (PDA-owned), not hidden in program globals.

#### 15.2.2 Rent and account sizing
- Accounts need lamports to stay rent-exempt.
- Oversizing wastes capital, undersizing breaks upgrades.

Implication for Habitat:
- Define exact byte size constants for each account type.
- Version account schemas intentionally to avoid accidental layout breakage.

#### 15.2.3 Program Derived Addresses (PDAs)
- PDAs are deterministic addresses derived from seeds + program id.
- PDAs have no private key; only the program can sign for them using seeds and bump.

Implication for Habitat:
- Use deterministic seeds for pledge and vault so retries do not create duplicate escrow state.
- Keep seed schema stable and mirrored in Go derivation helpers.

Suggested seed strategy:
- pledge PDA: ["pledge", user_pubkey, pledge_uuid]
- vault PDA: ["vault", pledge_pda]

#### 15.2.4 Signer model and authority
- Solana does not infer authority from account ownership alone; signer checks are explicit.

Implication for Habitat:
- `resolve_success` and `resolve_failure` must require oracle signer.
- `claim_timeout` must require user signer.
- State transitions must verify both role and current status.

#### 15.2.5 Transaction lifecycle and finality
- broadcast is not confirmation.
- confirmation level matters (`processed`, `confirmed`, `finalized`).

Implication for Habitat:
- Go adapter must persist intermediate state separately from final settlement state.
- Mark `settled_onchain` only after configured confirmation policy (for example, `finalized` or K-slot threshold).

### 15.3 Anchor Internals and Practical Usage

#### 15.3.1 Program entrypoint
- Anchor uses `#[program]` module handlers as instruction entrypoints.
- Each handler takes `Context<T>` + typed args.

#### 15.3.2 Account validation by type system
- `#[derive(Accounts)]` contexts express:
   - mutability
   - signer requirements
   - PDA seeds/bump constraints
   - ownership/program constraints

This shifts many runtime errors into deterministic validation failures.

#### 15.3.3 State accounts
- `#[account]` structs define serialized on-chain state.
- Anchor prepends an 8-byte discriminator.
- Allocate `8 + LEN` where LEN is exact payload length.

#### 15.3.4 Error model
- `#[error_code]` maps enum variants to numeric codes.
- Stable code mapping is required so Go can classify retriable vs terminal errors.

#### 15.3.5 Events and observability
- `#[event]` logs typed payloads.
- Go backend can parse logs for reconciliation/audit enrichment.

#### 15.3.6 IDL and cross-language bridge
- IDL includes instruction names, args, account order, types, and events.
- The Go adapter should treat IDL as the source of truth for ABI compatibility checks.

### 15.4 Rust Knowledge Required for This Implementation

#### 15.4.1 Ownership and borrowing in on-chain code
- Avoid unnecessary clones of account data.
- Use scoped mutable borrows; do not keep long mutable references across logic branches.

#### 15.4.2 Error handling
- Use typed errors and `Result<()>` paths.
- Avoid panic paths; every failure should map to deterministic program errors.

#### 15.4.3 Data modeling
- Use enums for pledge status transitions.
- Keep explicit conversion boundaries between raw lamports and business amounts.

#### 15.4.4 Serialization
- Anchor defaults to Borsh for account/instruction serialization.
- Never reorder existing account fields in production without migration strategy.

### 15.5 Habitat-Specific On-Chain Domain Model

#### 15.5.1 Pledge state account
Minimum fields:
- `user_pubkey`
- `oracle_pubkey`
- `escrow_amount`
- `deadline_timestamp`
- `status` (Pending, ResolvedSuccess, ResolvedFailure, TimeoutClaimed)
- `bump`
- `created_at`
- `resolved_at` (optional)

#### 15.5.2 Resolution receipt account
Purpose:
- immutable settlement evidence for audit/reconciliation.

Suggested fields:
- `pledge_pubkey`
- `resolver_pubkey`
- `resolution_type`
- `resolution_timestamp`
- `reference_hash` (optional correlation id)

#### 15.5.3 Event payloads
- `PledgeInitialized`
- `PledgeResolved`
- `TimeoutClaimed`

Each event should include enough correlation data for backend logs:
- pledge id
- user pubkey
- resolver pubkey
- amount
- status

### 15.6 Instruction-Level Requirements and Invariants

#### 15.6.1 initialize_pledge
Preconditions:
- amount > 0
- deadline in future
- pledge account not already initialized

Postconditions:
- state initialized to Pending
- funds moved into vault
- event emitted

#### 15.6.2 resolve_success
Preconditions:
- oracle signer must match state
- status must be Pending

Postconditions:
- funds moved to user
- status set ResolvedSuccess
- receipt created

#### 15.6.3 resolve_failure
Preconditions:
- oracle signer must match state
- status must be Pending

Postconditions:
- funds moved to penalty account
- status set ResolvedFailure
- receipt created

#### 15.6.4 claim_timeout
Preconditions:
- user signer must match state owner
- now > deadline + grace window
- status must still be Pending

Postconditions:
- funds returned to user
- status set TimeoutClaimed
- receipt created

### 15.7 Security Checklist (Non-Negotiable)

1. Verify signer authority in every state-changing instruction.
2. Enforce one-way state transitions; disallow double resolution.
3. Validate all PDA seeds and bumps in account constraints.
4. Ensure vault ownership and transfer authority are program-controlled.
5. Validate all amount math with checked arithmetic.
6. Prevent replay by deriving deterministic pledge accounts and rejecting duplicate initialization.
7. Enforce strict account ownership/program checks for all external accounts.
8. Emit events for every terminal state transition.
9. Add explicit timeout/grace logic to prevent oracle liveness failures from trapping user funds.
10. Version IDL and account schemas before production deploys.

### 15.8 Testing Strategy Across Layers

#### 15.8.1 Rust/Anchor program tests
Required test classes:
- happy path for all 4 instructions
- unauthorized signer tests
- state transition violation tests
- PDA determinism tests
- timeout boundary tests
- rent/insufficient funds tests

#### 15.8.2 Go adapter unit tests
- mock RPC client behavior
- sign/broadcast/confirm stage failures
- retry idempotency behavior
- error-code mapping from Anchor errors -> canonical service errors

#### 15.8.3 Go integration tests
- local validator + deployed Anchor program
- adapter contract suite reused from payment provider-neutral tests
- duplicate retry safety: ensure one settlement side effect only

#### 15.8.4 End-to-end settlement tests
- create intent -> initialize pledge -> resolve -> confirm -> persisted settlement proof
- broadcast failure and recover path
- oracle unavailable then timeout claim path

### 15.9 Go Backend Integration Design

#### 15.9.1 Adapter workflow stages
1. Build instruction accounts and args.
2. Sign transaction.
3. Broadcast transaction.
4. Confirm to required finality.
5. Persist canonical settlement result.

Persist stage-level metadata to support operations:
- submission signature
- latest slot seen
- confirmation status
- last RPC error category

#### 15.9.2 Canonical status mapping
- Solana pending tx -> `pending`
- signature confirmed but not final -> `captured` or transitional pending (policy decision)
- finalized success -> `settled_onchain`
- finalized failure/revert path -> `failed`
- timeout reclaim success -> `refunded` (or dedicated status if desired)

#### 15.9.3 RPC and resilience policy
- use multiple RPC endpoints with failover.
- classify errors into retryable vs terminal.
- enforce capped exponential backoff and dead-letter handling for manual review.

### 15.10 Devnet/Mainnet Operational Concerns

1. Key management
- oracle private key must be in secure KMS/HSM-backed storage.
- never commit keypairs in repo.

2. Program upgrade authority
- use a controlled multisig process.
- record upgrade events and changelog with IDL diff.

3. Finality policy
- define chain confirmation threshold in runbook.
- do not mark settlement final before threshold.

4. Observability
- logs: instruction, signature, slot, latency, resolver.
- metrics: submission success rate, confirmation latency p95/p99, failure categories.

5. Incident response
- playbook for stuck pending signatures.
- replay-safe reprocessing from persistent settlement intents.

### 15.11 Suggested Repository Task Breakdown

#### solana/ tasks
1. Add Anchor workspace files and program module macros.
2. Implement account structs and errors/events.
3. Implement 4 instructions with invariants.
4. Add test suite covering signer, PDA, timeout, replay.
5. Generate and version IDL.

#### backend/ tasks
1. Add Solana RPC client abstraction and typed error categories.
2. Implement adapter methods against provider-neutral interface.
3. Add deterministic PDA helper matching Anchor seeds exactly.
4. Add integration harness for local validator tests.
5. Extend reconciliation and retry workers for on-chain settlement confirmation.

#### docs/ tasks
1. Add program ABI compatibility contract doc.
2. Add local environment bootstrap guide.
3. Add operator runbook for settlement incidents.

### 15.12 Learning Path: Build While Learning Rust

Week-by-week progression:

Week 1:
- Rust syntax essentials, ownership, enums, Result.
- Read and run minimal Anchor example program locally.

Week 2:
- Implement `initialize_pledge` with tests.
- Understand PDAs, account constraints, rent.

Week 3:
- Implement `resolve_success` and `resolve_failure`.
- Add signer authorization and transition safety tests.

Week 4:
- Implement `claim_timeout` and boundary tests.
- Generate IDL and begin Go binding/mapping layer.

Week 5:
- Implement Go Solana adapter.
- Run adapter contract suite and failure-retry simulations.

Week 6:
- Add observability and runbooks.
- Validate deploy checklist on devnet.

### 15.13 Common Pitfalls to Avoid

1. Treating transaction submission as final settlement before confirmation.
2. Allowing non-deterministic seed formats between Rust and Go.
3. Missing unauthorized signer negative tests.
4. Failing to persist enough metadata for reconciliation.
5. Upgrading program interfaces without IDL version coordination.
6. Not modeling timeout recovery, leaving funds locked if oracle fails.

### 15.14 Exit Criteria for "Solana Payment Method Supported"

The Solana payment method should be considered supported only when all are true:

1. Anchor program implements all 4 lifecycle instructions with tests passing.
2. IDL is generated, versioned, and consumed by Go integration layer.
3. Go adapter fully implements provider-neutral payment interface methods used in production flow.
4. Adapter contract tests pass for both Stripe and Solana.
5. End-to-end retry/idempotency tests prove no duplicate settlement side effects.
6. Devnet deployment and operator runbook are validated.
7. Observability dashboards expose settlement submission and confirmation health.
