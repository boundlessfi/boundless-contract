#![cfg(test)]

//! Grant lifecycle scenarios, end to end against a real profile contract and
//! a Stellar asset contract. Every helper that moves money asserts the exact
//! token deltas, and `books` checks the escrow invariants after a step.

extern crate std;

mod auth;
mod cancel;
mod claims;
mod contributions;
mod create;
mod forfeits;
mod hardening;
mod lifecycle;
mod limits;
mod pause;
mod randomized;
mod refusing_token;
mod selection;
mod split;

use soroban_sdk::{
    testutils::{
        Address as _, BytesN as _, EnvTestConfig, Events as _, IssuerFlags, MockAuth,
        MockAuthInvoke,
    },
    token, xdr, Address, BytesN, Env, Event, IntoVal, Map, String, Val, Vec,
};

use crate::errors::Error;
use crate::storage;
use crate::types::{CreateEventParams, EventRecord, EventStatus, Pillar, ReleaseKind, WinnerSpec};
use crate::{EventsContract, EventsContractClient};
use boundless_profile::{ProfileContract, ProfileContractClient};

pub const FEE_BPS: u32 = 250;
pub const USDC: i128 = 10_000_000;

pub fn usdc(units: i128) -> i128 {
    units * USDC
}

pub fn fee_on(amount: i128, bps: u32) -> i128 {
    amount * bps as i128 / 10_000
}

pub struct G<'a> {
    pub env: Env,
    pub events: EventsContractClient<'a>,
    pub events_id: Address,
    pub profile: ProfileContractClient<'a>,
    pub admin: Address,
    pub fee_account: Address,
    pub token: token::Client<'a>,
    pub token_admin: token::StellarAssetClient<'a>,
    pub owner: Address,
    pub ids: std::cell::RefCell<std::vec::Vec<u64>>,
    pub log: std::cell::RefCell<std::vec::Vec<xdr::ContractEvent>>,
}

pub fn setup<'a>() -> G<'a> {
    let env = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env.mock_all_auths_allowing_non_root_auth();

    let profile_admin = Address::generate(&env);
    let profile_id = env.register(ProfileContract, (profile_admin,));
    let profile = ProfileContractClient::new(&env, &profile_id);

    let admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let events_id = env.register(
        EventsContract,
        (
            admin.clone(),
            fee_account.clone(),
            FEE_BPS,
            profile_id.clone(),
        ),
    );
    let events = EventsContractClient::new(&env, &events_id);
    profile.set_events_contract(&events_id);

    let sac = env.register_stellar_asset_contract_v2(Address::generate(&env));
    sac.issuer().set_flag(IssuerFlags::RevocableFlag);
    let token = token::Client::new(&env, &sac.address());
    let token_admin = token::StellarAssetClient::new(&env, &sac.address());
    events.register_supported_token(&sac.address());

    let owner = Address::generate(&env);

    G {
        env,
        events,
        events_id,
        profile,
        admin,
        fee_account,
        token,
        token_admin,
        owner,
        ids: std::cell::RefCell::new(std::vec::Vec::new()),
        log: std::cell::RefCell::new(std::vec::Vec::new()),
    }
}

pub struct Spec {
    pub budget: i128,
    pub milestones: u32,
    pub split: Option<std::vec::Vec<u32>>,
    pub floors: std::vec::Vec<(u32, i128)>,
    pub fee_bps_override: Option<u32>,
    pub manager: Option<Address>,
}

impl Spec {
    pub fn new(budget: i128, milestones: u32) -> Self {
        Spec {
            budget,
            milestones,
            split: None,
            floors: std::vec![(1, budget)],
            fee_bps_override: None,
            manager: None,
        }
    }

    pub fn floors(mut self, floors: &[(u32, i128)]) -> Self {
        self.floors = floors.to_vec();
        self
    }

    /// Pays milestones unequal shares, in basis points.
    pub fn split(mut self, shares: &[u32]) -> Self {
        self.milestones = shares.len() as u32;
        self.split = Some(shares.to_vec());
        self
    }

    pub fn fee_override(mut self, bps: u32) -> Self {
        self.fee_bps_override = Some(bps);
        self
    }

    pub fn manager(mut self, manager: &Address) -> Self {
        self.manager = Some(manager.clone());
        self
    }
}

impl<'a> G<'a> {
    pub fn op(&self) -> BytesN<32> {
        BytesN::random(&self.env)
    }

    pub fn someone(&self) -> Address {
        Address::generate(&self.env)
    }

    pub fn balance(&self, who: &Address) -> i128 {
        self.token.balance(who)
    }

    pub fn escrow_balance(&self) -> i128 {
        self.token.balance(&self.events_id)
    }

    pub fn params(&self, spec: &Spec) -> CreateEventParams {
        let mut floors = Map::new(&self.env);
        for (position, floor) in spec.floors.iter() {
            floors.set(*position, *floor);
        }
        CreateEventParams {
            pillar: Pillar::Grant,
            owner: self.owner.clone(),
            token: self.token.address.clone(),
            total_budget: spec.budget,
            release_kind: match &spec.split {
                Some(shares) => {
                    let mut v = Vec::new(&self.env);
                    for share in shares.iter() {
                        v.push_back(*share);
                    }
                    ReleaseKind::Split(v)
                }
                None => ReleaseKind::Multi(spec.milestones),
            },
            content_uri: String::from_str(&self.env, "https://api.boundless.fi/grants/x/content"),
            title: String::from_str(&self.env, "Community grants"),
            deadline: Some(self.env.ledger().timestamp() + 30 * 86_400),
            prize_floors: floors,
            fee_bps_override: spec.fee_bps_override,
            manager: spec.manager.clone(),
        }
    }

    /// Mints exactly budget + fee to the owner, publishes, and checks that the
    /// owner is drained, escrow holds the budget and the fee account the fee.
    pub fn create(&self, spec: Spec) -> u64 {
        let bps = spec.fee_bps_override.unwrap_or(self.events.get_fee_bps());
        let fee = fee_on(spec.budget, bps);
        self.token_admin.mint(&self.owner, &(spec.budget + fee));
        let owner_before = self.balance(&self.owner);
        let fee_before = self.balance(&self.fee_account);
        let escrow_before = self.escrow_balance();

        let id = self.events.create_event(&self.params(&spec), &self.op());
        self.capture();

        assert_eq!(owner_before - self.balance(&self.owner), spec.budget + fee);
        assert_eq!(self.balance(&self.fee_account) - fee_before, fee);
        assert_eq!(self.escrow_balance() - escrow_before, spec.budget);
        let event = self.events.get_event(&id);
        assert_eq!(event.remaining_escrow, spec.budget);
        assert_eq!(event.status, EventStatus::Active);
        self.ids.borrow_mut().push(id);
        self.books();
        id
    }

    pub fn select(&self, id: u64, awards: &[(Address, u32, i128)]) {
        let escrow_before = self.escrow_balance();
        let owed_before = self.owed(id);
        self.events
            .select_winners(&id, &self.specs(awards), &self.op());
        self.capture();
        assert_eq!(
            self.escrow_balance(),
            escrow_before,
            "selection moves no tokens"
        );
        let total: i128 = awards.iter().map(|(_, _, a)| *a).sum();
        assert_eq!(self.owed(id), owed_before + total);
        self.books();
    }

    pub fn specs(&self, awards: &[(Address, u32, i128)]) -> Vec<WinnerSpec> {
        let mut specs = Vec::new(&self.env);
        for (recipient, position, amount) in awards.iter() {
            specs.push_back(WinnerSpec {
                recipient: recipient.clone(),
                position: *position,
                amount: *amount,
                reputation_bump: 0,
            });
        }
        specs
    }

    /// Releases one milestone and asserts the recipient receives exactly
    /// `expected`, with no fee and matching drops in escrow and owed.
    pub fn pay(&self, id: u64, recipient: &Address, milestone: u32, expected: i128) {
        let before = self.balance(recipient);
        let fee_before = self.balance(&self.fee_account);
        let event_before = self.events.get_event(&id);
        let owed_before = self.owed(id);

        self.events
            .claim_milestone(&id, recipient, &milestone, &5, &self.op());
        self.capture();

        assert_eq!(
            self.balance(recipient) - before,
            expected,
            "milestone payout"
        );
        assert_eq!(
            self.balance(&self.fee_account),
            fee_before,
            "grant payouts carry no fee"
        );
        let event = self.events.get_event(&id);
        assert_eq!(
            event_before.remaining_escrow - event.remaining_escrow,
            expected
        );
        assert_eq!(owed_before - self.owed(id), expected);
        assert!(self.milestone_claimed(id, recipient, milestone));
        self.books();
    }

    /// Forfeits one milestone: nothing moves, and its share stops being owed.
    pub fn forfeit(&self, id: u64, recipient: &Address, milestone: u32, expected: i128) {
        let before = self.balance(recipient);
        let escrow_before = self.escrow_balance();
        let remaining_before = self.events.get_event(&id).remaining_escrow;
        let owed_before = self.owed(id);

        self.events
            .forfeit_milestone(&id, recipient, &milestone, &self.op());
        self.capture();

        assert_eq!(self.balance(recipient), before, "a forfeit pays nothing");
        assert_eq!(self.escrow_balance(), escrow_before);
        assert_eq!(
            self.events.get_event(&id).remaining_escrow,
            remaining_before
        );
        assert_eq!(owed_before - self.owed(id), expected, "forfeited share");
        assert!(self.milestone_claimed(id, recipient, milestone));
        self.books();
    }

    /// Partner top-up: mints amount + fee and checks the fee lands on top.
    pub fn contribute(&self, id: u64, from: &Address, amount: i128) {
        let bps = self
            .events
            .get_event(&id)
            .fee_bps_override
            .unwrap_or(self.events.get_fee_bps());
        let fee = fee_on(amount, bps);
        self.token_admin.mint(from, &(amount + fee));
        let from_before = self.balance(from);
        let fee_before = self.balance(&self.fee_account);
        let remaining_before = self.events.get_event(&id).remaining_escrow;

        self.events.add_funds(&id, from, &amount, &self.op());
        self.capture();

        assert_eq!(
            from_before - self.balance(from),
            amount + fee,
            "fee is on top"
        );
        assert_eq!(self.balance(&self.fee_account) - fee_before, fee);
        assert_eq!(
            self.events.get_event(&id).remaining_escrow - remaining_before,
            amount
        );
        self.books();
    }

    pub fn cancel(&self, id: u64) {
        self.events.start_cancel(&id, &self.op());
        self.capture();
        if self.status(id) == EventStatus::Cancelling {
            self.finish_cancel(id);
        }
        let event = self.events.get_event(&id);
        assert_eq!(event.status, EventStatus::Cancelled);
        assert_eq!(event.remaining_escrow, 0);
        assert_eq!(self.owed(id), 0);
        self.books();
    }

    /// Cranks and finalizes a cancel that `start_cancel` left in Cancelling.
    pub fn finish_cancel(&self, id: u64) {
        loop {
            let left = self.events.process_cancel_batch(
                &id,
                &crate::event_ops::MAX_REFUNDS_PER_BATCH,
                &self.op(),
            );
            self.capture();
            if left == 0 {
                break;
            }
        }
        self.events.finalize_cancel(&id, &self.op());
        self.capture();
        assert_eq!(self.status(id), EventStatus::Cancelled);
        self.books();
    }

    pub fn event(&self, id: u64) -> EventRecord {
        self.events.get_event(&id)
    }

    pub fn status(&self, id: u64) -> EventStatus {
        self.events.get_event(&id).status
    }

    pub fn owed(&self, id: u64) -> i128 {
        self.env
            .as_contract(&self.events_id, || storage::owed_total(&self.env, id))
    }

    pub fn milestone_claimed(&self, id: u64, recipient: &Address, milestone: u32) -> bool {
        self.env.as_contract(&self.events_id, || {
            storage::is_milestone_claimed(&self.env, id, recipient, milestone)
        })
    }

    /// Escrow invariants across every grant this context created: the contract
    /// holds at least what the records say it holds, nothing is owed beyond
    /// what remains, and a closed grant owes nothing.
    pub fn books(&self) {
        let mut accounted: i128 = 0;
        for id in self.ids.borrow().iter() {
            let event = self.events.get_event(id);
            let owed = self.owed(*id);
            assert!(event.remaining_escrow >= 0, "grant {id}: negative escrow");
            assert!(owed >= 0, "grant {id}: negative owed");
            assert!(
                owed <= event.remaining_escrow,
                "grant {id}: owes {owed} with only {} in escrow",
                event.remaining_escrow
            );
            if matches!(
                event.status,
                EventStatus::Completed | EventStatus::Cancelled
            ) {
                assert_eq!(
                    event.remaining_escrow, 0,
                    "grant {id}: closed with escrow left"
                );
                assert_eq!(owed, 0, "grant {id}: closed while still owing");
            }
            accounted += event.remaining_escrow;
        }
        assert!(
            self.escrow_balance() >= accounted,
            "contract holds {} but records account for {accounted}",
            self.escrow_balance()
        );
    }

    /// Only `who` signs the next call to `fn_name` with exactly `args`.
    pub fn sign_only(&self, who: &Address, fn_name: &str, args: Vec<Val>) {
        self.env.mock_auths(&[MockAuth {
            address: who,
            invoke: &MockAuthInvoke {
                contract: &self.events_id,
                fn_name,
                args,
                sub_invokes: &[],
            },
        }]);
    }

    pub fn restore_auth(&self) {
        self.env.mock_all_auths_allowing_non_root_auth();
    }

    /// Addresses that authorized the last top-level invocation.
    pub fn signers(&self) -> std::vec::Vec<Address> {
        self.env
            .auths()
            .into_iter()
            .map(|(address, _)| address)
            .collect()
    }

    /// Keeps the events of the call just made; `events().all()` only ever
    /// holds the last invocation's.
    pub fn capture(&self) {
        let all = self.env.events().all();
        let ours = all.filter_by_contract(&self.events_id);
        self.log.borrow_mut().extend(ours.events().iter().cloned());
    }

    /// Whether `event` was emitted by any captured call or the last one.
    pub fn emitted<E: Event>(&self, event: E) -> bool {
        self.capture();
        let wanted = event.to_xdr(&self.env, &self.events_id);
        self.log.borrow().contains(&wanted)
    }
}

pub fn args<T: IntoVal<Env, Vec<Val>>>(g: &G, value: T) -> Vec<Val> {
    value.into_val(&g.env)
}

/// Asserts a `try_` call failed with exactly this contract error.
#[track_caller]
pub fn fails_with<T: core::fmt::Debug, C: core::fmt::Debug, I: core::fmt::Debug>(
    result: Result<Result<T, C>, Result<Error, I>>,
    expected: Error,
) {
    match result {
        Err(Ok(actual)) => assert_eq!(actual, expected),
        other => panic!("expected {expected:?}, got {other:?}"),
    }
}

/// Asserts a `try_` call was rejected by the host's auth check.
#[track_caller]
pub fn fails_auth<T: core::fmt::Debug, C: core::fmt::Debug, I: core::fmt::Debug>(
    result: Result<Result<T, C>, Result<Error, I>>,
) {
    match result {
        Err(Err(_)) => {}
        other => panic!("expected an auth failure, got {other:?}"),
    }
}
