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
