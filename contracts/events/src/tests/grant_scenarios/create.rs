use soroban_sdk::{testutils::Address as _, Address, Map, String};

use super::*;
use crate::events::EventCreated;

#[test]
fn publish_escrows_the_budget_and_charges_the_fee_on_top() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));

    let event = g.event(id);
    assert_eq!(event.pillar, Pillar::Grant);
    assert_eq!(event.release_kind, ReleaseKind::Multi(2));
    assert_eq!(event.total_budget, usdc(1_000));
    assert_eq!(g.balance(&g.fee_account), usdc(25));
    assert_eq!(g.owed(id), 0);
    assert!(g.emitted(EventCreated {
        id,
        pillar: Pillar::Grant,
        owner: g.owner.clone(),
        token: g.token.address.clone(),
        total_budget: usdc(1_000),
        content_uri: String::from_str(&g.env, "https://api.boundless.fi/grants/x/content"),
        title: String::from_str(&g.env, "Community grants"),
    }));
}

#[test]
fn every_preset_shape_publishes() {
    let g = setup();
    // micro: 5 x 200, one milestone
    g.create(Spec::new(usdc(1_000), 1).floors(&[
        (1, usdc(200)),
        (2, usdc(200)),
        (3, usdc(200)),
        (4, usdc(200)),
        (5, usdc(200)),
    ]));
    // builder: tiered, three milestones
    g.create(Spec::new(usdc(25_000), 3).floors(&[
        (1, usdc(12_000)),
        (2, usdc(8_000)),
        (3, usdc(5_000)),
    ]));
    // infrastructure: one award, five milestones
    g.create(Spec::new(usdc(100_000), 5));
    // research: floors below the budget leave headroom
    g.create(Spec::new(usdc(20_000), 2).floors(&[(1, usdc(5_000)), (2, usdc(5_000))]));
    // backend clamp: ten milestones
    g.create(Spec::new(usdc(6_000), 10));
}

#[test]
fn fee_override_applies_to_the_grant_only() {
    let g = setup();
    g.create(Spec::new(usdc(1_000), 2).fee_override(0));
    assert_eq!(g.balance(&g.fee_account), 0);

    g.create(Spec::new(usdc(1_000), 2).fee_override(1_000));
    assert_eq!(g.balance(&g.fee_account), usdc(100));
}

#[test]
fn fee_override_above_the_cap_is_refused() {
    let g = setup();
    let spec = Spec::new(usdc(1_000), 2).fee_override(1_001);
    g.token_admin.mint(&g.owner, &usdc(2_000));
    fails_with(
        g.events.try_create_event(&g.params(&spec), &g.op()),
        Error::InvalidFeeBps,
    );
}

#[test]
fn single_release_is_not_a_grant() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(2_000));
    let mut params = g.params(&Spec::new(usdc(1_000), 2));
    params.release_kind = ReleaseKind::Single;
    fails_with(
        g.events.try_create_event(&params, &g.op()),
        Error::InvalidReleaseKind,
    );
}

#[test]
fn zero_milestones_is_refused() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(2_000));
    fails_with(
        g.events
            .try_create_event(&g.params(&Spec::new(usdc(1_000), 0)), &g.op()),
        Error::InvalidReleaseKind,
    );
}

#[test]
fn budget_must_be_positive() {
    let g = setup();
    for budget in [0, -1] {
        let mut params = g.params(&Spec::new(usdc(1_000), 1));
        params.total_budget = budget;
        fails_with(
            g.events.try_create_event(&params, &g.op()),
            Error::InvalidBudget,
        );
    }
}

#[test]
fn floors_must_be_present_positive_and_within_budget() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(10_000));

    let mut empty = g.params(&Spec::new(usdc(1_000), 1));
    empty.prize_floors = Map::new(&g.env);
    fails_with(
        g.events.try_create_event(&empty, &g.op()),
        Error::InvalidDistribution,
    );

    let zero = g.params(&Spec::new(usdc(1_000), 1).floors(&[(1, 0)]));
    fails_with(
        g.events.try_create_event(&zero, &g.op()),
        Error::InvalidDistribution,
    );

    let over = g.params(&Spec::new(usdc(1_000), 1).floors(&[(1, usdc(600)), (2, usdc(401))]));
    fails_with(
        g.events.try_create_event(&over, &g.op()),
        Error::DistributionMismatch,
    );

    let exact = g.params(&Spec::new(usdc(1_000), 1).floors(&[(1, usdc(600)), (2, usdc(400))]));
    assert!(g.events.try_create_event(&exact, &g.op()).is_ok());
}

#[test]
fn unlisted_token_is_refused() {
    let g = setup();
    let other = g
        .env
        .register_stellar_asset_contract_v2(Address::generate(&g.env));
    let mut params = g.params(&Spec::new(usdc(1_000), 1));
    params.token = other.address();
    fails_with(
        g.events.try_create_event(&params, &g.op()),
        Error::TokenNotSupported,
    );
}

#[test]
fn title_over_120_characters_is_refused() {
    let g = setup();
    let mut params = g.params(&Spec::new(usdc(1_000), 1));
    params.title = String::from_str(&g.env, &"x".repeat(121));
    fails_with(
        g.events.try_create_event(&params, &g.op()),
        Error::TitleTooLong,
    );
}

#[test]
fn owner_without_funds_cannot_publish() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(1_000));
    // budget is there, the fee on top is not
    assert!(g
        .events
        .try_create_event(&g.params(&Spec::new(usdc(1_000), 1)), &g.op())
        .is_err());
    assert_eq!(g.balance(&g.owner), usdc(1_000));
    assert_eq!(g.escrow_balance(), 0);
}

#[test]
fn publish_op_id_cannot_be_replayed() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(5_000));
    let op = g.op();
    let params = g.params(&Spec::new(usdc(1_000), 1));
    g.events.create_event(&params, &op);
    fails_with(
        g.events.try_create_event(&params, &op),
        Error::OpAlreadySeen,
    );
}

#[test]
fn naming_a_manager_only_proposes_it() {
    let g = setup();
    let manager = g.someone();
    let id = g.create(Spec::new(usdc(1_000), 1).manager(&manager));

    assert_eq!(g.events.get_manager(&id), g.owner);
    let pending = g.events.get_pending_manager(&id).unwrap();
    assert_eq!(pending.target, manager);

    g.events.accept_manager(&id);
    assert_eq!(g.events.get_manager(&id), manager);
    assert!(g.events.get_pending_manager(&id).is_none());
}
