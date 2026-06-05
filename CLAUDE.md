# Working in this repo

This repo holds the boundless-events and boundless-profile Soroban contracts. Read this and `BACKLOG.md` before changing the contract surface or deploy scripts.

## Use the Stellar dev skill

The Stellar Development Foundation publishes a Claude Code skill that bundles current Soroban patterns, audit checklists, SDK references, and SEP/CAP knowledge. Install once per machine:

```
/plugin marketplace add stellar/stellar-dev-skill
/plugin install stellar-dev@stellar-dev
```

After install, the seven sub-skills (`soroban`, `dapp`, `assets`, `data`, `agentic-payments`, `zk-proofs`, `standards`) become available across sessions. Lean on `soroban/` for contract changes and audit prep; lean on `dapp/` and `assets/` only when the work crosses into the frontend wallet or trustline flows.

Source: https://github.com/stellar/stellar-dev-skill

## Hard rules

- **No `unwrap()` on host-returned `Option`/`Result` in contract code.** Return a typed `Error` instead. Existing patterns: `Error::EventNotFound`, `Error::InsufficientEscrow`, `Error::WinnersAlreadySelected`.
- **Storage layout is stable.** Adding a field to `EventRecord` or any persisted struct must extend, never reorder, and must ship with a corresponding migration story (see `docs/mainnet-deploy-runbook.md` and the `upgrade()` admin function).
- **Per-event configuration over global constants.** Anything sales might want to vary per program (fees, windows, caps) belongs on `EventRecord` or its variant payload, not in module constants.
- **Tests cover the math.** Every payout split (single + multi-position + sweep) has a test that asserts both the recipient and the fee account deltas.
- **Snapshots are the audit trail.** When test fixtures change shape, regenerate snapshots intentionally and commit them in the same PR.

## Build, test, deploy

```bash
# Build
cd contracts/events && cargo build --target wasm32-unknown-unknown --release

# Test (host target)
cargo test -p boundless-events

# Deploy / upgrade testnet
./deploy_and_upgrade.sh

# Deploy / upgrade mainnet
./deploy_mainnet.sh           # see docs/mainnet-deploy-runbook.md
```

Mainnet admin operations live behind the multi-sig defined in `docs/admin-custody-policy.md`. Never touch mainnet without confirming the runbook prerequisites first.

## Before opening a PR

```bash
cargo test -p boundless-events
cargo build --release --target wasm32-unknown-unknown
```

Update `BACKLOG.md` if your PR closes one of the entries there.
