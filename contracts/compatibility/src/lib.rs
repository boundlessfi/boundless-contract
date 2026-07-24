#![cfg(test)]
#![allow(clippy::enum_variant_names)]

use std::cell::Cell;

use soroban_sdk::{
    contracttype,
    testutils::{Address as _, Ledger as _},
    token, vec, Address, BytesN, Env, Map, String,
};

mod old_events {
    soroban_sdk::contractimport!(file = "fixtures/mainnet-events-1.1.0-sdk23.wasm");
}

mod old_profile {
    soroban_sdk::contractimport!(file = "fixtures/mainnet-profile-1.1.0-sdk23.wasm");
}

mod pre27_events {
    soroban_sdk::contractimport!(file = "fixtures/events-1.3.0-sdk23.wasm");
}

mod new_events {
    soroban_sdk::contractimport!(file = "fixtures/events-1.5.0-sdk27.wasm");
}

mod new_profile {
    soroban_sdk::contractimport!(file = "fixtures/profile-1.2.0-sdk27.wasm");
}

const FEE_BPS: u32 = 250;
const BUDGET: i128 = 10_000_000_000;
const CONTRIBUTION: i128 = 100_000_000;
const TIMELOCK: u32 = 17_280;

fn next_op(env: &Env, counter: &Cell<u32>) -> BytesN<32> {
    let value = counter.get();
    counter.set(value + 1);
    let mut bytes = [0; 32];
    bytes[28..].copy_from_slice(&value.to_be_bytes());
    BytesN::from_array(env, &bytes)
}

#[contracttype]
enum LegacyProfileDataKey {
    DefaultBootstrapCredits,
    DeploymentSeq,
    OpSeen(BytesN<32>),
}

#[contracttype]
enum LegacyEventsDataKey {
    EventApplicantSlot(u64, Address),
    ContributorSlot(u64, Address),
    SupportedTokenSlot(Address),
}

fn distribution(env: &Env) -> Map<u32, u32> {
    let mut distribution = Map::new(env);
    distribution.set(1, 100);
    distribution
}

fn params(
    env: &Env,
    owner: &Address,
    token: &Address,
    title: &str,
) -> old_events::CreateEventParams {
    old_events::CreateEventParams {
        content_uri: String::from_str(env, "ipfs://event"),
        deadline: Some(env.ledger().timestamp() + 86_400),
        fee_bps_override: None,
        manager: None,
        owner: owner.clone(),
        pillar: old_events::Pillar::Hackathon,
        release_kind: old_events::ReleaseKind::Single,
        title: String::from_str(env, title),
        token: token.clone(),
        total_budget: BUDGET,
        winner_distribution: distribution(env),
    }
}

fn pre27_params(
    env: &Env,
    owner: &Address,
    token: &Address,
    title: &str,
) -> pre27_events::CreateEventParams {
    pre27_events::CreateEventParams {
        content_uri: String::from_str(env, "ipfs://event"),
        deadline: Some(env.ledger().timestamp() + 86_400),
        fee_bps_override: None,
        manager: None,
        owner: owner.clone(),
        pillar: pre27_events::Pillar::Hackathon,
        release_kind: pre27_events::ReleaseKind::Single,
        title: String::from_str(env, title),
        token: token.clone(),
        total_budget: BUDGET,
        winner_distribution: distribution(env),
    }
}

fn new_params(
    env: &Env,
    owner: &Address,
    token: &Address,
    title: &str,
) -> new_events::CreateEventParams {
    new_events::CreateEventParams {
        content_uri: String::from_str(env, "ipfs://event"),
        deadline: Some(env.ledger().timestamp() + 86_400),
        fee_bps_override: None,
        manager: None,
        owner: owner.clone(),
        pillar: new_events::Pillar::Hackathon,
        release_kind: new_events::ReleaseKind::Single,
        title: String::from_str(env, title),
        token: token.clone(),
        total_budget: BUDGET,
        winner_distribution: distribution(env),
    }
}

#[test]
fn sdk23_v110_rows_survive_h6_upgrade_to_sdk27() {
    let env = Env::default();
    let ops = Cell::new(1);
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number = 100;
        ledger.timestamp = 1_000_000;
        // Preserve legacy replay markers while the test advances through the timelock.
        ledger.min_temp_entry_ttl = TIMELOCK + 100;
    });

    let admin = Address::generate(&env);
    let fee_account = Address::generate(&env);

    let profile_id = env.register(old_profile::WASM, (&admin,));
    let profile_v1 = old_profile::Client::new(&env, &profile_id);

    let events_id = env.register(
        old_events::WASM,
        (&admin, &fee_account, &FEE_BPS, &profile_id),
    );
    let events_v1 = old_events::Client::new(&env, &events_id);
    profile_v1.set_events_contract(&events_id);

    let issuer = Address::generate(&env);
    let asset = env.register_stellar_asset_contract_v2(issuer);
    let token_id = asset.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&fee_account, &0);

    let owner = Address::generate(&env);
    let winner = Address::generate(&env);
    let grant_recipient = Address::generate(&env);
    let contributor = Address::generate(&env);
    let manager = Address::generate(&env);
    let next_events_admin = Address::generate(&env);
    let next_profile_admin = Address::generate(&env);
    let rotated_events_id = Address::generate(&env);
    let legacy_profile_user = Address::generate(&env);
    let legacy_profile_op = next_op(&env, &ops);
    token_admin.mint(&owner, &(BUDGET * 6));
    token_admin.mint(&contributor, &(CONTRIBUTION * 4));
    events_v1.register_supported_token(&token_id);
    let token = token::Client::new(&env, &token_id);
    profile_v1.bootstrap_self(&legacy_profile_user, &legacy_profile_op);

    let mut completed_params = params(&env, &owner, &token_id, "completed");
    completed_params.pillar = old_events::Pillar::Bounty;
    let legacy_event_create_op = next_op(&env, &ops);
    let completed_id = events_v1.create_event(&completed_params, &legacy_event_create_op);
    events_v1.apply_to_bounty(&completed_id, &winner, &next_op(&env, &ops));
    events_v1.submit(
        &completed_id,
        &winner,
        &String::from_str(&env, "ipfs://submission"),
        &next_op(&env, &ops),
    );
    events_v1.select_winners(
        &completed_id,
        &vec![
            &env,
            old_events::WinnerSpec {
                position: 1,
                recipient: winner.clone(),
                reputation_bump: 7,
            },
        ],
        &next_op(&env, &ops),
    );

    let cancelling_id = events_v1.create_event(
        &params(&env, &owner, &token_id, "cancelling"),
        &next_op(&env, &ops),
    );
    events_v1.add_funds(
        &cancelling_id,
        &contributor,
        &CONTRIBUTION,
        &next_op(&env, &ops),
    );
    events_v1.start_cancel(&cancelling_id, &next_op(&env, &ops));

    let mut active_params = params(&env, &owner, &token_id, "active");
    active_params.manager = Some(manager.clone());
    let active_contributor_id = events_v1.create_event(&active_params, &next_op(&env, &ops));
    events_v1.add_funds(
        &active_contributor_id,
        &contributor,
        &CONTRIBUTION,
        &next_op(&env, &ops),
    );

    let mut grant_params = params(&env, &owner, &token_id, "grant");
    grant_params.pillar = old_events::Pillar::Grant;
    grant_params.release_kind = old_events::ReleaseKind::Multi(2);
    let grant_id = events_v1.create_event(&grant_params, &next_op(&env, &ops));
    events_v1.select_winners(
        &grant_id,
        &vec![
            &env,
            old_events::WinnerSpec {
                position: 1,
                recipient: grant_recipient.clone(),
                reputation_bump: 0,
            },
        ],
        &next_op(&env, &ops),
    );
    let grant_balance_before = token.balance(&grant_recipient);
    events_v1.claim_milestone(&grant_id, &grant_recipient, &0, &5, &next_op(&env, &ops));

    let mut crowdfunding_params = params(&env, &owner, &token_id, "crowdfunding");
    crowdfunding_params.pillar = old_events::Pillar::Crowdfunding;
    crowdfunding_params.release_kind = old_events::ReleaseKind::Multi(2);
    let crowdfunding_id = events_v1.create_event(&crowdfunding_params, &next_op(&env, &ops));
    events_v1.add_funds(
        &crowdfunding_id,
        &contributor,
        &CONTRIBUTION,
        &next_op(&env, &ops),
    );
    events_v1.claim_milestone(&crowdfunding_id, &owner, &0, &0, &next_op(&env, &ops));

    profile_v1.migrate();
    events_v1.migrate();
    profile_v1.set_admin(&next_profile_admin);
    events_v1.set_admin(&next_events_admin);
    profile_v1.propose_events_contract(&rotated_events_id);

    let new_profile_hash = env.deployer().upload_contract_wasm(new_profile::WASM);
    profile_v1.propose_upgrade(&new_profile_hash, &String::from_str(&env, "1.2.0"));
    let new_events_hash = env.deployer().upload_contract_wasm(new_events::WASM);
    events_v1.propose_upgrade(&new_events_hash, &String::from_str(&env, "1.5.0"));
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number += TIMELOCK;
    });
    profile_v1.apply_upgrade();

    let profile_v2 = new_profile::Client::new(&env, &profile_id);
    assert_eq!(profile_v2.version(), String::from_str(&env, "1.2.0"));
    assert_eq!(
        profile_v2.get_migrated_to_version(),
        Some(String::from_str(&env, "1.1.0"))
    );
    let pending_events_contract = profile_v2.get_pending_events_contract().unwrap();
    assert_eq!(pending_events_contract.target, rotated_events_id);
    profile_v2.cancel_pending_events_contract();
    assert_eq!(profile_v2.get_pending_events_contract(), None);
    profile_v2.accept_admin();
    assert_eq!(profile_v2.get_admin(), next_profile_admin);
    let stored_profile = profile_v2.get_profile(&winner).unwrap();
    assert_eq!(stored_profile.reputation, 7);
    assert_eq!(profile_v2.get_earnings(&winner, &token_id), BUDGET);
    let profile_replay_error = profile_v2
        .try_bootstrap_self(&legacy_profile_user, &legacy_profile_op)
        .expect_err("SDK-23 profile replay row must remain effective")
        .unwrap();
    assert_eq!(profile_replay_error, new_profile::Error::OpAlreadySeen);

    events_v1.apply_upgrade();

    let events_v2 = new_events::Client::new(&env, &events_id);
    assert_eq!(events_v2.version(), String::from_str(&env, "1.5.0"));
    assert_eq!(
        events_v2.get_migrated_to_version(),
        Some(String::from_str(&env, "1.1.0"))
    );
    events_v2.accept_admin();
    assert_eq!(events_v2.get_admin(), next_events_admin);
    assert_eq!(events_v2.get_fee_account(), fee_account);
    assert_eq!(events_v2.get_fee_bps(), FEE_BPS);
    assert_eq!(events_v2.get_profile_contract(), profile_id);
    assert_eq!(events_v2.get_applicant_count(&completed_id), 1);
    assert_eq!(
        events_v2.get_applicant_at(&completed_id, &0),
        Some(winner.clone())
    );
    assert_eq!(
        env.as_contract(&events_id, || env.storage().persistent().get::<_, u32>(
            &LegacyEventsDataKey::EventApplicantSlot(completed_id, winner.clone())
        )),
        Some(1)
    );

    let completed = events_v2.get_event(&completed_id);
    assert_eq!(completed.title, String::from_str(&env, "completed"));
    assert_eq!(completed.status, new_events::EventStatus::Completed);
    let submission = events_v2.get_submission(&completed_id, &winner);
    assert_eq!(
        submission.content_uri,
        String::from_str(&env, "ipfs://submission")
    );
    let winners = events_v2.get_winners(&completed_id);
    assert_eq!(winners.len(), 1);
    assert_eq!(winners.get(0).unwrap().recipient, winner);

    assert_eq!(events_v2.get_contributor_count(&cancelling_id), 1);
    assert_eq!(
        events_v2.get_contributor_amount(&cancelling_id, &contributor),
        CONTRIBUTION
    );
    assert_eq!(
        events_v2.process_cancel_batch(&cancelling_id, &25, &next_op(&env, &ops)),
        0
    );
    events_v2.finalize_cancel(&cancelling_id, &next_op(&env, &ops));
    assert_eq!(
        events_v2.get_event(&cancelling_id).status,
        new_events::EventStatus::Cancelled
    );

    assert_eq!(
        events_v2.get_contributor_amount(&active_contributor_id, &contributor),
        CONTRIBUTION
    );
    assert_eq!(
        events_v2.get_contributor_at(&active_contributor_id, &0),
        Some(contributor.clone())
    );
    assert_eq!(
        env.as_contract(&events_id, || env.storage().persistent().get::<_, u32>(
            &LegacyEventsDataKey::ContributorSlot(active_contributor_id, contributor.clone())
        )),
        Some(1)
    );
    assert_eq!(events_v2.get_manager(&active_contributor_id), manager);
    assert!(events_v2
        .try_start_cancel(&active_contributor_id, &next_op(&env, &ops))
        .is_err());

    let events_replay_error = events_v2
        .try_create_event(
            &new_params(&env, &owner, &token_id, "legacy-op-replay"),
            &legacy_event_create_op,
        )
        .expect_err("SDK-23 events replay row must remain effective")
        .unwrap();
    assert_eq!(events_replay_error, new_events::Error::OpAlreadySeen);

    let post_upgrade_id = events_v2.create_event(
        &new_params(&env, &owner, &token_id, "post-upgrade"),
        &next_op(&env, &ops),
    );
    assert_eq!(post_upgrade_id, crowdfunding_id + 1);

    let grant_replay_error = events_v2
        .try_claim_milestone(&grant_id, &grant_recipient, &0, &5, &next_op(&env, &ops))
        .expect_err("SDK-23 milestone row must reject a duplicate claim")
        .unwrap();
    assert_eq!(
        grant_replay_error,
        new_events::Error::MilestoneAlreadyClaimed
    );
    let fee_before_grant_claim = token.balance(&fee_account);
    events_v2.claim_milestone(&grant_id, &grant_recipient, &1, &5, &next_op(&env, &ops));
    assert_eq!(
        token.balance(&grant_recipient) - grant_balance_before,
        BUDGET
    );
    assert_eq!(token.balance(&fee_account) - fee_before_grant_claim, 0);
    assert_eq!(
        events_v2.get_event(&grant_id).status,
        new_events::EventStatus::Completed
    );
    assert_eq!(
        profile_v2.get_profile(&grant_recipient).unwrap().reputation,
        10
    );
    assert_eq!(profile_v2.get_earnings(&grant_recipient, &token_id), BUDGET);

    let crowdfunding_replay_error = events_v2
        .try_claim_milestone(&crowdfunding_id, &owner, &0, &0, &next_op(&env, &ops))
        .expect_err("SDK-23 crowdfunding milestone row must reject a duplicate claim")
        .unwrap();
    assert_eq!(
        crowdfunding_replay_error,
        new_events::Error::MilestoneAlreadyClaimed
    );
    let crowdfunding_remaining = events_v2.get_event(&crowdfunding_id).remaining_escrow;
    let crowdfunding_balance_before = token.balance(&owner);
    let fee_before_crowdfunding_claim = token.balance(&fee_account);
    events_v2.claim_milestone(&crowdfunding_id, &owner, &1, &0, &next_op(&env, &ops));
    let crowdfunding_fee = crowdfunding_remaining * FEE_BPS as i128 / 10_000;
    assert_eq!(
        token.balance(&owner) - crowdfunding_balance_before,
        crowdfunding_remaining - crowdfunding_fee
    );
    assert_eq!(
        token.balance(&fee_account) - fee_before_crowdfunding_claim,
        crowdfunding_fee
    );
    assert_eq!(
        events_v2.get_event(&crowdfunding_id).status,
        new_events::EventStatus::Completed
    );

    profile_v2.migrate();
    events_v2.migrate();
    assert_eq!(
        profile_v2.get_migrated_to_version(),
        Some(String::from_str(&env, "1.2.0"))
    );
    assert_eq!(
        events_v2.get_migrated_to_version(),
        Some(String::from_str(&env, "1.5.0"))
    );
}

#[test]
fn sdk23_pre27_prize_and_cancellation_keys_work_after_upgrade() {
    let env = Env::default();
    let ops = Cell::new(1_000);
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number = 100;
        ledger.timestamp = 1_000_000;
    });

    let admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let profile_id = env.register(old_profile::WASM, (&admin,));
    let profile_v1 = old_profile::Client::new(&env, &profile_id);
    let events_id = env.register(
        pre27_events::WASM,
        (&admin, &fee_account, &FEE_BPS, &profile_id),
    );
    let events_v1 = pre27_events::Client::new(&env, &events_id);
    assert_eq!(events_v1.version(), String::from_str(&env, "1.3.0"));
    profile_v1.set_events_contract(&events_id);

    let issuer = Address::generate(&env);
    let asset = env.register_stellar_asset_contract_v2(issuer);
    let token_id = asset.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&fee_account, &0);

    let owner = Address::generate(&env);
    let winner = Address::generate(&env);
    let second_winner = Address::generate(&env);
    let contributor = Address::generate(&env);
    token_admin.mint(&owner, &(BUDGET * 3));
    token_admin.mint(&contributor, &(CONTRIBUTION * 2));
    events_v1.register_supported_token(&token_id);

    let mut prize_params = pre27_params(&env, &owner, &token_id, "prize");
    let mut prize_distribution = Map::new(&env);
    prize_distribution.set(1, 50);
    prize_distribution.set(2, 50);
    prize_params.winner_distribution = prize_distribution;
    let prize_id = events_v1.create_event(&prize_params, &next_op(&env, &ops));
    events_v1.select_winners(
        &prize_id,
        &vec![
            &env,
            pre27_events::WinnerSpec {
                position: 1,
                recipient: winner.clone(),
                reputation_bump: 11,
            },
        ],
        &next_op(&env, &ops),
    );

    let cancellation_id = events_v1.create_event(
        &pre27_params(&env, &owner, &token_id, "aggregate"),
        &next_op(&env, &ops),
    );
    events_v1.add_funds(
        &cancellation_id,
        &contributor,
        &CONTRIBUTION,
        &next_op(&env, &ops),
    );

    let new_profile_hash = env.deployer().upload_contract_wasm(new_profile::WASM);
    profile_v1.propose_upgrade(&new_profile_hash, &String::from_str(&env, "1.2.0"));
    let new_events_hash = env.deployer().upload_contract_wasm(new_events::WASM);
    events_v1.propose_upgrade(&new_events_hash, &String::from_str(&env, "1.5.0"));
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number += TIMELOCK;
    });
    profile_v1.apply_upgrade();

    events_v1.apply_upgrade();

    let profile_v2 = new_profile::Client::new(&env, &profile_id);
    let events_v2 = new_events::Client::new(&env, &events_id);
    let token = token::Client::new(&env, &token_id);

    assert_eq!(token.balance(&winner), 0);
    assert!(events_v2
        .try_start_cancel(&prize_id, &next_op(&env, &ops))
        .is_err());
    events_v2.select_winners(
        &prize_id,
        &vec![
            &env,
            new_events::WinnerSpec {
                position: 2,
                recipient: second_winner.clone(),
                reputation_bump: 13,
            },
        ],
        &next_op(&env, &ops),
    );
    let fee_before_prize_claims = token.balance(&fee_account);
    events_v2.claim_prize(&prize_id, &1, &next_op(&env, &ops));
    events_v2.claim_prize(&prize_id, &2, &next_op(&env, &ops));
    assert_eq!(token.balance(&winner), BUDGET / 2);
    assert_eq!(token.balance(&second_winner), BUDGET / 2);
    assert_eq!(token.balance(&fee_account) - fee_before_prize_claims, 0);
    assert_eq!(
        events_v2.get_event(&prize_id).status,
        new_events::EventStatus::Completed
    );
    assert_eq!(profile_v2.get_profile(&winner).unwrap().reputation, 11);
    assert_eq!(
        profile_v2.get_profile(&second_winner).unwrap().reputation,
        13
    );
    assert_eq!(profile_v2.get_earnings(&winner, &token_id), BUDGET / 2);
    assert_eq!(
        profile_v2.get_earnings(&second_winner, &token_id),
        BUDGET / 2
    );

    events_v2.start_cancel(&cancellation_id, &next_op(&env, &ops));
    assert_eq!(
        events_v2.get_event(&cancellation_id).status,
        new_events::EventStatus::Cancelling
    );
    assert_eq!(
        events_v2.process_cancel_batch(&cancellation_id, &25, &next_op(&env, &ops)),
        0
    );
    events_v2.finalize_cancel(&cancellation_id, &next_op(&env, &ops));
    assert_eq!(
        events_v2.get_event(&cancellation_id).status,
        new_events::EventStatus::Cancelled
    );
}

#[test]
fn exact_mainnet_snapshot_survives_h6_upgrade_to_sdk27() {
    let env = Env::from_ledger_snapshot_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/mainnet-state-63617727.json"
    ));
    env.mock_all_auths_allowing_non_root_auth();

    let events_id = Address::from_str(
        &env,
        "CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ",
    );
    let profile_id = Address::from_str(
        &env,
        "CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC",
    );
    let admin = Address::from_str(
        &env,
        "GCVK72I6TVJVDTTY4UKU6MQT4QJ2T2AAG3NULNEUDM46L3UOQYDSO4O2",
    );
    let fee_account = Address::from_str(
        &env,
        "GADSIP2HPINWTMEUNMVYA5NJRL372OV52HDEJTRTLDZXMHADIPQNYH55",
    );
    let token_0 = Address::from_str(
        &env,
        "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75",
    );
    let token_1 = Address::from_str(
        &env,
        "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA",
    );
    let profile_addresses = [
        "GDCLE5VYYAZ5HYSBSG3PIKQEXHPYNSHF2W7PSW2R7WEIPP2AD275EL4S",
        "GAHGDV4MWDLT3BXOUT5A2T774IUAE54DZ6BUCBH4O7PG5TJGWXSV3OJO",
        "GBVQKBWHZGO6YED3EVXOZW76VPNYUMJLBLKE5IN4XE5LTT262UXCBIR7",
        "GCCCPDSWYDVTD2VSXNXDKWWNENSRTA637CHVHIIXISTKSK4FV6T7G3HV",
        "GCS5XWO3M72B6PSMZVGOSB3CLQVWB5OJ6Z7FTFAP5FAZVWS3MLO4Y3WH",
    ];
    let legacy_op_ids = [
        [
            0xd0, 0x12, 0x86, 0xfa, 0x65, 0x4b, 0x4f, 0x92, 0x4e, 0xb2, 0xa1, 0x65, 0xe3, 0xd8,
            0x38, 0x61, 0x42, 0x91, 0x7b, 0x07, 0xaf, 0x9a, 0x22, 0xf2, 0x22, 0x42, 0x0b, 0xa8,
            0xd2, 0xb0, 0x1a, 0xd8,
        ],
        [
            0x22, 0xf9, 0xdf, 0xf5, 0xb7, 0x18, 0x43, 0x44, 0xe5, 0xba, 0xc3, 0x94, 0xf9, 0xfb,
            0x14, 0x1a, 0xf5, 0xb1, 0xda, 0x8e, 0x71, 0xae, 0x36, 0xe7, 0x61, 0x2f, 0xb1, 0xaa,
            0x68, 0xf2, 0xf1, 0xbc,
        ],
        [
            0xd4, 0x56, 0xce, 0x7d, 0xbe, 0x62, 0xe2, 0xf5, 0x94, 0x9d, 0xc2, 0xcc, 0xe2, 0xb6,
            0xa9, 0x99, 0x00, 0xce, 0x9e, 0xd0, 0x8e, 0xe4, 0x42, 0x37, 0x21, 0x9a, 0xf2, 0x27,
            0x55, 0xfa, 0x71, 0xd5,
        ],
    ];

    env.deployer().upload_contract_wasm(old_events::WASM);
    env.deployer().upload_contract_wasm(old_profile::WASM);

    let events_v1 = old_events::Client::new(&env, &events_id);
    let profile_v1 = old_profile::Client::new(&env, &profile_id);
    assert_eq!(events_v1.version(), String::from_str(&env, "1.1.0"));
    assert_eq!(profile_v1.version(), String::from_str(&env, "1.1.0"));
    let expected_event_id_base = events_v1.id_base();
    events_v1.pause();
    profile_v1.pause();
    assert!(events_v1.is_paused());
    assert!(profile_v1.is_paused());

    for strkey in profile_addresses {
        let address = Address::from_str(&env, strkey);
        let profile = profile_v1.get_profile(&address).unwrap();
        assert_eq!(profile.reputation, 0);
    }
    assert_eq!(
        env.as_contract(&profile_id, || env
            .storage()
            .instance()
            .get::<_, u32>(&LegacyProfileDataKey::DefaultBootstrapCredits)),
        Some(10)
    );
    assert_eq!(
        env.as_contract(&profile_id, || env
            .storage()
            .instance()
            .get::<_, u32>(&LegacyProfileDataKey::DeploymentSeq)),
        Some(63_222_664)
    );
    for bytes in legacy_op_ids {
        let op_id = BytesN::from_array(&env, &bytes);
        assert_eq!(
            env.as_contract(&profile_id, || env
                .storage()
                .temporary()
                .get::<_, bool>(&LegacyProfileDataKey::OpSeen(op_id))),
            Some(true)
        );
    }

    let new_profile_hash = env.deployer().upload_contract_wasm(new_profile::WASM);
    profile_v1.propose_upgrade(&new_profile_hash, &String::from_str(&env, "1.2.0"));
    let new_events_hash = env.deployer().upload_contract_wasm(new_events::WASM);
    events_v1.propose_upgrade(&new_events_hash, &String::from_str(&env, "1.5.0"));
    env.ledger().with_mut(|ledger| {
        ledger.sequence_number += TIMELOCK;
    });
    profile_v1.apply_upgrade();
    let profile_v2 = new_profile::Client::new(&env, &profile_id);
    assert!(profile_v2.is_paused());

    events_v1.apply_upgrade();

    let events_v2 = new_events::Client::new(&env, &events_id);
    assert!(events_v2.is_paused());
    profile_v2.migrate();
    events_v2.migrate();
    profile_v2.unpause();
    events_v2.unpause();

    assert_eq!(events_v2.version(), String::from_str(&env, "1.5.0"));
    assert_eq!(profile_v2.version(), String::from_str(&env, "1.2.0"));
    assert_eq!(
        events_v2.get_migrated_to_version(),
        Some(String::from_str(&env, "1.5.0"))
    );
    assert_eq!(
        profile_v2.get_migrated_to_version(),
        Some(String::from_str(&env, "1.2.0"))
    );
    assert!(!events_v2.is_paused());
    assert!(!profile_v2.is_paused());
    assert_eq!(events_v2.get_admin(), admin);
    assert_eq!(events_v2.get_fee_account(), fee_account);
    assert_eq!(events_v2.get_fee_bps(), 250);
    assert_eq!(events_v2.get_profile_contract(), profile_id);
    assert_eq!(events_v2.id_base(), expected_event_id_base);
    assert!(events_v2.is_supported_token(&token_0));
    assert!(events_v2.is_supported_token(&token_1));
    assert_eq!(events_v2.supported_token_count(), 2);
    assert_eq!(events_v2.supported_token_at(&0), Some(token_0.clone()));
    assert_eq!(events_v2.supported_token_at(&1), Some(token_1.clone()));
    assert_eq!(
        env.as_contract(&events_id, || env.storage().instance().get::<_, u32>(
            &LegacyEventsDataKey::SupportedTokenSlot(token_0.clone())
        )),
        Some(1)
    );
    assert_eq!(
        env.as_contract(&events_id, || env.storage().instance().get::<_, u32>(
            &LegacyEventsDataKey::SupportedTokenSlot(token_1.clone())
        )),
        Some(2)
    );
    events_v2.deregister_supported_token(&token_1);
    assert_eq!(events_v2.supported_token_count(), 1);
    assert!(!events_v2.is_supported_token(&token_1));
    events_v2.register_supported_token(&token_1);
    assert_eq!(events_v2.supported_token_count(), 2);
    assert_eq!(events_v2.supported_token_at(&1), Some(token_1.clone()));
    assert_eq!(profile_v2.get_admin(), admin);
    assert_eq!(profile_v2.get_events_contract(), Some(events_id));
    let legacy_replay_user = Address::from_str(&env, profile_addresses[0]);
    let legacy_replay_op = BytesN::from_array(&env, &legacy_op_ids[0]);
    let legacy_replay_error = profile_v2
        .try_bootstrap_self(&legacy_replay_user, &legacy_replay_op)
        .expect_err("captured SDK-23 replay row must remain effective")
        .unwrap();
    assert_eq!(legacy_replay_error, new_profile::Error::OpAlreadySeen);

    for strkey in profile_addresses {
        let address = Address::from_str(&env, strkey);
        let profile = profile_v2.get_profile(&address).unwrap();
        assert_eq!(profile.reputation, 0);
    }
    assert_eq!(
        env.as_contract(&profile_id, || env
            .storage()
            .instance()
            .get::<_, u32>(&LegacyProfileDataKey::DefaultBootstrapCredits)),
        Some(10)
    );
    assert_eq!(
        env.as_contract(&profile_id, || env
            .storage()
            .instance()
            .get::<_, u32>(&LegacyProfileDataKey::DeploymentSeq)),
        Some(63_222_664)
    );
    for bytes in legacy_op_ids {
        let op_id = BytesN::from_array(&env, &bytes);
        assert_eq!(
            env.as_contract(&profile_id, || env
                .storage()
                .temporary()
                .get::<_, bool>(&LegacyProfileDataKey::OpSeen(op_id))),
            Some(true)
        );
    }
}
