#![cfg(test)]

//! Replays the mainnet upgrade from the deployed builds (events 1.7.0, profile
//! 1.2.0) to the release build of this tree, against contract storage captured
//! from a mainnet ledger. Everything readable before the upgrade must read the
//! same after it, the records still in the pre-1.7.0 layout must convert, and
//! the upgraded pair must run a grant end to end.

use std::fmt::Debug;

use boundless_events::types::{
    CreateEventParams, DataKey, EventStatus, Pillar, ReleaseKind, WinnerSpec,
};
use boundless_events::EventsContractClient;
use boundless_profile::ProfileContractClient;
use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, vec, Address, BytesN, Env, Map, String,
};

mod deployed_events {
    soroban_sdk::contractimport!(file = "fixtures/mainnet-events-1.7.0.wasm");
}

mod deployed_profile {
    soroban_sdk::contractimport!(file = "fixtures/mainnet-profile-1.2.0.wasm");
}

const SNAPSHOT: &str = "mainnet-state.json";
const EVENTS_ID: &str = "CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ";
const PROFILE_ID: &str = "CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC";
const ADMIN: &str = "GCVK72I6TVJVDTTY4UKU6MQT4QJ2T2AAG3NULNEUDM46L3UOQYDSO4O2";
const FEE_ACCOUNT: &str = "GADSIP2HPINWTMEUNMVYA5NJRL372OV52HDEJTRTLDZXMHADIPQNYH55";
const MAINNET_USDC: &str = "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75";
const PAGE: u32 = 90;

/// The release build this tree produces. The replay upgrades into the exact
/// artifact that would be proposed on chain, so it is read from the build
/// output rather than compiled into the test.
fn release_wasm(name: &str) -> std::vec::Vec<u8> {
    let dir = std::env::var("BOUNDLESS_WASM_DIR").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/wasm32v1-none/release"
        )
        .to_string()
    });
    let path = format!("{dir}/{name}.wasm");
    std::fs::read(&path).unwrap_or_else(|_| {
        panic!("{path} is missing; run scripts/test-storage-compat.sh, which builds it")
    })
}

/// Loads the captured ledger. The test host runs protocol 28, the newest it
/// supports, so a capture from a later protocol is replayed at 28; nothing the
/// contracts store depends on the protocol version.
fn mainnet_env() -> Env {
    let fixture = format!("{}/fixtures/{SNAPSHOT}", env!("CARGO_MANIFEST_DIR"));
    let captured = std::fs::read_to_string(&fixture).expect("mainnet snapshot fixture");
    let key = "\"protocol_version\": ";
    let at = captured.find(key).expect("snapshot records its protocol") + key.len();
    let digits = captured[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .count();
    let replayed = format!("{}28{}", &captured[..at], &captured[at + digits..]);
    let path = std::env::temp_dir().join("boundless-mainnet-state-p28.json");
    std::fs::write(&path, replayed).expect("write replayed snapshot");
    let env = Env::from_ledger_snapshot_file(path);
    env.mock_all_auths_allowing_non_root_auth();
    env
}

fn addr(env: &Env, strkey: &str) -> Address {
    Address::from_str(env, strkey)
}

fn debug<T: Debug>(value: T) -> std::string::String {
    format!("{value:?}")
}

/// recipient, position, amount, milestone, paid_at
type WinnerRow = (Address, u32, i128, Option<u32>, Option<u64>);

/// One event as either build reports it, reduced to values both can express.
#[derive(Debug, PartialEq)]
struct EventView {
    pillar: std::string::String,
    owner: Address,
    token: Address,
    total_budget: i128,
    remaining_escrow: i128,
    release_kind: std::string::String,
    status: std::string::String,
    title: String,
    created_at: u64,
    prize_floors: Map<u32, i128>,
    winners: std::vec::Vec<WinnerRow>,
}

fn event_ids(env: &Env, events_id: &Address, first: u64) -> std::ops::Range<u64> {
    let next: u64 = env.as_contract(events_id, || {
        env.storage()
            .instance()
            .get(&DataKey::NextEventId)
            .expect("deployment has created events")
    });
    first..next
}

fn view_before(events: &deployed_events::Client, id: u64) -> Option<EventView> {
    let event = events.try_get_event(&id).ok()?.ok()?;
    let count = events.get_winner_count(&id);
    let mut winners = std::vec::Vec::new();
    let mut start = 0;
    while start < count {
        for w in events.get_winners_page(&id, &start, &PAGE).iter() {
            winners.push((w.recipient, w.position, w.amount, w.milestone, w.paid_at));
        }
        start += PAGE;
    }
    Some(EventView {
        pillar: debug(event.pillar),
        owner: event.owner,
        token: event.token,
        total_budget: event.total_budget,
        remaining_escrow: event.remaining_escrow,
        release_kind: debug(event.release_kind),
        status: debug(event.status),
        title: event.title,
        created_at: event.created_at,
        prize_floors: event.prize_floors,
        winners,
    })
}

fn view_after(events: &EventsContractClient, id: u64) -> EventView {
    let event = events.get_event(&id);
    let count = events.get_winner_count(&id);
    let mut winners = std::vec::Vec::new();
    let mut start = 0;
    while start < count {
        for w in events.get_winners_page(&id, &start, &PAGE).iter() {
            winners.push((w.recipient, w.position, w.amount, w.milestone, w.paid_at));
        }
        start += PAGE;
    }
    EventView {
        pillar: debug(event.pillar),
        owner: event.owner,
        token: event.token,
        total_budget: event.total_budget,
        remaining_escrow: event.remaining_escrow,
        release_kind: debug(event.release_kind),
        status: debug(event.status),
        title: event.title,
        created_at: event.created_at,
        prize_floors: event.prize_floors,
        winners,
    }
}

#[test]
#[ignore = "needs the release wasm; run scripts/test-storage-compat.sh"]
fn mainnet_upgrades_from_the_deployed_builds_to_this_tree() {
    let env = mainnet_env();
    let events_id = addr(&env, EVENTS_ID);
    let profile_id = addr(&env, PROFILE_ID);

    // Uploading the deployed code is what makes the captured instances run;
    // its hash has to equal the one each instance points at.
    env.deployer().upload_contract_wasm(deployed_events::WASM);
    env.deployer().upload_contract_wasm(deployed_profile::WASM);
    let events_v1 = deployed_events::Client::new(&env, &events_id);
    let profile_v1 = deployed_profile::Client::new(&env, &profile_id);
    assert_eq!(events_v1.version(), String::from_str(&env, "1.7.0"));
    assert_eq!(profile_v1.version(), String::from_str(&env, "1.2.0"));

    let first = events_v1.id_base() + 1;
    let ids = event_ids(&env, &events_id, first);
    let mut before = std::vec::Vec::new();
    let mut old_layout = std::vec::Vec::new();
    for id in ids.clone() {
        match view_before(&events_v1, id) {
            Some(view) => before.push((id, view)),
            None => old_layout.push(id),
        }
    }
    assert_eq!(
        old_layout,
        std::vec![first, first + 1],
        "the deployed build cannot read exactly the two pre-1.7.0 records"
    );

    assert!(
        before.iter().any(|(_, view)| !view.winners.is_empty()),
        "the capture must hold paid winners for the comparison to mean anything"
    );

    let recipients: std::vec::Vec<Address> = before
        .iter()
        .flat_map(|(_, view)| view.winners.iter().map(|w| w.0.clone()))
        .collect();
    let reputations_before: std::vec::Vec<u64> = recipients
        .iter()
        .map(|r| profile_v1.get_profile(r).map_or(0, |p| p.reputation))
        .collect();

    // The runbook order: profile first, left running; then events, paused
    // from propose to unpause. The deployed builds have no upgrade timelock.
    let profile_hash = env
        .deployer()
        .upload_contract_wasm(release_wasm("boundless_profile").as_slice());
    profile_v1.propose_upgrade(&profile_hash, &String::from_str(&env, "1.2.1"));
    profile_v1.apply_upgrade();
    let profile = ProfileContractClient::new(&env, &profile_id);
    profile.migrate();
    assert!(!profile.is_paused());

    events_v1.pause();
    let events_hash = env
        .deployer()
        .upload_contract_wasm(release_wasm("boundless_events").as_slice());
    events_v1.propose_upgrade(&events_hash, &String::from_str(&env, "2.0.0"));
    events_v1.apply_upgrade();
    let events = EventsContractClient::new(&env, &events_id);
    assert!(events.is_paused());
    assert_eq!(
        events.try_migrate().unwrap_err().unwrap(),
        boundless_events::errors::Error::MigrationIncomplete
    );
    assert_eq!(events.migrate_events(&8), 0);
    events.migrate();
    events.unpause();

    assert_eq!(events.version(), String::from_str(&env, "2.0.0"));
    assert_eq!(profile.version(), String::from_str(&env, "1.2.1"));
    assert_eq!(
        events.get_migrated_to_version(),
        Some(String::from_str(&env, "2.0.0"))
    );
    assert_eq!(
        profile.get_migrated_to_version(),
        Some(String::from_str(&env, "1.2.1"))
    );
    assert_eq!(events.get_admin(), addr(&env, ADMIN));
    assert_eq!(profile.get_admin(), addr(&env, ADMIN));
    assert_eq!(events.get_fee_account(), addr(&env, FEE_ACCOUNT));
    assert_eq!(events.get_fee_bps(), 250);
    assert_eq!(events.get_profile_contract(), profile_id);
    assert_eq!(profile.get_events_contract(), Some(events_id.clone()));
    assert_eq!(events.id_base() + 1, first);
    assert!(events.is_supported_token(&addr(&env, MAINNET_USDC)));
    assert_eq!(events.get_release_validator(), None);

    for (id, view) in &before {
        assert_eq!(&view_after(&events, *id), view, "event {id} changed");
    }

    // Percentages were always taken of the budget, so each converted floor is
    // exactly what the old record would have paid that position.
    for id in old_layout {
        let event = events.get_event(&id);
        let winners = view_after(&events, id).winners;
        assert!(!event.prize_floors.is_empty());
        let floors_total: i128 = event.prize_floors.values().iter().sum();
        assert!(floors_total <= event.total_budget);
        for (recipient, position, amount, milestone, paid_at) in winners {
            assert!(milestone.is_none() && paid_at.is_some(), "{recipient:?}");
            assert_eq!(event.prize_floors.get(position), Some(amount));
        }
    }

    for (recipient, reputation) in recipients.iter().zip(reputations_before) {
        let after = profile.get_profile(recipient).map_or(0, |p| p.reputation);
        assert_eq!(after, reputation);
    }

    // A second pass finds nothing to do, and the stamp cannot be replayed.
    assert_eq!(events.migrate_events(&8), 0);
    assert!(events.try_migrate().is_err());

    run_a_grant(&env, &events, &profile, ids.end);
}

/// A grant through the upgraded pair, on a fresh token: selection, releases, a
/// forfeit, and the close that returns the forfeited share.
fn run_a_grant(
    env: &Env,
    events: &EventsContractClient,
    profile: &ProfileContractClient,
    expected_id: u64,
) {
    // The mainnet fee account has no ledger entry in the capture, so a test
    // address takes the fee.
    let fee_account = Address::generate(env);
    events.set_fee_account(&fee_account);

    let issuer = Address::generate(env);
    let token_id = env.register_stellar_asset_contract_v2(issuer).address();
    let token = token::Client::new(env, &token_id);
    events.register_supported_token(&token_id);

    let budget: i128 = 900_0000000;
    let fee = budget * 250 / 10_000;
    let owner = Address::generate(env);
    token::StellarAssetClient::new(env, &token_id).mint(&owner, &(budget + fee));

    let mut floors = Map::new(env);
    floors.set(1, 600_0000000_i128);
    floors.set(2, 300_0000000_i128);
    let id = events.create_event(
        &CreateEventParams {
            pillar: Pillar::Grant,
            owner: owner.clone(),
            token: token_id.clone(),
            total_budget: budget,
            release_kind: ReleaseKind::Multi(3),
            content_uri: String::from_str(env, "https://boundlessfi.xyz/grants/replay"),
            title: String::from_str(env, "Upgrade replay"),
            deadline: None,
            prize_floors: floors,
            fee_bps_override: None,
            manager: None,
        },
        &BytesN::random(env),
    );
    assert_eq!(
        id, expected_id,
        "event ids continue from the deployed counter"
    );
    assert_eq!(token.balance(&fee_account), fee);

    let (a, b) = (Address::generate(env), Address::generate(env));
    events.select_winners(
        &id,
        &vec![
            env,
            WinnerSpec {
                recipient: a.clone(),
                position: 1,
                amount: 600_0000000,
                reputation_bump: 10,
            },
            WinnerSpec {
                recipient: b.clone(),
                position: 2,
                amount: 300_0000000,
                reputation_bump: 10,
            },
        ],
        &BytesN::random(env),
    );

    events.claim_milestone(&id, &a, &0, &10, &BytesN::random(env));
    events.forfeit_milestone(&id, &a, &1, &BytesN::random(env));
    events.claim_milestone(&id, &a, &2, &10, &BytesN::random(env));
    for milestone in 0..3 {
        events.claim_milestone(&id, &b, &milestone, &10, &BytesN::random(env));
    }
    assert_eq!(token.balance(&a), 400_0000000);
    assert_eq!(token.balance(&b), 300_0000000);
    assert_eq!(profile.get_earnings(&a, &token_id), 400_0000000);
    assert_eq!(profile.get_profile(&b).unwrap().reputation, 30);

    events.start_cancel(&id, &BytesN::random(env));
    if events.get_event(&id).status == EventStatus::Cancelling {
        while events.process_cancel_batch(&id, &15, &BytesN::random(env)) > 0 {}
        events.finalize_cancel(&id, &BytesN::random(env));
    }
    assert_eq!(events.get_event(&id).status, EventStatus::Cancelled);
    assert_eq!(token.balance(&owner), 200_0000000);
    assert_eq!(token.balance(&events.address), 0);
}
