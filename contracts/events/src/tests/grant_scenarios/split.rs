use soroban_sdk::vec;

use super::*;
use crate::event_ops::MAX_SPLIT_MILESTONES;

fn refused(g: &G, spec: Spec, error: Error) {
    g.token_admin.mint(&g.owner, &usdc(2_000));
    fails_with(g.events.try_create_event(&g.params(&spec), &g.op()), error);
}

#[test]
fn a_split_is_stored_on_the_grant() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 0).split(&[3_000, 4_000, 3_000]));
    assert_eq!(
        g.event(id).release_kind,
        ReleaseKind::Split(vec![&g.env, 3_000, 4_000, 3_000])
    );
}

#[test]
fn a_split_must_add_up_to_the_whole_award() {
    let g = setup();
    refused(
        &g,
        Spec::new(usdc(1_000), 0).split(&[3_000, 4_000, 2_999]),
        Error::InvalidMilestoneSplit,
    );
    refused(
        &g,
        Spec::new(usdc(1_000), 0).split(&[3_000, 4_000, 3_001]),
        Error::InvalidMilestoneSplit,
    );
}

#[test]
fn every_milestone_in_a_split_pays_something() {
    let g = setup();
    refused(
        &g,
        Spec::new(usdc(1_000), 0).split(&[10_000, 0]),
        Error::InvalidMilestoneSplit,
    );
    refused(
        &g,
        Spec::new(usdc(1_000), 0).split(&[]),
        Error::InvalidMilestoneSplit,
    );
}

#[test]
fn a_split_is_bounded() {
    let g = setup();
    let mut shares = std::vec![1u32; MAX_SPLIT_MILESTONES as usize];
    shares[0] = 10_000 - (MAX_SPLIT_MILESTONES - 1);
    g.create(Spec::new(usdc(1_000), 0).split(&shares));

    let mut over = std::vec![1u32; MAX_SPLIT_MILESTONES as usize + 1];
    over[0] = 10_000 - MAX_SPLIT_MILESTONES;
    refused(
        &g,
        Spec::new(usdc(1_000), 0).split(&over),
        Error::InvalidMilestoneSplit,
    );
}

#[test]
fn a_split_is_for_grants_only() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(2_000));
    for pillar in [Pillar::Crowdfunding, Pillar::Bounty, Pillar::Hackathon] {
        let mut params = g.params(&Spec::new(usdc(1_000), 0).split(&[5_000, 5_000]));
        params.pillar = pillar;
        fails_with(
            g.events.try_create_event(&params, &g.op()),
            Error::InvalidReleaseKind,
        );
    }
}

#[test]
fn each_milestone_pays_its_share() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 0).split(&[3_000, 4_000, 3_000]));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);

    g.pay(id, &a, 0, usdc(300));
    g.pay(id, &a, 1, usdc(400));
    g.pay(id, &a, 2, usdc(300));
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn awards_of_different_sizes_share_one_split() {
    let g = setup();
    let id = g.create(
        Spec::new(usdc(1_500), 0)
            .split(&[2_000, 8_000])
            .floors(&[(1, 1), (2, 1)]),
    );
    let a = g.someone();
    let b = g.someone();
    g.select(
        id,
        &[(a.clone(), 1, usdc(1_000)), (b.clone(), 2, usdc(500))],
    );

    g.pay(id, &a, 0, usdc(200));
    g.pay(id, &b, 0, usdc(100));
    g.pay(id, &b, 1, usdc(400));
    g.pay(id, &a, 1, usdc(800));
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn the_last_milestone_settled_takes_the_rounding_in_any_order() {
    let g = setup();
    // 1_000_001 units: 30% and 40% round down, and the remainder carries the dust.
    let award = 1_000_001;
    let id = g.create(
        Spec::new(award, 0)
            .split(&[3_000, 4_000, 3_000])
            .floors(&[(1, award)]),
    );
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, award)]);

    g.pay(id, &a, 1, 400_000);
    g.pay(id, &a, 2, 300_000);
    // Milestone 0 settles last: its 300_000 plus the one unit of rounding.
    g.pay(id, &a, 0, 300_001);
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn a_forfeit_releases_that_milestones_share() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 0).split(&[2_500, 5_000, 2_500]));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);

    g.pay(id, &a, 0, usdc(250));
    g.forfeit(id, &a, 1, usdc(500));
    g.pay(id, &a, 2, usdc(250));
    assert_eq!(g.owed(id), 0);
}

#[test]
fn selection_refuses_an_award_too_small_for_its_smallest_share() {
    let g = setup();
    let id = g.create(
        Spec::new(usdc(1_000), 0)
            .split(&[1, 9_999])
            .floors(&[(1, 1)]),
    );
    let a = g.someone();
    // 9_999 units leave the 1 bps milestone with nothing.
    fails_with(
        g.events
            .try_select_winners(&id, &g.specs(&[(a.clone(), 1, 9_999)]), &g.op()),
        Error::InvalidDistribution,
    );
    // 10_000 units pay it exactly one.
    g.select(id, &[(a.clone(), 1, 10_000)]);
    g.pay(id, &a, 0, 1);
    g.pay(id, &a, 1, 9_999);
}

#[test]
fn a_split_grant_selects_once() {
    let g = setup();
    let id = g.create(
        Spec::new(usdc(1_000), 0)
            .split(&[5_000, 5_000])
            .floors(&[(1, 1), (2, 1)]),
    );
    g.select(id, &[(g.someone(), 1, usdc(400))]);
    fails_with(
        g.events
            .try_select_winners(&id, &g.specs(&[(g.someone(), 2, usdc(400))]), &g.op()),
        Error::WinnersAlreadySelected,
    );
}
