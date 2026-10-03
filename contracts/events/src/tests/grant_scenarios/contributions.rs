use super::*;
use crate::events::FundsAdded;

#[test]
fn partner_money_before_selection_can_fund_bigger_awards() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2).floors(&[(1, usdc(500))]));
    let partner = g.someone();
    g.contribute(id, &partner, usdc(500));
    assert!(g.emitted(FundsAdded {
        event_id: id,
        contributor: partner.clone(),
        amount: usdc(500),
        new_remaining: usdc(1_500),
    }));

    let (a, b) = (g.someone(), g.someone());
    g.select(id, &[(a.clone(), 1, usdc(900)), (b.clone(), 2, usdc(600))]);
    g.pay(id, &a, 0, usdc(450));
    g.pay(id, &a, 1, usdc(450));
    g.pay(id, &b, 0, usdc(300));
    g.pay(id, &b, 1, usdc(300));
    assert_eq!(g.status(id), EventStatus::Completed);
    assert_eq!(g.escrow_balance(), 0);
}

#[test]
fn partners_are_tracked_and_the_owner_is_not() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1));
    let (p1, p2) = (g.someone(), g.someone());
    g.contribute(id, &p1, usdc(100));
    g.contribute(id, &p2, usdc(50));
    g.contribute(id, &p1, usdc(25));
    g.contribute(id, &g.owner, usdc(10));

    assert_eq!(g.events.get_contributor_count(&id), 2);
    assert_eq!(g.events.get_contributor_amount(&id, &p1), usdc(125));
    assert_eq!(g.events.get_contributor_amount(&id, &p2), usdc(50));
    assert_eq!(g.events.get_contributor_amount(&id, &g.owner), 0);
    assert_eq!(g.event(id).remaining_escrow, usdc(1_185));
    assert_eq!(
        g.event(id).total_budget,
        usdc(1_000),
        "budget is the publish amount"
    );
}

#[test]
fn contributions_below_ten_usdc_are_refused() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1));
    let partner = g.someone();
    g.token_admin.mint(&partner, &usdc(100));
    fails_with(
        g.events
            .try_add_funds(&id, &partner, &(usdc(10) - 1), &g.op()),
        Error::BelowMinimumContribution,
    );
    for amount in [0, -1] {
        fails_with(
            g.events.try_add_funds(&id, &partner, &amount, &g.op()),
            Error::InvalidContributionAmount,
        );
    }
    g.contribute(id, &partner, usdc(10));
}

#[test]
fn money_added_after_selection_cannot_be_awarded_and_comes_back_first() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);

    let partner = g.someone();
    g.contribute(id, &partner, usdc(300));
    g.pay(id, &a, 0, usdc(500));
    g.pay(id, &a, 1, usdc(500));

    assert_eq!(
        g.status(id),
        EventStatus::Active,
        "the top-up keeps it open"
    );
    assert_eq!(g.event(id).remaining_escrow, usdc(300));
    assert_eq!(g.owed(id), 0);

    g.cancel(id);
    assert_eq!(g.balance(&partner), usdc(300));
    assert_eq!(g.balance(&g.owner), 0);
}

#[test]
fn no_contributions_once_the_grant_is_closed() {
    let g = setup();
    let partner = g.someone();
    g.token_admin.mint(&partner, &usdc(1_000));

    let done = g.create(Spec::new(usdc(100), 1));
    let a = g.someone();
    g.select(done, &[(a.clone(), 1, usdc(100))]);
    g.pay(done, &a, 0, usdc(100));
    fails_with(
        g.events.try_add_funds(&done, &partner, &usdc(10), &g.op()),
        Error::EventNotActive,
    );

    let cancelled = g.create(Spec::new(usdc(100), 1));
    g.cancel(cancelled);
    fails_with(
        g.events
            .try_add_funds(&cancelled, &partner, &usdc(10), &g.op()),
        Error::EventNotActive,
    );

    let cancelling = g.create(Spec::new(usdc(100), 1));
    g.contribute(cancelling, &g.someone(), usdc(10));
    g.events.start_cancel(&cancelling, &g.op());
    assert_eq!(g.status(cancelling), EventStatus::Cancelling);
    fails_with(
        g.events
            .try_add_funds(&cancelling, &partner, &usdc(10), &g.op()),
        Error::EventNotActive,
    );

    fails_with(
        g.events.try_add_funds(&777, &partner, &usdc(10), &g.op()),
        Error::EventNotFound,
    );
}

#[test]
fn fee_override_applies_to_top_ups() {
    let g = setup();
    let free = g.create(Spec::new(usdc(1_000), 1).fee_override(0));
    let fee_before = g.balance(&g.fee_account);
    g.contribute(free, &g.someone(), usdc(100));
    assert_eq!(g.balance(&g.fee_account), fee_before);
}

#[test]
fn top_ups_follow_the_current_global_fee() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1));
    g.events.set_fee_bps(&500);
    let fee_before = g.balance(&g.fee_account);
    g.contribute(id, &g.someone(), usdc(100));
    assert_eq!(g.balance(&g.fee_account) - fee_before, usdc(5));
}

#[test]
fn top_up_op_id_cannot_be_replayed() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1));
    let partner = g.someone();
    g.token_admin.mint(&partner, &usdc(100));
    let op = g.op();
    g.events.add_funds(&id, &partner, &usdc(10), &op);
    fails_with(
        g.events.try_add_funds(&id, &partner, &usdc(10), &op),
        Error::OpAlreadySeen,
    );
}

#[test]
fn a_partner_without_the_funds_moves_nothing() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1));
    let partner = g.someone();
    g.token_admin.mint(&partner, &usdc(10));
    // the fee on top is missing
    assert!(g
        .events
        .try_add_funds(&id, &partner, &usdc(10), &g.op())
        .is_err());
    assert_eq!(g.balance(&partner), usdc(10));
    assert_eq!(g.events.get_contributor_count(&id), 0);
    assert_eq!(g.event(id).remaining_escrow, usdc(1_000));
}
