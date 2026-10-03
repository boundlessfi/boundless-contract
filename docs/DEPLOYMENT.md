# Deploying the contracts

This covers a fresh deployment of `boundless-profile` and `boundless-events`:
build, deploy, wire the two together, register tokens, verify, and on mainnet
hand admin to the multisig. Upgrading a live deployment is in
`docs/upgrade-runbook.md`; how a multisig transaction is built, signed and
submitted is in `docs/contract-ops-runbook.md`; who may sign what is in
`docs/admin-custody-policy.md`.

Testnet and futurenet deploy with `scripts/deploy/*.sh`. Mainnet deploys with
`./deploy_mainnet.sh`, which adds a typed confirmation, checks the CLI network
config, and refuses to deploy over an existing record.

## 1. What a deployment is made of

| Parameter | Where it goes |
|---|---|
| Admin | Constructor of both contracts. A single CLI identity on testnet. On mainnet a throwaway deployer key that hands admin to the 2-of-3 multisig straight after the deploy. |
| Fee account | Events constructor (`fee_account`). Receives the platform fee on every deposit and crowdfunding release, so it needs an authorized trustline for every registered token. Keep it separate from the admin. |
| Fee bps | Events constructor (`fee_bps`). 250 is 2.5%. The contract refuses fees above 1000 (10%). |
| Profile binding | Events constructor (`profile_contract`), then `profile.set_events_contract(events)`. The second call works once; later changes go through the timelocked rotation in `docs/upgrade-runbook.md`. |
| Tokens | `register_supported_token` on events, one call per token. |

The profile constructor takes only `--admin`. The events constructor takes
`--admin --fee_account --fee_bps --profile_contract`, so profile is always
deployed first.

Each deploy writes a local record to `deployments/<network>.json` (and
mainnet upgrades append to `deployments/mainnet-upgrades.jsonl`). The
`deployments/` directory is gitignored: the records live on the operator's
machine, so keep a copy with the team's ops notes. Published contract ids are
in the README.

## 2. Toolchain

- Rust 1.93.0 with the `wasm32v1-none` target (`rust-toolchain.toml` pins it).
- soroban-sdk 28.0.0 (workspace `Cargo.toml`).
- Stellar CLI 28.1.0, the version CI pins; `deploy_mainnet.sh` refuses any
  other. `cargo install --locked stellar-cli@28.1.0`.
- `jq`.

Build with `scripts/build-release.sh`, which runs `stellar contract build
--locked` with machine paths remapped, so the same commit gives the same hash
on any machine of the same platform. Mainnet builds use
`scripts/build-release-linux.sh`, which runs that build in CI's Linux x86-64
environment through Docker, so its hashes match CI's from any machine. A plain
`cargo build --target wasm32v1-none` is refused by SDK 28. Testnet and
futurenet builds add `--features testnet`, which sets the upgrade timelock to
0 ledgers; mainnet builds leave it off and carry the 17,280-ledger timelock.
The scripts choose the right build for the network.

## 3. Testnet and futurenet

### 3.1 Identities

```sh
stellar keys generate boundless-admin --network testnet --fund
stellar keys generate boundless-fee --network testnet --fund
stellar keys address boundless-fee      # this is FEE_ACCOUNT
```

The fee account never signs a contract call. It only needs to exist and hold
trustlines.

### 3.2 Parameters

```sh
cp .env.deploy.example .env.deploy
$EDITOR .env.deploy      # ADMIN_IDENTITY, FEE_ACCOUNT, FEE_BPS
```

`.env.deploy` is gitignored and is parsed as plain `KEY=VALUE` lines, never
executed.

### 3.3 Deploy and wire

```sh
./scripts/deploy/deploy.sh testnet
```

The script builds both contracts with `--features testnet`, deploys profile,
deploys events against it, calls `profile.set_events_contract`, and writes
`deployments/testnet.json` (including `deployer_identity`, which
`./deploy_and_upgrade.sh` uses as its default signer). It prints the two
contract ids for the backend environment.

### 3.4 Register tokens

Give the fee account a trustline first, then register:

```sh
stellar tx new change-trust --source boundless-fee --network testnet \
  --line USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5

./scripts/deploy/register_token.sh testnet \
  CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA \
  USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
```

`CBIELTK6…` is the testnet USDC SAC. The script checks that the asset you name
is the one the token contract wraps (`stellar contract id asset`) and runs
`scripts/admin/verify-fee-trustline.sh` before it signs; the contract checks
neither, and a missing trustline makes every `create_event` and `add_funds` in
that token revert. For native XLM pass `native` as the asset and the id that
`stellar contract id asset --asset native --network testnet` prints.

### 3.5 Verify

```sh
./scripts/deploy/verify.sh testnet
```

It prints the record next to each contract's version, migration marker, admin,
fee account, fee bps, bindings, release validator and pause flag. Both admins
should be the deployer, the bindings should point at each other, and neither
contract should be paused.

## 4. Mainnet

### 4.1 Before you start

- The release has been through the third-party audit, and the same commit has
  been deployed and exercised on testnet.
- The admin multisig is provisioned and passes
  `scripts/admin/verify-multisig.sh` with the signer roster pinned
  (`docs/multisig-guide.md`, Part E).
- The fee account exists, is funded, and holds an authorized trustline for
  every token you will register.
- Two people who can stop the deploy are present for the whole window.

### 4.2 Network and environment

Configure a named network once and use only that name. `deploy_mainnet.sh`
checks that it carries the public passphrase and a real RPC URL, and refuses
to run while `STELLAR_RPC_URL` or `STELLAR_NETWORK_PASSPHRASE` is set (mixing
the two breaks RPC resolution; see `docs/contract-ops-runbook.md`, rule 5).

```sh
stellar network add boundless-mainnet \
  --rpc-url <production RPC URL> \
  --network-passphrase "Public Global Stellar Network ; September 2015"
export STELLAR_NETWORK=boundless-mainnet
unset STELLAR_RPC_URL STELLAR_NETWORK_PASSPHRASE

stellar keys generate boundless-deployer      # throwaway initial admin
stellar keys address boundless-deployer       # fund it with about 10 XLM

export INITIAL_ADMIN_KEY=boundless-deployer
export FEE_ACCOUNT=G...                       # the fee account
export INITIAL_GLOBAL_FEE_BPS=250
```

The deployer key exists only to deploy and rotate. It is destroyed in 4.6.

### 4.3 Deploy and wire

```sh
./deploy_mainnet.sh deploy-profile
./deploy_mainnet.sh deploy-events
```

Each command asks you to type `mainnet`, builds its contract without the
testnet feature, deploys it, and records the id and the deployed wasm hash in
`deployments/mainnet.json`. It stops if the deployed hash differs from the
local build. `deploy-events` also calls `profile.set_events_contract`. Either
command refuses to run if the record already names that contract.

### 4.4 Register tokens

```sh
./deploy_mainnet.sh register-token \
  CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75 \
  USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN
```

`CCW67TSZ…` is the mainnet USDC SAC. As on testnet, the asset must be the one
the contract wraps and the fee account trustline must pass before anything is
signed. For native XLM pass `native` and the id from
`stellar contract id asset --asset native --network "$STELLAR_NETWORK"`.
`register-token` only works while the deployer is still admin; after the
rotation, register through the multisig (`docs/contract-ops-runbook.md`,
Section 5) and run `verify-fee-trustline.sh` first.

### 4.5 Verify, smoke, rotate admin

```sh
ADMIN_SOURCE=$(stellar keys address boundless-deployer) ./deploy_mainnet.sh verify
```

Before rotating, run one small end-to-end flow against the new contracts with
the backend's smoke scripts (see the boundless-nestjs repo, which also owns
the env values, migrations and orchestrator checks that follow a deploy). If
anything is wrong, the deployer can still `pause` both contracts on its own.

Then hand admin to the multisig:

```sh
EXPECTED_SIGNERS=<G1>,<G2>,<G3> ./deploy_mainnet.sh rotate-admin <MULTISIG_G_ADDRESS>
```

`rotate-admin` runs `scripts/admin/verify-multisig.sh` against the multisig
(and the roster, when `EXPECTED_SIGNERS` is set), asks for the typed
confirmation, and calls `set_admin` on both contracts from the deployer. The
multisig then accepts on each contract. Wait about ten seconds after
`set_admin` so the nomination is in a closed ledger, then build and simulate:

```sh
EVENTS_ID=$(jq -r .events_contract deployments/mainnet.json)
stellar contract invoke --network "$STELLAR_NETWORK" \
  --source-account <MULTISIG_G_ADDRESS> --id "$EVENTS_ID" --build-only \
  -- accept_admin \
  | stellar tx simulate --network "$STELLAR_NETWORK" \
      --source-account <MULTISIG_G_ADDRESS> > events-accept-admin.xdr
```

Sign it sequentially with two signers and submit it with `stellar tx send`
(`docs/contract-ops-runbook.md`, Section 5). Repeat for the profile contract.
Then confirm both admins changed and record the four transaction hashes:

```sh
ADMIN_SOURCE=<MULTISIG_G_ADDRESS> ./deploy_mainnet.sh verify
jq --arg a <MULTISIG_G_ADDRESS> '.admin = $a' deployments/mainnet.json > tmp.json \
  && mv tmp.json deployments/mainnet.json
```

Prove the signing path once with a no-op admin call, for example
`set_fee_bps` to its current value.

### 4.6 Destroy the deployer key

```sh
stellar keys rm boundless-deployer
```

The account keeps any leftover XLM but has no authority over either contract.
Sweep it first if you want the balance back.

## 5. If a deploy goes wrong

Before the rotation, the deployer pauses either contract directly
(`stellar contract invoke ... -- pause`). After it, pausing is a 2-of-3
multisig call: `./deploy_mainnet.sh prepare-pause-events <xdr-output>`.

A fresh deploy has no in-place undo. If the contracts are misconfigured in a
way the admin setters cannot fix, deploy a new pair, point the backend at the
new ids, and leave the old pair paused.
