# Deployment runbook

Deploys the `boundless-events` and `boundless-profile` Soroban contracts to a
Stellar network, wires them together, and emits the env values the nestjs
orchestrator needs.

Scripts: `scripts/deploy/{deploy,register_token,verify}.sh`.

---

## 0. Decisions to lock before deploying

Each of these is a per-network parameter the contracts are constructed with.
Pick once per environment and document.

| Parameter | Where it goes | What it does |
|---|---|---|
| **Network** | `stellar --network` | testnet / futurenet / mainnet. |
| **Admin identity** | both contracts' `admin` | controls `set_admin`, `set_fee_bps`, `pause`, `upgrade`, `register_supported_token`. **Use a Stellar multisig account for mainnet.** Testnet can be a single key. |
| **Fee account** | events `fee_account` | G-address that receives the platform fee on every deposit. Must hold a trustline for every registered token. **Should be a separately-keyed account from admin.** |
| **Fee bps** | events `fee_bps` | Basis points. 100 = 1%, 250 = 2.5%. Contract caps at 1000 (10%) per audit L4. |
| **Bootstrap credits** | profile `default_bootstrap_credits` | u32. Initial credit balance for newly-created profiles. PRD default is 10. |
| **USDC asset address** | events `register_supported_token` | Token contract (SAC) address. See Section 4. |

Production values land in `deployments/<network>.json` after `deploy.sh` runs.

---

## 1. Prerequisites (one-time per workstation)

```sh
# stellar CLI — version 26.0.0 or newer required.
# Older CLIs reject Rust 1.90.0 wasm32v1-none output with
# "reference-types not enabled" at simulation time.
brew install stellar/tap/stellar-cli   # or cargo install --locked stellar-cli@26.1.0
stellar --version                      # confirm >= 26.0.0

# rust toolchain
rustup target add wasm32v1-none

# verify build still passes locally
cd contracts/events && stellar contract build && cd ../..
cd contracts/profile && stellar contract build && cd ../..
```

### Create the admin identity (per network)

```sh
# Generates and funds a testnet admin identity via friendbot.
stellar keys generate boundless-admin --network testnet --fund

# Show its G-address.
stellar keys address boundless-admin
```

For mainnet, generate without `--fund`, send XLM manually, and convert the
account to a multisig before any contract deploy. See the wallet runbook in
`boundless-infra/`.

### Create or pick the fee account

```sh
stellar keys generate boundless-fee --network testnet --fund
stellar keys address boundless-fee   # -> G... ; this goes into FEE_ACCOUNT
```

The fee account does NOT need to be a CLI identity (it never signs contract
calls). Treat the value as data; what matters is that it has trustlines for
every token before that token is registered.

---

## 2. Configure deployment parameters

```sh
cp .env.deploy.example .env.deploy
$EDITOR .env.deploy
```

Required values:

```
ADMIN_IDENTITY=boundless-admin
FEE_ACCOUNT=G...                  # output of `stellar keys address boundless-fee`
FEE_BPS=250                       # 2.5%
BOOTSTRAP_CREDITS=10              # PRD default
```

`.env.deploy` is gitignored. Do not commit.

---

## 3. Deploy + wire

```sh
./scripts/deploy/deploy.sh testnet
```

The script does, in order:

1. `stellar contract build` on both contracts.
2. Deploys `boundless-profile` first (events constructor needs its address).
3. Deploys `boundless-events` with `fee_account`, `fee_bps`, and the profile contract id.
4. `profile.set_events_contract(events_id)` so profile recognizes the events contract for cross-contract auth.
5. Writes the deployment record to `deployments/testnet.json`.
6. Prints the env-var lines for the nestjs side.

Expected output tail:

```
==> done. summary written to deployments/testnet.json

set these in the nestjs deployment env:
  BOUNDLESS_EVENTS_CONTRACT_ADDRESS=C...
  BOUNDLESS_PROFILE_CONTRACT_ADDRESS=C...

next: ./scripts/deploy/register_token.sh testnet <token-address>
```

---

## 4. Add the fee-account trustline + register the token

USDC on testnet uses the Circle test issuer. The token contract address
depends on whether you wrap the Stellar Classic asset or use Circle's Soroban
deployment directly. Check Stellar's developer portal for the current testnet
USDC SAC address before running this step.

### 4a. Add the trustline (off-chain)

The contract's `register_supported_token` does NOT verify the trustline (a
Soroban contract cannot authorize a deeper-than-root call). The admin runbook
covers it instead:

```sh
# Where boundless-fee is the CLI identity that owns the FEE_ACCOUNT key.
# (If FEE_ACCOUNT is a separate non-CLI account, run this on that account's
#  signing setup.)
stellar tx new change-trust \
  --source boundless-fee \
  --network testnet \
  --line USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5

# Verify:
stellar contract invoke \
  --id CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75 \
  --network testnet \
  -- balance --id "$(stellar keys address boundless-fee)"
```

(`CCW67TSZV3SSS...` is the testnet USDC SAC address at the time of writing;
verify against current docs.)

### 4b. Register on the events contract

```sh
./scripts/deploy/register_token.sh testnet <USDC_SAC_ADDRESS>
```

The script prompts to confirm the trustline exists, then invokes
`register_supported_token`. The record updates `deployments/testnet.json` with
the supported token.

---

## 5. Verify

```sh
./scripts/deploy/verify.sh testnet
```

Cross-checks the deployment record against on-chain state. Both contracts
should report the expected admin, the events contract should report the
profile contract id, the profile contract should report the events contract
address, and both should be `is_paused == false`.

If anything looks wrong, treat it as a deploy regression and re-deploy fresh.
Soroban storage costs are real but minimal at this scale.

---

## 6. Update nestjs orchestrator env

Take the `BOUNDLESS_EVENTS_CONTRACT_ADDRESS` and
`BOUNDLESS_PROFILE_CONTRACT_ADDRESS` lines from `deploy.sh`'s output and put
them in the nestjs deployment environment (Railway, etc.):

```
STELLAR_RPC_URL=https://soroban-rpc.testnet.stellar.org
STELLAR_NETWORK_PASSPHRASE=Test SDF Network ; September 2015
BOUNDLESS_EVENTS_CONTRACT_ADDRESS=C...
BOUNDLESS_PROFILE_CONTRACT_ADDRESS=C...
```

The env schema (`src/config/env-schema.ts`) validates both addresses are
56-char strings starting with `C`. Mistakes surface at app boot.

---

## 7. Apply the Prisma migration

Against the active database (dev / staging / production):

```sh
cd ../boundless-nestjs
npx prisma migrate deploy
```

Creates the `escrow_op` table + three enums. Migration file:
`prisma/migrations/20260601213000_add_escrow_op/migration.sql`.

Verify:

```sh
npx prisma db pull --print 2>&1 | grep -A 8 "^model EscrowOp"
# or, on the DB directly:
psql "$DATABASE_URL" -c '\d escrow_op'
```

---

## 8. Smoke-test the orchestrator end-to-end

After the nestjs deployment picks up the new env vars and the migration is
applied:

```sh
# From a node REPL or a small script in the nestjs repo:
import { EscrowOrchestratorService } from './modules/escrow-contract/...';

// Build an unsigned create_event XDR. No tokens move; this only assembles
// the transaction and stores an EscrowOp row.
const op = await orchestrator.beginCreateEvent({
  entityKind: 'HACKATHON',
  entityId: 'hck_smoke_test',
  params: { /* CreateEventParams */ },
});

console.log(op.opId, op.status, op.unsignedXdr);
```

The expected state is `PENDING_SIGN` with a non-empty `unsignedXdr`. If the
RPC call fails, check the contract address env vars and that the deployment
record matches what's on chain.

---

## 9. Rollback

The contract layer has no migration story for breaking changes; a bad deploy
means deploying fresh and pointing the orchestrator at the new addresses.

For the orchestrator, every `EscrowOp` row carries the contract addresses
it targeted (via `signer_hint` + the deployment record); a redeploy creates
new event_ids and a new namespace.

Migration rollback:

```sh
# Reverse the schema migration manually if needed.
psql "$DATABASE_URL" -c 'DROP TABLE escrow_op; DROP TYPE "EscrowOpKind"; DROP TYPE "EscrowOpStatus"; DROP TYPE "EscrowOpEntityKind";'
# Then delete the migration directory and re-generate the Prisma client.
```

Or, more cleanly, write a follow-up migration that drops the table.

---

## Cross-references

- `scripts/deploy/deploy.sh`, `register_token.sh`, `verify.sh`
- `boundless-platform-contract-prd.md` Section 12 (deployment)
- `boundless-payout-prd.md` Section 9.1 (orchestrator integration)
- `boundless-credits-reputation-prd.md` Section 9 (bootstrap credit policy)
- `.env.deploy.example` (per-network parameter template)
