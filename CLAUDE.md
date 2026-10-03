# Working in this repo

This repo holds the boundless-events and boundless-profile Soroban contracts. Read this and `BACKLOG.md` before changing the contract surface or deploy scripts.

## Use the Stellar dev skill

The Stellar Development Foundation publishes a Claude Code skill that bundles current Soroban patterns, audit checklists, SDK references, and SEP/CAP knowledge. Install once per machine:

```
/plugin marketplace add stellar/stellar-dev-skill
/plugin install stellar-dev@stellar-dev
```

After install, the seven sub-skills (`soroban`, `dapp`, `assets`, `data`, `agentic-payments`, `zk-proofs`, `standards`) become available across sessions. Lean on `soroban/` for contract changes and audit prep; lean on `dapp/` and `assets/` only when the work crosses into the frontend wallet or trustline flows.

Source: <https://github.com/stellar/stellar-dev-skill>

## Hard rules

- **No `unwrap()` on host-returned `Option`/`Result` in contract code.** Return a typed `Error` instead. Existing patterns: `Error::EventNotFound`, `Error::InsufficientEscrow`, `Error::WinnersAlreadySelected`.
- **Storage layout is stable.** Adding a field to `EventRecord` or any persisted struct must extend, never reorder, and must ship with a corresponding migration story (see `docs/upgrade-runbook.md`: `propose_upgrade` / `apply_upgrade`, then `migrate_events` and `migrate`), proven by the mainnet replay in `docs/storage-compatibility.md`.
- **Per-event configuration over global constants.** Anything sales might want to vary per program (fees, windows, caps) belongs on `EventRecord` or its variant payload, not in module constants.
- **Tests cover the math.** Every payout split (single + multi-position + sweep) has a test that asserts both the recipient and the fee account deltas.
- **Snapshots are inspection tooling, not history.** `test_snapshots/` is gitignored: snapshots are derived artifacts that `cargo test` regenerates from any commit, and committing them made PR diffs so large that the security scanners skipped them. When reviewing a storage-layout or auth change, regenerate locally and inspect the snapshot diff, but never commit snapshot files.
- **Comment sparingly.** A comment earns its place only by stating a constraint the code cannot: an invariant, a security ordering, a compatibility trap, a non-obvious "why". Do not narrate what the code does, restate the function name, tag changes with version/PR numbers, or leave banners over self-evident blocks. When in doubt, delete it: dense explanatory comments read as AI-generated and make review harder, not easier. Match the density of the surrounding file.

## Build, test, deploy

```bash
# Build (soroban-sdk 28 refuses a plain `cargo build --target wasm32v1-none`)
stellar contract build --locked

# Test (host target)
cargo test -p boundless-events
cargo test -p boundless-profile

# Replay the next mainnet upgrade against captured mainnet storage
./scripts/test-storage-compat.sh

# Fresh deploy (testnet)
scripts/deploy/deploy.sh

# Upgrade testnet
./deploy_and_upgrade.sh <action> <events|profile> <arg|-> testnet

# Deploy / upgrade mainnet
./deploy_mainnet.sh           # see docs/DEPLOYMENT.md and docs/upgrade-runbook.md
```

Mainnet admin operations live behind the multi-sig defined in `docs/admin-custody-policy.md`. Never touch mainnet without confirming the runbook prerequisites first.

## Before opening a PR

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --workspace
./scripts/test-storage-compat.sh
```

Update `BACKLOG.md` if your PR closes one of the entries there.

@AGENTS.md
Never add "Co-Authored-By" lines to commits
