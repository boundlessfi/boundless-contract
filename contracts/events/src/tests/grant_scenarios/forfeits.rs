use super::*;
use crate::events::{MilestoneForfeited, OwnerResidualRefunded};

#[test]
fn a_forfeit_frees_the_share_without_moving_tokens() {
    let g = setup();
    let id = g.create(Spec::new(usdc(900), 3));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(900))]);
    g.forfeit(id, &a, 1, usdc(300));
    assert!(g.emitted(MilestoneForfeited {
        event_id: id,
        recipient: a.clone(),
        milestone: 1,
        amount: usdc(300),
    }));
    assert_eq!(g.owed(id), usdc(600));
    assert_eq!(g.event(id).remaining_escrow, usdc(900));
    assert_eq!(
        g.events.get_winner_count(&id),
        1,
        "no paid row for a forfeit"
    );
}

#[test]
fn a_forfeited_last_milestone_takes_the_remainder_with_it() {
    let g = setup();
    let award = usdc(10) + 2;
    let id = g.create(Spec::new(award, 3));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, award)]);
    let part = award / 3;
    g.pay(id, &a, 0, part);
    g.pay(id, &a, 2, part);
    g.forfeit(id, &a, 1, award - 2 * part);
    assert_eq!(g.owed(id), 0);
    g.cancel(id);
    assert_eq!(g.balance(&g.owner), award - 2 * part);
    assert!(g.emitted(OwnerResidualRefunded {
        event_id: id,
        owner: g.owner.clone(),
        amount: award - 2 * part,
    }));
}

#[test]
fn a_migration_pass_after_a_forfeit_keeps_it_released() {
    let g = setup();
    let id = g.create(Spec::new(usdc(900), 3));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(900))]);
    g.forfeit(id, &a, 0, usdc(300));

    // Every upgrade pages migrate_events over the events created since the
    // last stamp, so a live grant is always in its path.
    assert_eq!(g.events.migrate_events(&8_u32), 0);
    assert_eq!(g.owed(id), usdc(600));

    g.pay(id, &a, 1, usdc(300));
    g.pay(id, &a, 2, usdc(300));
    assert_eq!(g.owed(id), 0);
    g.cancel(id);
    assert_eq!(g.balance(&g.owner), usdc(300));
}

#[test]
fn ending_a_whole_award_returns_it_on_close_while_others_continue() {
    let g = setup();
    let id = g.create(Spec::new(usdc(2_000), 2).floors(&[(1, usdc(1_000)), (2, usdc(1_000))]));
    let (a, b) = (g.someone(), g.someone());
    g.select(
        id,
        &[(a.clone(), 1, usdc(1_000)), (b.clone(), 2, usdc(1_000))],
    );

    g.forfeit(id, &a, 0, usdc(500));
    g.forfeit(id, &a, 1, usdc(500));
    fails_with(
        g.events.try_start_cancel(&id, &g.op()),
        Error::AwardsOutstanding,
    );
    g.pay(id, &b, 0, usdc(500));
    g.pay(id, &b, 1, usdc(500));
    g.cancel(id);
    assert_eq!(g.balance(&b), usdc(1_000));
    assert_eq!(g.balance(&a), 0);
    assert_eq!(g.balance(&g.owner), usdc(1_000));
}

#[test]
fn a_settled_milestone_cannot_be_settled_again_either_way() {
    let g = setup();
    let id = g.create(Spec::new(usdc(900), 3));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(900))]);
    g.pay(id, &a, 0, usdc(300));
    g.forfeit(id, &a, 1, usdc(300));

    fails_with(
        g.events.try_forfeit_milestone(&id, &a, &0, &g.op()),
        Error::MilestoneAlreadyClaimed,
    );
    fails_with(
        g.events.try_claim_milestone(&id, &a, &1, &0, &g.op()),
        Error::MilestoneAlreadyClaimed,
    );
    fails_with(
        g.events.try_forfeit_milestone(&id, &a, &1, &g.op()),
        Error::MilestoneAlreadyClaimed,
    );
}

#[test]
fn forfeits_are_signed_by_the_owner_alone() {
    let g = setup();
    let manager = g.someone();
    let id = g.create(Spec::new(usdc(900), 3).manager(&manager));
    g.events.accept_manager(&id);
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(900))]);

    for signer in [a.clone(), manager.clone(), g.admin.clone()] {
        let op = g.op();
        g.sign_only(
            &signer,
            "forfeit_milestone",
            args(&g, (id, a.clone(), 0u32, op.clone())),
        );
        fails_auth(g.events.try_forfeit_milestone(&id, &a, &0, &op));
    }
    let op = g.op();
    g.sign_only(
        &g.owner,
        "forfeit_milestone",
        args(&g, (id, a.clone(), 0u32, op.clone())),
    );
    g.events.forfeit_milestone(&id, &a, &0, &op);
    g.restore_auth();
    assert_eq!(g.owed(id), usdc(600));
}

#[test]
fn forfeits_check_the_same_things_releases_do() {
    let g = setup();
    let id = g.create(Spec::new(usdc(900), 3));
    let a = g.someone();
    fails_with(
        g.events.try_forfeit_milestone(&id, &a, &0, &g.op()),
        Error::NoSubmissions,
    );
    g.select(id, &[(a.clone(), 1, usdc(900))]);
    fails_with(
        g.events.try_forfeit_milestone(&id, &a, &3, &g.op()),
        Error::InvalidMilestone,
    );
    fails_with(
        g.events.try_forfeit_milestone(&99, &a, &0, &g.op()),
        Error::EventNotFound,
    );
    let op = g.op();
    g.events.forfeit_milestone(&id, &a, &0, &op);
    fails_with(
        g.events.try_forfeit_milestone(&id, &a, &1, &op),
        Error::OpAlreadySeen,
    );
    g.events.pause();
    fails_with(
        g.events.try_forfeit_milestone(&id, &a, &1, &g.op()),
        Error::Paused,
    );
}

#[test]
fn only_grants_forfeit() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(1_000));
    let mut params = g.params(&Spec::new(usdc(100), 1));
    params.pillar = Pillar::Hackathon;
    params.release_kind = ReleaseKind::Single;
    let hackathon = g.events.create_event(&params, &g.op());
    fails_with(
        g.events
            .try_forfeit_milestone(&hackathon, &g.someone(), &0, &g.op()),
        Error::InvalidPillar,
    );

    let mut params = g.params(&Spec::new(usdc(100), 2));
    params.pillar = Pillar::Crowdfunding;
    let campaign = g.events.create_event(&params, &g.op());
    fails_with(
        g.events
            .try_forfeit_milestone(&campaign, &g.owner, &0, &g.op()),
        Error::InvalidPillar,
    );
}

#[test]
fn nothing_settles_once_the_grant_is_closed() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(100))]);
    g.pay(id, &a, 0, usdc(100));
    fails_with(
        g.events.try_forfeit_milestone(&id, &a, &0, &g.op()),
        Error::EventNotActive,
    );
}
