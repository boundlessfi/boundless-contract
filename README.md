# boundless-contract

Soroban smart contracts that anchor the Boundless platform on Stellar.

## What is in this repo

Two contracts in one workspace.

| Contract | Path | Purpose |
|----------|------|---------|
| `boundless-events` | `contracts/events` | Event records (hackathon, bounty, grant) and inlined escrow. Multi-token whitelist. Idempotency. Per-pillar dispatch on a single canonical `create_event`. |
| `boundless-profile` | `contracts/profile` | Per-user credits and reputation. Lazy bootstrap. Per-token earnings tracking. Mutated almost exclusively by the events contract. |

Specs live in the platform PRD set under the parent `boundless-repos/` directory:

- `boundless-platform-contract-prd.md` (events contract)
- `boundless-credits-reputation-prd.md` (profile contract)
- `boundless-chain-abstraction-adr.md` (off-chain readiness for a future second chain)
- `boundless-organizer-end-to-end-prd.md` (umbrella; everything organizer-side)

## Architecture in one paragraph

The events contract is the on-chain anchor of event existence, key terms, escrow custody, submission anchors, and winner records. The profile contract is the on-chain anchor of credit balances and reputation scores. The events contract calls into the profile contract for credit charging on apply, credit earning on accept, and reputation bumping on win. The off-chain orchestrator (`boundless-nestjs`) handles everything else: drafts, rich content via `content_uri`, KYC, role policies, AI features, moderation. The contracts hold the things the platform cannot afford to be trusted on; nestjs holds the things that benefit from iteration speed.

## Prerequisites

- Rust 1.90.0 (`rust-toolchain.toml` pins this).
- Soroban SDK 23.5.2.
- `stellar` CLI for building WASM (`brew install stellar/tap/stellar-cli`).
- `wasm32v1-none` target: `rustup target add wasm32v1-none`.

## Build and test

```sh
# Unit tests, host target
cargo test --all

# WASM build, both contracts
cd contracts/events && make build && cd ../..
cd contracts/profile && make build && cd ../..

# Check WASM sizes against the 64 KB Soroban ceiling
cd contracts/events && make size && cd ../..
cd contracts/profile && make size && cd ../..
```

## Repo layout

```
boundless-contract/
├── Cargo.toml                    # workspace
├── rust-toolchain.toml
├── .cargo/
├── .github/workflows/            # verify, deploy, rustfmt
├── contracts/
│   ├── events/
│   │   ├── Cargo.toml
│   │   ├── Makefile
│   │   └── src/
│   │       ├── lib.rs            # contract entry, public surface
│   │       ├── types.rs          # EventRecord, Submission, Winner, enums
│   │       ├── errors.rs         # error code enum
│   │       ├── events.rs         # contract event emissions
│   │       ├── storage.rs        # persistent / temporary key helpers
│   │       ├── admin.rs          # init, rotation, pause, upgrade
│   │       ├── token_whitelist.rs # admin-managed supported-tokens set
│   │       ├── escrow.rs         # fee math, token transfer helpers
│   │       ├── event_ops.rs      # canonical create / submit / select_winners
│   │       ├── hackathon.rs      # pillar-specific validation
│   │       ├── bounty.rs         # pillar-specific validation + apply / withdraw
│   │       ├── grant.rs          # pillar-specific validation + claim_milestone
│   │       ├── idempotency.rs    # OpSeen helpers + deployment-epoch ID base
│   │       └── tests/            # per-area integration tests
│   └── profile/
│       ├── Cargo.toml
│       ├── Makefile
│       └── src/
│           ├── lib.rs            # contract entry, public surface
│           ├── types.rs          # Profile, PendingAdmin
│           ├── errors.rs
│           ├── events.rs
│           ├── storage.rs
│           ├── admin.rs          # init, two-step rotations, pause, upgrade
│           ├── credits.rs        # bootstrap, spend, earn, refund, admin_grant
│           ├── reputation.rs     # bump, slash, admin_slash
│           ├── earnings.rs       # per-token earnings registration
│           ├── idempotency.rs    # OpSeen helpers
│           └── tests/
└── docs/
    └── ARCHITECTURE.md           # the why
```

## Deployment

Two-step sequence (see `boundless-credits-reputation-prd.md` Section 13.1):

1. Deploy `boundless-profile` with `default_bootstrap_credits = 10`.
2. Deploy `boundless-events` with the profile contract's address + admin + fee_account + fee_bps.
3. Call `profile.set_events_contract(events_addr)`, then `accept_events_contract` as the events contract address.
4. Register supported tokens on `boundless-events` (USDC at launch).

Deployment scripts live in `scripts/`. Mainnet deploys are multisig-gated and only after external audit clears.

## Audit

External audit is the hard gate before mainnet. Audit checklist is in the platform contract PRD Section 15.3.

## Contributing

See `CONTRIBUTING.md`.
