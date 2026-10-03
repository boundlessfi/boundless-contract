/**
 * Runs the grant lifecycle against the deployed events contract on testnet,
 * with fresh accounts and the BGT test asset, and checks every balance the
 * contract moves. Writes a report with each transaction.
 *
 *   cd scripts/testnet && npm install && npm run grants
 *
 * Needs the stellar CLI identities `boundless-grant-test-issuer` (issues BGT,
 * revocable; funds the run) and the events contract to have BGT whitelisted
 * with the fee account trusting it. EVENTS_CONTRACT overrides the contract id.
 */
import { execFileSync } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  Account,
  Address,
  Asset,
  BASE_FEE,
  Contract,
  Horizon,
  Keypair,
  Networks,
  Operation,
  TransactionBuilder,
  authorizeEntry,
  nativeToScVal,
  rpc,
  scValToNative,
  xdr,
} from '@stellar/stellar-sdk';

const RPC_URL = process.env.STELLAR_RPC_URL ?? 'https://soroban-testnet.stellar.org';
const HORIZON_URL = 'https://horizon-testnet.stellar.org';
const PASSPHRASE = Networks.TESTNET;
const EVENTS =
  process.env.EVENTS_CONTRACT ?? 'CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP';
const ISSUER_IDENTITY = process.env.ISSUER_IDENTITY ?? 'boundless-grant-test-issuer';
const VALIDATOR_IDENTITY =
  process.env.VALIDATOR_IDENTITY ?? 'boundless-release-validator-testnet';
const CAPACITY = Number(process.env.CAPACITY_RECIPIENTS ?? 40);
const ONLY = process.env.ONLY?.split(',').map((s: string) => s.trim());
const wanted = (name: string) => !ONLY || ONLY.some((o: string) => name.startsWith(o));

const server = new rpc.Server(RPC_URL);
const horizon = new Horizon.Server(HORIZON_URL);
const U = 10_000_000n;
const bgt = (units: number | bigint) => BigInt(units) * U;

// ---------------------------------------------------------------- reporting

type Check = { scenario: string; label: string; ok: boolean; detail?: string };
type TxLog = { scenario: string; step: string; hash: string; fee: string; ok: boolean };
const checks: Check[] = [];
const txs: TxLog[] = [];
let scenario = 'setup';

function check(ok: boolean, label: string, detail?: string) {
  checks.push({ scenario, label, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'} [${scenario}] ${label}${detail ? ` (${detail})` : ''}`);
}

function eq(actual: unknown, expected: unknown, label: string) {
  const a = typeof actual === 'bigint' ? actual.toString() : JSON.stringify(actual);
  const e = typeof expected === 'bigint' ? expected.toString() : JSON.stringify(expected);
  check(a === e, label, a === e ? undefined : `got ${a}, expected ${e}`);
}

// ---------------------------------------------------------------- encoding

const sv = {
  u32: (n: number) => nativeToScVal(n, { type: 'u32' }),
  u64: (n: bigint) => nativeToScVal(n, { type: 'u64' }),
  i128: (n: bigint) => nativeToScVal(n, { type: 'i128' }),
  addr: (a: string) => new Address(a).toScVal(),
  op: () => xdr.ScVal.scvBytes(randomBytes(32)),
  sym: (s: string) => xdr.ScVal.scvSymbol(s),
  struct(fields: Record<string, xdr.ScVal>) {
    return xdr.ScVal.scvMap(
      Object.keys(fields)
        .sort()
        .map((k) => new xdr.ScMapEntry({ key: sv.sym(k), val: fields[k] })),
    );
  },
};

type Award = { to: string; position: number; amount: bigint; bump?: number };

function winners(awards: Award[]) {
  return xdr.ScVal.scvVec(
    awards.map((a) =>
      sv.struct({
        amount: sv.i128(a.amount),
        position: sv.u32(a.position),
        recipient: sv.addr(a.to),
        reputation_bump: sv.u32(a.bump ?? 10),
      }),
    ),
  );
}

// ---------------------------------------------------------------- chain

const stroopsText = (n: bigint) => `${n / U}.${(n % U).toString().padStart(7, '0')}`;

async function classic(source: Keypair, ops: xdr.Operation[], signers: Keypair[], step: string) {
  const account = await horizon.loadAccount(source.publicKey());
  const tx = new TransactionBuilder(account, {
    fee: (Number(BASE_FEE) * 10).toString(),
    networkPassphrase: PASSPHRASE,
  });
  ops.forEach((op) => tx.addOperation(op));
  const built = tx.setTimeout(120).build();
  built.sign(source, ...signers);
  const res = await horizon.submitTransaction(built);
  txs.push({ scenario, step, hash: res.hash, fee: built.fee, ok: true });
}

type Outcome =
  | { ok: true; value: unknown; hash: string; fee: string }
  | { ok: false; code?: number; detail: string; hash?: string };

/** The address an auth entry needs a signature from, or null for source credentials. */
function signerOf(entry: xdr.SorobanAuthorizationEntry): string | null {
  const c = entry.credentials;
  if (c.type === 'sorobanCredentialsAddress') {
    return Address.fromScAddress(c.address.address).toString();
  }
  if (c.type === 'sorobanCredentialsAddressV2') {
    return Address.fromScAddress(c.addressV2.address).toString();
  }
  return null;
}

const contractCode = (text: string) => {
  const m = /Error\(Contract, #(\d+)\)/.exec(text);
  return m ? Number(m[1]) : undefined;
};

type InvokeOpts = { signers?: Keypair[]; simulate?: boolean; step?: string };

// The RPC sometimes serves an account a ledger behind, so a transaction built
// right after the previous one confirms can carry a stale sequence. Those
// rejections never reach the contract; rebuild and resend.
async function invoke(
  contractId: string,
  source: Keypair,
  method: string,
  args: xdr.ScVal[],
  opts: InvokeOpts = {},
): Promise<Outcome> {
  for (let attempt = 1; ; attempt++) {
    const outcome = await invokeOnce(contractId, source, method, args, opts);
    const transient =
      !outcome.ok && /send (TRY_AGAIN_LATER|ERROR: txBadSeq)/.test(outcome.detail);
    if (!transient || attempt === 4) return outcome;
    console.log(`retry ${method} after ${outcome.detail}`);
    await new Promise((r) => setTimeout(r, 3000));
  }
}

async function invokeOnce(
  contractId: string,
  source: Keypair,
  method: string,
  args: xdr.ScVal[],
  opts: InvokeOpts,
): Promise<Outcome> {
  const loaded = await server.getAccount(source.publicKey());
  // build() advances the account's sequence, and a call that signs another
  // party's auth entry builds twice, so each build starts from the same one.
  const build = (op: xdr.Operation) =>
    new TransactionBuilder(new Account(loaded.accountId(), loaded.sequenceNumber()), {
      fee: BASE_FEE,
      networkPassphrase: PASSPHRASE,
    })
      .addOperation(op)
      .setTimeout(300)
      .build();

  let tx = build(new Contract(contractId).call(method, ...args));
  let sim = await server.simulateTransaction(tx);
  if (rpc.Api.isSimulationError(sim)) {
    return { ok: false, code: contractCode(sim.error), detail: sim.error.split('\n')[0] };
  }
  if (opts.simulate) {
    return {
      ok: true,
      value: sim.result?.retval ? scValToNative(sim.result.retval) : undefined,
      hash: '',
      fee: sim.minResourceFee,
    };
  }

  // Address-credential entries need a signature from that address; sign the
  // ones we hold and leave the rest unsigned, so a missing signer fails on chain.
  const auth = sim.result?.auth ?? [];
  const needsSigning = auth.some((e) => signerOf(e) !== null);
  if (needsSigning) {
    const { sequence } = await server.getLatestLedger();
    const signed = await Promise.all(
      auth.map(async (entry) => {
        const who = signerOf(entry);
        if (!who) return entry;
        const signer = (opts.signers ?? []).find((k) => k.publicKey() === who);
        return signer ? authorizeEntry(entry, signer, sequence + 100, PASSPHRASE) : entry;
      }),
    );
    const func = (tx.operations[0] as Operation.InvokeHostFunction).func;
    tx = build(Operation.invokeHostFunction({ func, auth: signed }));
    sim = await server.simulateTransaction(tx);
    if (rpc.Api.isSimulationError(sim)) {
      return { ok: false, code: contractCode(sim.error), detail: sim.error.split('\n')[0] };
    }
  }

  const assembled = rpc.assembleTransaction(tx, sim).build();
  assembled.sign(source);
  const sent = await server.sendTransaction(assembled);
  if (sent.status === 'ERROR' || sent.status === 'TRY_AGAIN_LATER') {
    const code = sent.errorResult?.result.type;
    const detail = `send ${sent.status}${code ? `: ${code}` : ''}`;
    txs.push({ scenario, step: opts.step ?? method, hash: sent.hash, fee: assembled.fee, ok: false });
    return { ok: false, detail, hash: sent.hash };
  }
  for (let i = 0; i < 90; i++) {
    const got = await server.getTransaction(sent.hash);
    if (got.status === rpc.Api.GetTransactionStatus.NOT_FOUND) {
      await new Promise((r) => setTimeout(r, 1000));
      continue;
    }
    const ok = got.status === rpc.Api.GetTransactionStatus.SUCCESS;
    txs.push({ scenario, step: opts.step ?? method, hash: sent.hash, fee: assembled.fee, ok });
    if (!ok) return { ok: false, detail: 'transaction failed on chain', hash: sent.hash };
    return {
      ok: true,
      value: got.returnValue ? scValToNative(got.returnValue) : undefined,
      hash: sent.hash,
      fee: assembled.fee,
    };
  }
  return { ok: false, detail: 'not confirmed in 90s', hash: sent.hash };
}

const events = (source: Keypair, method: string, args: xdr.ScVal[], opts?: InvokeOpts) =>
  invoke(EVENTS, source, method, args, opts);

async function must(outcome: Promise<Outcome>, label: string): Promise<unknown> {
  const o = await outcome;
  check(o.ok, label, o.ok ? undefined : `${o.detail}${o.hash ? ` tx ${o.hash}` : ''}`);
  if (!o.ok) throw new Error(`${scenario}: ${label} failed`);
  return o.value;
}

async function refused(outcome: Promise<Outcome>, code: number | 'auth' | 'any', label: string) {
  const o = await outcome;
  if (o.ok) return check(false, label, 'call succeeded');
  if (code === 'any') return check(true, label, o.detail);
  if (code === 'auth') return check(o.code === undefined, label, o.detail);
  check(o.code === code, label, o.code === code ? undefined : `got ${o.code ?? o.detail}`);
}

// ---------------------------------------------------------------- setup

const secretOf = (identity: string) =>
  execFileSync('stellar', ['keys', 'show', identity], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  }).trim();

const issuer = Keypair.fromSecret(secretOf(ISSUER_IDENTITY));
const asset = new Asset('BGT', issuer.publicKey());
const TOKEN = asset.contractId(PASSPHRASE);
let feeAccount = '';
let feeBps = 0n;

async function balance(who: string): Promise<bigint> {
  const o = await invoke(TOKEN, issuer, 'balance', [sv.addr(who)], { simulate: true });
  if (!o.ok) throw new Error(`balance of ${who}: ${o.detail}`);
  return BigInt(o.value as bigint);
}

async function provision(count: number, mint: bigint, xlm = '5'): Promise<Keypair[]> {
  const out: Keypair[] = [];
  for (let start = 0; start < count; start += 15) {
    const batch = Array.from({ length: Math.min(15, count - start) }, () => Keypair.random());
    const ops: xdr.Operation[] = [];
    for (const k of batch) {
      ops.push(Operation.createAccount({ destination: k.publicKey(), startingBalance: xlm }));
      ops.push(Operation.changeTrust({ asset, source: k.publicKey() }));
      if (mint > 0n) {
        ops.push(
          Operation.payment({ destination: k.publicKey(), asset, amount: stroopsText(mint) }),
        );
      }
    }
    await classic(issuer, ops, batch, `provision ${batch.length} accounts`);
    out.push(...batch);
  }
  return out;
}

async function setFrozen(who: Keypair, frozen: boolean) {
  await classic(
    issuer,
    [Operation.setTrustLineFlags({ trustor: who.publicKey(), asset, flags: { authorized: !frozen } })],
    [],
    frozen ? 'freeze' : 'unfreeze',
  );
}

const fee = (amount: bigint) => (amount * feeBps) / 10_000n;
let feesExpected = 0n;

type GrantSpec = {
  budget: bigint;
  milestones: number;
  floors: [number, bigint][];
  manager?: string;
  pillar?: 'Grant' | 'Crowdfunding';
};

async function createGrant(owner: Keypair, g: GrantSpec): Promise<bigint> {
  const floors = xdr.ScVal.scvMap(
    [...g.floors]
      .sort((a, b) => a[0] - b[0])
      .map(([p, v]) => new xdr.ScMapEntry({ key: sv.u32(p), val: sv.i128(v) })),
  );
  const params = sv.struct({
    content_uri: xdr.ScVal.scvString('https://api.boundless.fi/grants/testnet-run/content'),
    deadline: sv.u64(BigInt(Math.floor(Date.now() / 1000) + 30 * 86_400)),
    fee_bps_override: xdr.ScVal.scvVoid(),
    manager: g.manager ? sv.addr(g.manager) : xdr.ScVal.scvVoid(),
    owner: sv.addr(owner.publicKey()),
    pillar: xdr.ScVal.scvVec([sv.sym(g.pillar ?? 'Grant')]),
    prize_floors: floors,
    release_kind: xdr.ScVal.scvVec([sv.sym('Multi'), sv.u32(g.milestones)]),
    title: xdr.ScVal.scvString(`Testnet grant run: ${scenario}`),
    token: sv.addr(TOKEN),
    total_budget: sv.i128(g.budget),
  });
  const before = await balance(owner.publicKey());
  const id = BigInt(
    (await must(events(owner, 'create_event', [params, sv.op()]), 'publish the grant')) as bigint,
  );
  // Crowdfunding starts at zero escrow and takes its fee at release.
  const paid = g.pillar === 'Crowdfunding' ? 0n : g.budget + fee(g.budget);
  eq(before - (await balance(owner.publicKey())), paid, 'owner paid budget plus fee');
  if (g.pillar !== 'Crowdfunding') feesExpected += fee(g.budget);
  return id;
}

async function event(id: bigint): Promise<Record<string, unknown>> {
  const o = await events(issuer, 'get_event', [sv.u64(id)], { simulate: true });
  if (!o.ok) throw new Error(`get_event ${id}: ${o.detail}`);
  return o.value as Record<string, unknown>;
}

const status = async (id: bigint) => ((await event(id)).status as string[])[0];

async function release(owner: Keypair, id: bigint, to: Keypair | string, m: number, expected: bigint) {
  const who = typeof to === 'string' ? to : to.publicKey();
  const before = await balance(who);
  await must(
    events(owner, 'claim_milestone', [sv.u64(id), sv.addr(who), sv.u32(m), sv.u32(10), sv.op()]),
    `release milestone ${m} to ${who.slice(0, 6)}`,
  );
  eq((await balance(who)) - before, expected, `milestone ${m} paid exactly`);
}

async function forfeit(owner: Keypair, id: bigint, to: Keypair, m: number) {
  const before = await balance(to.publicKey());
  await must(
    events(owner, 'forfeit_milestone', [sv.u64(id), sv.addr(to.publicKey()), sv.u32(m), sv.op()]),
    `forfeit milestone ${m}`,
  );
  eq(await balance(to.publicKey()), before, 'a forfeit pays nothing');
}

async function contribute(id: bigint, from: Keypair, amount: bigint) {
  const before = await balance(from.publicKey());
  await must(
    events(from, 'add_funds', [sv.u64(id), sv.addr(from.publicKey()), sv.i128(amount), sv.op()]),
    `partner adds ${stroopsText(amount)}`,
  );
  eq(before - (await balance(from.publicKey())), amount + fee(amount), 'partner paid amount plus fee');
  feesExpected += fee(amount);
}

async function cancel(manager: Keypair, id: bigint) {
  await must(events(manager, 'start_cancel', [sv.u64(id), sv.op()]), 'start cancel');
  if ((await status(id)) === 'Cancelling') {
    for (;;) {
      const left = await must(
        events(manager, 'process_cancel_batch', [sv.u64(id), sv.u32(15), sv.op()]),
        'refund batch',
      );
      if (Number(left) === 0) break;
    }
    await must(events(manager, 'finalize_cancel', [sv.u64(id), sv.op()]), 'finalize cancel');
  }
  eq(await status(id), 'Cancelled', 'grant cancelled');
}

// ---------------------------------------------------------------- scenarios

async function main() {
  feeAccount = (await events(issuer, 'get_fee_account', [], { simulate: true }) as { value: string }).value;
  feeBps = BigInt((await events(issuer, 'get_fee_bps', [], { simulate: true }) as { value: number }).value);
  const version = (await events(issuer, 'version', [], { simulate: true }) as { value: string }).value;
  const listed = await events(issuer, 'is_supported_token', [sv.addr(TOKEN)], { simulate: true });
  check(listed.ok && listed.value === true, `BGT ${TOKEN} is whitelisted on ${EVENTS} (${version})`);
  const feeStart = await balance(feeAccount);

  // Owners sign every publish and release, and each release pays rent on a new row.
  const [owner, ownerB] = await provision(2, bgt(2_000), '200');
  const partners = await provision(4, bgt(100));
  const people = await provision(CAPACITY + 12, 0n);
  const [manager, managerB, stranger] = await provision(3, 0n);
  const r = (i: number) => people[i];

  scenario = 'micro grant';
  if (wanted(scenario)) {
    const id = await createGrant(owner, {
      budget: bgt(30),
      milestones: 1,
      floors: [[1, bgt(10)], [2, bgt(10)], [3, bgt(10)]],
    });
    await must(
      events(owner, 'select_winners', [
        sv.u64(id),
        winners([1, 2, 3].map((p) => ({ to: r(p - 1).publicKey(), position: p, amount: bgt(10) }))),
        sv.op(),
      ]),
      'select three recipients',
    );
    for (const i of [0, 1, 2]) await release(owner, id, r(i), 0, bgt(10));
    eq(await status(id), 'Completed', 'completes when the last award is paid');
  }

  scenario = 'tiered, rounding, any order';
  if (wanted(scenario)) {
    const [a, b, c] = [bgt(50) + 1n, bgt(30) + 2n, bgt(20)];
    const id = await createGrant(owner, {
      budget: a + b + c,
      milestones: 3,
      floors: [[1, a], [2, b], [3, c]],
    });
    await must(
      events(owner, 'select_winners', [
        sv.u64(id),
        winners([
          { to: r(3).publicKey(), position: 1, amount: a },
          { to: r(4).publicKey(), position: 2, amount: b },
          { to: r(5).publicKey(), position: 3, amount: c },
        ]),
        sv.op(),
      ]),
      'select three tiers',
    );
    const part = (x: bigint) => x / 3n;
    const last = (x: bigint) => x - 2n * part(x);
    await release(owner, id, r(4), 2, part(b));
    await release(owner, id, r(3), 0, part(a));
    await release(owner, id, r(5), 1, part(c));
    await release(owner, id, r(3), 2, part(a));
    await release(owner, id, r(4), 0, part(b));
    await release(owner, id, r(5), 0, part(c));
    await release(owner, id, r(3), 1, last(a));
    await release(owner, id, r(4), 1, last(b));
    await release(owner, id, r(5), 2, last(c));
    eq(await balance(r(3).publicKey()), a, 'tier 1 received its whole award');
    eq(await balance(r(4).publicKey()), b, 'tier 2 received its whole award');
    eq(await status(id), 'Completed', 'completed');
  }

  scenario = 'forfeit and the cancel guard';
  if (wanted(scenario)) {
    const id = await createGrant(owner, { budget: bgt(30), milestones: 3, floors: [[1, bgt(30)]] });
    await must(
      events(owner, 'select_winners', [
        sv.u64(id),
        winners([{ to: r(6).publicKey(), position: 1, amount: bgt(30) }]),
        sv.op(),
      ]),
      'select one recipient',
    );
    await release(owner, id, r(6), 0, bgt(10));
    await forfeit(owner, id, r(6), 1);
    await refused(
      events(owner, 'start_cancel', [sv.u64(id), sv.op()], { simulate: true }),
      96,
      'cancel refused while an award is owed (AwardsOutstanding)',
    );
    await release(owner, id, r(6), 2, bgt(10));
    const before = await balance(owner.publicKey());
    await cancel(owner, id);
    eq((await balance(owner.publicKey())) - before, bgt(10), 'forfeited share returned to the owner');
  }

  scenario = 'partners, pro rata';
  if (wanted(scenario)) {
    const [p1, p2] = partners;
    const id = await createGrant(owner, { budget: bgt(50), milestones: 2, floors: [[1, bgt(10)]] });
    await contribute(id, p1, bgt(20));
    await must(
      events(owner, 'select_winners', [
        sv.u64(id),
        winners([{ to: r(7).publicKey(), position: 1, amount: bgt(60) }]),
        sv.op(),
      ]),
      'award more than the budget, funded by the partner',
    );
    await release(owner, id, r(7), 0, bgt(30));
    await contribute(id, p2, bgt(15));
    await release(owner, id, r(7), 1, bgt(30));
    eq(await status(id), 'Active', 'the late top-up keeps the grant open');
    const [b1, b2, bo] = [p1, p2, owner].map((k) => k.publicKey());
    const before = await Promise.all([b1, b2, bo].map(balance));
    await must(events(owner, 'start_cancel', [sv.u64(id), sv.op()]), 'start cancel');
    await refused(
      events(owner, 'process_cancel_batch', [sv.u64(id), sv.u32(0), sv.op()], { simulate: true }),
      94,
      'an empty refund batch is refused (InvalidBatchSize)',
    );
    await must(events(stranger, 'process_cancel_batch', [sv.u64(id), sv.u32(15), sv.op()]), 'anyone can crank');
    await must(events(stranger, 'finalize_cancel', [sv.u64(id), sv.op()]), 'anyone can finalize');
    const remaining = bgt(25);
    const total = bgt(35);
    eq((await balance(b1)) - before[0], (bgt(20) * remaining) / total, 'partner 1 refunded pro rata');
    eq((await balance(b2)) - before[1], (bgt(15) * remaining) / total, 'partner 2 refunded pro rata');
    eq((await balance(bo)) - before[2], 0n, 'owner gets nothing when partners are short');
  }

  scenario = 'frozen partner';
  if (wanted(scenario)) {
    const [, , p3, p4] = partners;
    const id = await createGrant(owner, { budget: bgt(20), milestones: 1, floors: [[1, bgt(20)]] });
    await contribute(id, p3, bgt(10));
    await contribute(id, p4, bgt(10));
    await setFrozen(p4, true);
    const before = await Promise.all([p3, p4, owner].map((k) => balance(k.publicKey())));
    await cancel(owner, id);
    eq((await balance(p3.publicKey())) - before[0], bgt(10), 'other partners are refunded');
    eq((await balance(owner.publicKey())) - before[2], bgt(20), 'owner residual paid');
    const held = await events(issuer, 'get_unclaimed_refund', [sv.u64(id), sv.addr(p4.publicKey())], {
      simulate: true,
    });
    eq(held.ok ? BigInt(held.value as bigint) : -1n, bgt(10), 'frozen partner refund held');
    await refused(
      events(p4, 'claim_refund', [sv.u64(id), sv.addr(p4.publicKey()), sv.op()]),
      'any',
      'a frozen account cannot take its refund yet',
    );
    await setFrozen(p4, false);
    await must(
      events(p4, 'claim_refund', [sv.u64(id), sv.addr(p4.publicKey()), sv.op()]),
      'refund claimed after unfreezing',
    );
    eq((await balance(p4.publicKey())) - before[1], bgt(10), 'frozen partner made whole');
    await refused(
      events(p4, 'claim_refund', [sv.u64(id), sv.addr(p4.publicKey()), sv.op()], { simulate: true }),
      95,
      'a second claim is refused (NoRefundOwed)',
    );
  }

  scenario = 'refusals';
  if (wanted(scenario)) {
    const id = await createGrant(owner, { budget: bgt(100), milestones: 4, floors: [[1, 1n]] });
    const sel = (awards: Award[]) =>
      events(owner, 'select_winners', [sv.u64(id), winners(awards), sv.op()], { simulate: true });
    const x = r(8).publicKey();
    await refused(
      sel([
        { to: x, position: 1, amount: bgt(10) },
        { to: x, position: 2, amount: bgt(10) },
      ]),
      92,
      'one recipient, two awards (DuplicateRecipient)',
    );
    await refused(sel([{ to: x, position: 1, amount: 3n }]), 34, 'award smaller than its milestones');
    await refused(
      sel([{ to: x, position: 1, amount: bgt(10), bump: 101 }]),
      93,
      'reputation bump over the cap',
    );
    await refused(
      sel(people.slice(0, CAPACITY + 1).map((k, i) => ({ to: k.publicKey(), position: i + 1, amount: 4n }))),
      51,
      `more than ${CAPACITY} recipients`,
    );
    await refused(sel([{ to: x, position: 1, amount: bgt(101) }]), 56, 'awards beyond the escrow');
    await must(
      events(owner, 'select_winners', [sv.u64(id), winners([{ to: x, position: 1, amount: bgt(100) }]), sv.op()]),
      'select',
    );
    await refused(sel([{ to: r(9).publicKey(), position: 2, amount: 4n }]), 90, 'a second selection');
    const claim = (to: string, m: number, op = sv.op(), simulate = true) =>
      events(owner, 'claim_milestone', [sv.u64(id), sv.addr(to), sv.u32(m), sv.u32(10), op], { simulate });
    await refused(claim(x, 4), 55, 'milestone out of range');
    await refused(claim(r(9).publicKey(), 0), 50, 'someone never selected');
    const op = sv.op();
    await release(owner, id, r(8), 0, bgt(25));
    await refused(claim(x, 0), 54, 'the same milestone twice');
    await must(claim(x, 1, op, false), 'release with an op id');
    await refused(claim(x, 2, op), 60, 'the op id replayed');
    await refused(
      events(owner, 'claim_milestone', [sv.u64(id), sv.addr(x), sv.u32(2), sv.u32(101), sv.op()], { simulate: true }),
      93,
      'reputation bump over the cap on release',
    );
    await refused(
      events(owner, 'claim_prize', [sv.u64(id), sv.u32(1), sv.op()], { simulate: true }),
      33,
      'claim_prize is not a grant path',
    );
    await refused(
      events(owner, 'start_cancel', [sv.u64(id), sv.op()], { simulate: true }),
      96,
      'cancel refused while owed',
    );
    await forfeit(owner, id, r(8), 2);
    await forfeit(owner, id, r(8), 3);
    await cancel(owner, id);
  }

  scenario = 'signatures';
  if (wanted(scenario)) {
    const id = await createGrant(owner, { budget: bgt(10), milestones: 1, floors: [[1, bgt(10)]] });
    const x = r(10);
    await refused(
      events(stranger, 'select_winners', [
        sv.u64(id),
        winners([{ to: stranger.publicKey(), position: 1, amount: bgt(10) }]),
        sv.op(),
      ]),
      'auth',
      'a stranger cannot select',
    );
    await must(
      events(owner, 'select_winners', [sv.u64(id), winners([{ to: x.publicKey(), position: 1, amount: bgt(10) }]), sv.op()]),
      'owner selects',
    );
    await refused(
      events(x, 'claim_milestone', [sv.u64(id), sv.addr(x.publicKey()), sv.u32(0), sv.u32(10), sv.op()]),
      'auth',
      'a recipient cannot release their own milestone',
    );
    eq(await balance(x.publicKey()), 0n, 'nothing moved');
    await release(owner, id, x, 0, bgt(10));
  }

  scenario = 'manager handover';
  if (wanted(scenario)) {
    const id = await createGrant(ownerB, {
      budget: bgt(20),
      milestones: 2,
      floors: [[1, bgt(20)]],
      manager: manager.publicKey(),
    });
    await must(events(manager, 'accept_manager', [sv.u64(id)]), 'manager accepts');
    const m = await events(issuer, 'get_manager', [sv.u64(id)], { simulate: true });
    eq(m.ok ? m.value : '', manager.publicKey(), 'manager recorded');
    const awards = winners([{ to: r(11).publicKey(), position: 1, amount: bgt(20) }]);
    await refused(events(ownerB, 'select_winners', [sv.u64(id), awards, sv.op()]), 'auth', 'the owner no longer selects');
    await must(events(manager, 'select_winners', [sv.u64(id), awards, sv.op()]), 'the manager selects');
    await refused(
      events(ownerB, 'reclaim_management', [sv.u64(id)], { simulate: true }),
      90,
      'no reclaim after selection',
    );
    await refused(
      events(manager, 'claim_milestone', [sv.u64(id), sv.addr(r(11).publicKey()), sv.u32(0), sv.u32(10), sv.op()]),
      'auth',
      'the manager cannot release',
    );
    await release(ownerB, id, r(11), 0, bgt(10));
    await release(ownerB, id, r(11), 1, bgt(10));

    const other = await createGrant(ownerB, {
      budget: bgt(20),
      milestones: 1,
      floors: [[1, bgt(20)]],
      manager: managerB.publicKey(),
    });
    await must(events(managerB, 'accept_manager', [sv.u64(other)]), 'second manager accepts');
    await must(events(ownerB, 'reclaim_management', [sv.u64(other)]), 'owner takes management back');
    const back = await events(issuer, 'get_manager', [sv.u64(other)], { simulate: true });
    eq(back.ok ? back.value : '', ownerB.publicKey(), 'the owner manages again');
    await must(
      events(ownerB, 'select_winners', [
        sv.u64(other),
        winners([{ to: r(11).publicKey(), position: 1, amount: bgt(20) }]),
        sv.op(),
      ]),
      'owner selects after reclaiming',
    );
    await release(ownerB, other, r(11), 0, bgt(20));
  }

  scenario = `${CAPACITY} recipients`;
  if (wanted(scenario)) {
    const award = bgt(2);
    const id = await createGrant(owner, {
      budget: award * BigInt(CAPACITY),
      milestones: 2,
      floors: [[1, award]],
    });
    const crowd = people.slice(12, 12 + CAPACITY);
    await must(
      events(owner, 'select_winners', [
        sv.u64(id),
        winners(crowd.map((k, i) => ({ to: k.publicKey(), position: i + 1, amount: award }))),
        sv.op(),
      ]),
      `select ${CAPACITY} recipients in one transaction`,
    );
    for (const m of [0, 1]) {
      for (const k of crowd) await release(owner, id, k, m, bgt(1));
    }
    eq(await status(id), 'Completed', 'completed');
    const page = await events(issuer, 'get_winners_page', [sv.u64(id), sv.u32(0), sv.u32(90)], { simulate: true });
    eq(page.ok ? (page.value as unknown[]).length : -1, 90, 'a full page of winners reads in one call');
    const rest = await events(issuer, 'get_winners_page', [sv.u64(id), sv.u32(90), sv.u32(90)], { simulate: true });
    eq(rest.ok ? (rest.value as unknown[]).length : -1, CAPACITY * 3 - 90, 'the rest on the next page');
  }

  scenario = 'crowdfunding release validator';
  if (wanted(scenario)) {
    const validator = Keypair.fromSecret(secretOf(VALIDATOR_IDENTITY));
    const appointed = await events(issuer, 'get_release_validator', [], { simulate: true });
    eq(appointed.ok ? appointed.value : null, validator.publicKey(), 'the validator is appointed on chain');

    const [creator] = await provision(1, 0n, '20');
    const [backer] = partners;
    const id = await createGrant(creator, {
      budget: bgt(100),
      milestones: 2,
      floors: [[1, bgt(100)]],
      pillar: 'Crowdfunding',
    });
    const backerBefore = await balance(backer.publicKey());
    await must(
      events(backer, 'add_funds', [sv.u64(id), sv.addr(backer.publicKey()), sv.i128(bgt(60)), sv.op()]),
      'a backer pledges 60',
    );
    eq(backerBefore - (await balance(backer.publicKey())), bgt(60), 'backers pay no fee');

    const release = (source: Keypair, signers: Keypair[], m: number) =>
      events(
        source,
        'claim_milestone',
        [sv.u64(id), sv.addr(creator.publicKey()), sv.u32(m), sv.u32(10), sv.op()],
        { signers },
      );
    await refused(release(creator, [], 0), 'auth', 'the creator alone cannot release');

    // The backend's shape: the validator is the source and signs the envelope,
    // the creator's auth entry is signed with their own key.
    for (const m of [0, 1]) {
      const creatorBefore = await balance(creator.publicKey());
      const feeBefore = await balance(feeAccount);
      await must(release(validator, [creator], m), `release ${m} co-signed by the validator`);
      const share = bgt(30);
      eq((await balance(creator.publicKey())) - creatorBefore, share - fee(share), 'creator paid net of fee');
      eq((await balance(feeAccount)) - feeBefore, fee(share), 'fee withheld at release');
      feesExpected += fee(share);
    }
    eq(await status(id), 'Completed', 'campaign completes on its last release');
  }

  scenario = 'fees';
  eq((await balance(feeAccount)) - feeStart, feesExpected, 'fee account received exactly the fees charged');
}

// ---------------------------------------------------------------- report

function report(error?: unknown) {
  const here = dirname(fileURLToPath(import.meta.url));
  const out = join(here, '..', '..', 'docs', 'audit');
  mkdirSync(out, { recursive: true });
  const failed = checks.filter((c) => !c.ok);
  const scenarios = [...new Set(checks.map((c) => c.scenario))];
  const fees = txs.filter((t) => t.ok).map((t) => Number(t.fee));
  const lines = [
    '# Testnet grant run',
    '',
    `Run ${new Date().toISOString()} against \`${EVENTS}\` with the BGT test asset \`${TOKEN}\`.`,
    `${checks.length - failed.length} of ${checks.length} checks passed across ${txs.length} transactions` +
      (error ? `; the run stopped early: ${String(error)}` : '.'),
    fees.length ? `Highest fee paid by one transaction: ${Math.max(...fees)} stroops.` : '',
    '',
    ...scenarios.flatMap((s) => [
      `## ${s}`,
      '',
      ...checks
        .filter((c) => c.scenario === s)
        .map((c) => `- ${c.ok ? 'pass' : '**FAIL**'}: ${c.label}${c.detail ? ` (${c.detail})` : ''}`),
      '',
      ...txs
        .filter((t) => t.scenario === s)
        .map(
          (t) =>
            `  - [${t.step}](https://stellar.expert/explorer/testnet/tx/${t.hash})${t.ok ? '' : ' (failed, as expected or not: see checks)'}`,
        ),
      '',
    ]),
  ];
  writeFileSync(join(out, 'testnet-grant-run.md'), lines.join('\n'));
  console.log(`\n${checks.length - failed.length}/${checks.length} checks passed; report in docs/audit/testnet-grant-run.md`);
  if (failed.length || error) process.exitCode = 1;
}

main().then(
  () => report(),
  (error) => {
    console.error(error);
    report(error);
  },
);
