use super::*;
use crate::event_ops::MAX_REFUNDS_PER_BATCH;
use crate::events::{ContributorRefunded, EventCancelled, OwnerResidualRefunded};

#[test]
fn cancel_before_selection_returns_the_budget_but_not_the_fee() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    g.events.start_cancel(&id, &g.op());
    g.capture();

    assert_eq!(g.status(id), EventStatus::Cancelled);
    assert_eq!(g.balance(&g.owner), usdc(1_000));
    assert_eq!(g.balance(&g.fee_account), usdc(25));
    assert!(g.emitted(OwnerResidualRefunded {
        event_id: id,
        owner: g.owner.clone(),
        amount: usdc(1_000),
    }));
    assert!(g.emitted(EventCancelled { id }));
    g.books();
}

#[test]
fn partners_are_refunded_in_full_before_the_owner() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let (p1, p2) = (g.someone(), g.someone());
    g.contribute(id, &p1, usdc(200));
    g.contribute(id, &p2, usdc(300));

    g.events.start_cancel(&id, &g.op());
    assert_eq!(g.status(id), EventStatus::Cancelling);
    assert_eq!(g.owed(id), 0);

    let left = g
        .events
        .process_cancel_batch(&id, &MAX_REFUNDS_PER_BATCH, &g.op());
    assert_eq!(left, 0);
    assert!(g.emitted(ContributorRefunded {
        event_id: id,
        contributor: p2.clone(),
        amount: usdc(300),
    }));
    g.events.finalize_cancel(&id, &g.op());

    assert_eq!(g.balance(&p1), usdc(200));
    assert_eq!(g.balance(&p2), usdc(300));
    assert_eq!(g.balance(&g.owner), usdc(1_000));
    assert_eq!(g.status(id), EventStatus::Cancelled);
    assert_eq!(g.escrow_balance(), 0);
    g.books();
}

#[test]
fn after_payouts_partners_share_what_is_left_pro_rata() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let (p1, p2) = (g.someone(), g.someone());
    g.contribute(id, &p1, usdc(300));
    g.contribute(id, &p2, usdc(200));

    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_400))]);
    g.pay(id, &a, 0, usdc(700));
    g.pay(id, &a, 1, usdc(700));
    // 100 left against 500 of partner money
    g.cancel(id);

    assert_eq!(g.balance(&p1), usdc(60));
    assert_eq!(g.balance(&p2), usdc(40));
    assert_eq!(g.balance(&g.owner), 0);
    assert_eq!(g.escrow_balance(), 0);
}

#[test]
fn pro_rata_rounding_dust_stays_in_the_contract() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let partners = [g.someone(), g.someone(), g.someone()];
    for p in partners.iter() {
        g.contribute(id, p, usdc(10));
    }
    let a = g.someone();
    // leave 20 USDC + 2 stroops against 30 USDC of partner money
    let award = usdc(110) - 2;
    g.select(id, &[(a.clone(), 1, award)]);
    g.pay(id, &a, 0, award);
    g.cancel(id);

    let each = usdc(10) * (usdc(20) + 2) / usdc(30);
    for p in partners.iter() {
        assert_eq!(g.balance(p), each);
    }
    let dust = usdc(20) + 2 - 3 * each;
    assert!(dust > 0);
    assert_eq!(g.escrow_balance(), dust);
}

#[test]
fn many_partners_are_refunded_across_batches() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let partners: std::vec::Vec<Address> = (0..30).map(|_| g.someone()).collect();
    for p in partners.iter() {
        g.contribute(id, p, usdc(10));
    }
    g.events.start_cancel(&id, &g.op());

    fails_with(
        g.events.try_finalize_cancel(&id, &g.op()),
        Error::CancellationNotFinished,
    );
    assert_eq!(
        g.events
            .process_cancel_batch(&id, &MAX_REFUNDS_PER_BATCH, &g.op()),
        30 - MAX_REFUNDS_PER_BATCH
    );
    // asking for more than the cap processes the cap
    assert_eq!(g.events.process_cancel_batch(&id, &100, &g.op()), 0);
    g.events.finalize_cancel(&id, &g.op());

    for p in partners.iter() {
        assert_eq!(g.balance(p), usdc(10));
    }
    assert_eq!(g.balance(&g.owner), usdc(100));
    assert_eq!(g.escrow_balance(), 0);
}

#[test]
fn nothing_is_paid_while_cancelling_or_after() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.contribute(id, &g.someone(), usdc(10));
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    g.pay(id, &a, 0, usdc(500));
    g.forfeit(id, &a, 1, usdc(500));

    g.events.start_cancel(&id, &g.op());
    fails_with(
        g.events.try_claim_milestone(&id, &a, &1, &0, &g.op()),
        Error::EventNotActive,
    );
    g.finish_cancel(id);
    fails_with(
        g.events.try_claim_milestone(&id, &a, &1, &0, &g.op()),
        Error::EventNotActive,
    );
    assert_eq!(g.balance(&a), usdc(500));
}

#[test]
fn a_completed_grant_cannot_be_cancelled() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(100))]);
    g.pay(id, &a, 0, usdc(100));
    fails_with(
        g.events.try_start_cancel(&id, &g.op()),
        Error::EventNotActive,
    );
}

#[test]
fn cancel_starts_once() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    g.contribute(id, &g.someone(), usdc(10));
    g.events.start_cancel(&id, &g.op());
    fails_with(
        g.events.try_start_cancel(&id, &g.op()),
        Error::EventNotActive,
    );
}

#[test]
fn crank_and_finalize_need_a_cancel_in_progress() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    fails_with(
        g.events.try_process_cancel_batch(&id, &25, &g.op()),
        Error::CancellationNotStarted,
    );
    fails_with(
        g.events.try_finalize_cancel(&id, &g.op()),
        Error::CancellationNotStarted,
    );
}

#[test]
fn residual_return_after_every_award_is_paid() {
    let g = setup();
    let id = g.create(Spec::new(usdc(20_000), 2).floors(&[(1, usdc(5_000)), (2, usdc(5_000))]));
    let (a, b) = (g.someone(), g.someone());
    g.select(
        id,
        &[(a.clone(), 1, usdc(6_000)), (b.clone(), 2, usdc(5_000))],
    );
    for m in 0..2 {
        g.pay(id, &a, m, usdc(3_000));
        g.pay(id, &b, m, usdc(2_500));
    }
    assert_eq!(g.status(id), EventStatus::Active);
    g.cancel(id);
    assert_eq!(g.balance(&g.owner), usdc(9_000));
}
