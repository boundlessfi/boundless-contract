# boundless-contract

The Soroban contracts behind Boundless on Stellar: escrow and records for
hackathons, bounties, grants and crowdfunding, plus builder reputation and
earnings.

| Contract | Path | Purpose |
|----------|------|---------|
| `boundless-events` | `contracts/events` | Events for the four pillars with escrow held in the contract. Whitelisted tokens, prize floors, winner awards and claims, milestone releases, paged cancellation and refunds, idempotent operations, timelocked upgrades. |
| `boundless-profile` | `contracts/profile` | Per-user reputation and per-token earnings, written by the events contract. |

## Deployments

| Network | Contract | Address | Version |
|---------|----------|---------|---------|
| Mainnet | `boundless-events` | `CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ` | `1.7.0` |
| Mainnet | `boundless-profile` | `CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC` | `1.2.0` |
| Testnet | `boundless-events` | `CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP` | `2.0.0` |
| Testnet | `boundless-profile` | `CCA3OAIBOZBUPHPRI5GI6N5PDTE7RTNLKAEID4JTC2YZIHIZNDX5Q6T3` | `1.2.0` |

This tree builds events `2.0.0` and profile `1.2.1`. Mainnet moves to them
through the procedure in `docs/upgrade-runbook.md` (Section 8); until then it
runs the versions above. Both contracts answer `version()`, so read it rather
than trusting this table.

On mainnet both contracts are administered by the 2-of-3 multisig
`GCVK72I6TVJVDTTY4UKU6MQT4QJ2T2AAG3NULNEUDM46L3UOQYDSO4O2`. Fees go to
`GADSIP2HPINWTMEUNMVYA5NJRL372OV52HDEJTRTLDZXMHADIPQNYH55` at 250 bps, and the
whitelisted tokens are USDC (`CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75`)
and native XLM (`CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA`).

## How it fits together

An owner creates an event with `create_event`; the budget moves into the
contract and the platform fee goes to the fee account in the same call
(crowdfunding starts empty and is charged at release instead). Anyone can top
an event up with `add_funds`. The owner, or a manager the owner has delegated
to, records awards with `select_winners`: hackathons and bounties in batches of
at most 20, grants once with at most 40 recipients. No award may fall below the
event's published prize floor for its position.
Hackathon and bounty winners collect with `claim_prize`; grant awards and
crowdfunding funds are paid per milestone with `claim_milestone`, and an owner
can `forfeit_milestone` a grant milestone it rejects. A crowdfunding release
also needs the release validator's signature, or the admin's while none is
appointed. Cancellation is paged (`start_cancel`, `process_cancel_batch` with at
most 15 refunds per call, `finalize_cancel`), and a refund an account cannot
receive is held for it to collect with `claim_refund`.

Applying and submitting are off chain since 2.0.0; the contract keeps what
money depends on. The profile contract accepts reputation bumps and earnings
only from the events contract it is bound to; the admin can lower a
reputation with `admin_slash_reputation`. Every mutating call carries an
`op_id` for idempotency, both contracts can be paused, and upgrades are
timelocked: `propose_upgrade`, then `apply_upgrade` 17,280 ledgers later on
mainnet builds, then `migrate_events` (events) and `migrate`. More in
`docs/ARCHITECTURE.md`.

## Toolchain

- Rust 1.93.0 with the `wasm32v1-none` target (`rust-toolchain.toml`).
- soroban-sdk 28.0.0 (workspace `Cargo.toml`).
- Stellar CLI 28.1.0 (`cargo install --locked stellar-cli@28.1.0`).

Build release wasm with `scripts/build-release.sh`, which runs
`stellar contract build --locked` with machine paths remapped so the same
commit gives the same hash on any machine. SDK 28 refuses a plain
`cargo build --target wasm32v1-none`.

## Build and test

```sh
cargo test --workspace

# Release wasm as mainnet runs it (17,280-ledger upgrade timelock)
./scripts/build-release.sh --package boundless-events
./scripts/build-release.sh --package boundless-profile

# Testnet wasm (no upgrade timelock)
./scripts/build-release.sh --package boundless-events --features testnet

# Size against the 64 KB ceiling
cd contracts/events && make size && cd ../..

./scripts/check-versions.sh              # Cargo.toml, INITIAL_VERSION and contractmeta agree
./scripts/test-admin-scripts.sh          # admin pre-flight scripts against fixtures
./scripts/test-mainnet-upgrade-guards.sh # deploy_mainnet.sh guards against a mock CLI
./scripts/test-storage-compat.sh         # replays the mainnet upgrade on real storage
```

Test snapshots (`test_snapshots/`) are gitignored. `cargo test` regenerates
them; inspect the diff locally when reviewing a storage or auth change, but
never commit them.

## Repo layout

```
boundless-contract/
├── Cargo.toml                      # workspace
├── rust-toolchain.toml
├── deploy_mainnet.sh               # mainnet deploy and upgrade tool (prepares multisig XDRs)
├── deploy_and_upgrade.sh           # testnet upgrade helper
├── .env.deploy.example             # parameters for scripts/deploy/deploy.sh
├── contracts/
│   ├── events/src/                 # lib.rs, admin.rs, event_ops.rs, grant.rs, escrow.rs,
│   │                               # token_whitelist.rs, idempotency.rs, profile_client.rs,
│   │                               # storage.rs, types.rs, errors.rs, events.rs, tests/
│   ├── profile/src/                # lib.rs, admin.rs, bootstrap.rs, reputation.rs, earnings.rs,
│   │                               # idempotency.rs, storage.rs, types.rs, errors.rs, events.rs, tests/
│   └── compatibility/              # storage compatibility suite and its fixtures
├── scripts/
│   ├── deploy/                     # deploy.sh, register_token.sh, verify.sh (testnet, futurenet)
│   ├── admin/                      # verify-multisig.sh, verify-fee-trustline.sh
│   ├── lib/deploy-env.sh           # shared helpers for the deploy scripts
│   ├── testnet/                    # grant-scenarios.ts, a testnet grant lifecycle run
│   ├── testdata/                   # Horizon fixtures and the mock stellar CLI
│   ├── check-versions.sh
│   ├── test-admin-scripts.sh
│   ├── test-mainnet-upgrade-guards.sh
│   ├── test-storage-compat.sh
│   └── capture-mainnet-compat-snapshot.sh
├── docs/                           # see below
└── deployments/                    # local deploy records, gitignored
```

## Docs

| Doc | What it is for |
|-----|----------------|
| `docs/ARCHITECTURE.md` | What lives on chain and why, roles, cross-contract calls |
| `docs/DEPLOYMENT.md` | Fresh deploy on testnet or mainnet, token registration, admin rotation |
| `docs/upgrade-runbook.md` | Upgrading live contracts, including mainnet 1.7.0 to 2.0.0 |
| `docs/contract-ops-runbook.md` | Building, simulating, signing and submitting admin transactions |
| `docs/multisig-guide.md` | Plain-English guide for signers; provisioning the multisig |
| `docs/admin-custody-policy.md` | Custody policy: signers, thresholds, rotation, release validator |
| `docs/threat-model.md` | Threat model for the 2.0.0 contracts |
| `docs/scout-audit-report.md` | Static analysis report |
| `docs/storage-compatibility.md` | The storage compatibility suite |
| `docs/dune-analytics.md` | Emitted events and Dune queries (`docs/dune-queries/`) |
| `docs/push-payouts-feasibility.md` | Proposal: paying prizes at selection again |
| `docs/audit/testnet-grant-run.md` | Latest testnet grant scenario run |
| `docs/history/` | Finished procedures and the June 2026 audit, kept for the record |

Follow-ups live in `BACKLOG.md`. Contributing: `CONTRIBUTING.md`.
