# boundless-contract

Soroban smart contracts that anchor the Boundless platform on Stellar.

## What is in this repo

Two contracts in one workspace.

| Contract | Path | Purpose |
|----------|------|---------|
| `boundless-events` | `contracts/events` | Event records for the four pillars (hackathon, bounty, grant, crowdfunding) with inlined escrow. Multi-token whitelist. Idempotent operations. Per-pillar dispatch on a single canonical `create_event`. Paged cancellation and timelocked upgrades. |
| `boundless-profile` | `contracts/profile` | Per-user reputation and per-token earnings. Lazy profile bootstrap. Mutated almost exclusively by the events contract. |

## Deployments

| Network | Contract | Address | Version |
|---------|----------|---------|---------|
| Mainnet | `boundless-events` | `CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ` | `1.1.0` |
| Mainnet | `boundless-profile` | `CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC` | `1.1.0` |
| Testnet | `boundless-events` | `CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP` | `1.0.0` |
| Testnet | `boundless-profile` | `CCA3OAIBOZBUPHPRI5GI6N5PDTE7RTNLKAEID4JTC2YZIHIZNDX5Q6T3` | `1.0.0` |

Both contracts expose an on-chain `version()` view; the versions above are live reads.

## Architecture in one paragraph

The events contract is the on-chain anchor of event existence, key terms, escrow custody, submission anchors, and winner records. Funds enter escrow at creation (or through top-ups), fees are taken at deposit, and payouts release through `select_winners` or per-milestone claims. The profile contract is the on-chain anchor of reputation scores and per-token earnings. The events contract calls into the profile contract to bootstrap profiles on first touch, bump reputation on wins and milestones, and register earnings on payout. Every state-mutating operation carries an idempotency key, admin surfaces are pausable, and upgrades are timelocked.

## Prerequisites

- Rust 1.90.0 (`rust-toolchain.toml` pins this).
- Soroban SDK 23.5.x (workspace-pinned in `Cargo.toml`).
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

Test snapshots under `contracts/*/test_snapshots/` are the audit trail. When fixtures change shape, regenerate snapshots intentionally and commit them in the same PR.

## Repo layout

```
boundless-contract/
├── Cargo.toml                    # workspace
├── rust-toolchain.toml
├── deploy_and_upgrade.sh         # testnet deploy / upgrade helper
├── deploy_mainnet.sh             # mainnet cold deploy
├── .github/workflows/            # verify-build, rustfmt, deploy
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
│   │       ├── admin.rs          # init, rotation, pause, timelocked upgrade
│   │       ├── token_whitelist.rs # admin-managed supported-tokens set
│   │       ├── escrow.rs         # fee math, token transfer helpers
│   │       ├── event_ops.rs      # canonical create / submit / select_winners
│   │       ├── hackathon.rs      # pillar-specific validation
│   │       ├── bounty.rs         # pillar-specific validation + apply / withdraw
│   │       ├── grant.rs          # pillar-specific validation + claim_milestone
│   │       ├── crowdfunding.rs   # pillar-specific validation
│   │       ├── profile_client.rs # cross-contract interface to boundless-profile
│   │       ├── idempotency.rs    # OpSeen helpers + deployment-epoch ID base
│   │       └── tests/            # per-area integration tests
│   └── profile/
│       ├── Cargo.toml
│       ├── Makefile
│       └── src/
│           ├── lib.rs            # contract entry, public surface
│           ├── types.rs          # Profile, PendingAdmin, PendingUpgrade
│           ├── errors.rs
│           ├── events.rs
│           ├── storage.rs
│           ├── admin.rs          # init, two-step rotations, pause, timelocked upgrade
│           ├── bootstrap.rs      # lazy profile creation (bootstrap / bootstrap_self)
│           ├── reputation.rs     # bump, slash, admin_slash
│           ├── earnings.rs       # per-token earnings registration
│           ├── idempotency.rs    # OpSeen helpers
│           └── tests/
└── scripts/
    ├── admin/                    # verify-multisig.sh
    └── deploy/                   # deploy.sh, register_token.sh
```

## Deployment and upgrades

Fresh deploy sequence for a new network:

1. Deploy `boundless-profile` with the admin address.
2. Deploy `boundless-events` with the profile contract's address, admin, fee account, and fee bps.
3. Call `profile.set_events_contract(events_addr)` (first-set-only; later rotation is a timelocked two-step).
4. Register supported tokens on `boundless-events`.

Upgrades are timelocked three-step operations: `propose_upgrade(wasm_hash, version)`, wait the timelock (~1 day on mainnet), `apply_upgrade()`, then `migrate()` to stamp the migration marker. A pending proposal can be inspected with `get_pending_upgrade()` and withdrawn with `cancel_pending_upgrade()`.

## Contributing

See `CONTRIBUTING.md`.
