use soroban_sdk::testutils::Ledger as _;

use super::*;
use crate::events::MilestoneClaimed;

#[test]
fn even_split_pays_equal_parts_and_completes() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);

    g.pay(id, &a, 0, usdc(500));
    assert_eq!(g.status(id), EventStatus::Active);
    assert!(g.emitted(MilestoneClaimed {
        event_id: id,
        recipient: a.clone(),
        milestone: 0,
        amount: usdc(500),
    }));

    g.pay(id, &a, 1, usdc(500));
    assert_eq!(g.status(id), EventStatus::Completed);
    assert_eq!(g.balance(&a), usdc(1_000));
    assert_eq!(g.escrow_balance(), 0);
}

#[test]
fn one_milestone_pays_the_whole_award_at_once() {
    let g = setup();
    let id = g.create(Spec::new(usdc(200), 1));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(200))]);
    g.pay(id, &a, 0, usdc(200));
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn rounding_remainder_goes_to_the_last_payment() {
    let g = setup();
    // 100 USDC + 1 stroop over 3 milestones: 333333336, 333333336, 333333337... floored
    let award = usdc(100) + 1;
    let id = g.create(Spec::new(award, 3));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, award)]);

    let part = award / 3;
    g.pay(id, &a, 0, part);
    g.pay(id, &a, 1, part);
    g.pay(id, &a, 2, award - 2 * part);
    assert_eq!(g.balance(&a), award);
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn last_payment_is_decided_by_count_not_index() {
    let g = setup();
    let award = usdc(10) + 2;
    let id = g.create(Spec::new(award, 3));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, award)]);

    let part = award / 3;
    g.pay(id, &a, 2, part);
    g.pay(id, &a, 0, part);
    g.pay(id, &a, 1, award - 2 * part);
    assert_eq!(g.balance(&a), award);
}

#[test]
fn ten_milestones_with_an_awkward_amount_pay_exactly() {
    let g = setup();
    let award = usdc(6_000) + 7;
    let id = g.create(Spec::new(award, 10));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, award)]);
    let part = award / 10;
    for m in 0..9 {
        g.pay(id, &a, m, part);
    }
    g.pay(id, &a, 9, award - 9 * part);
    assert_eq!(g.balance(&a), award);
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn tiered_recipients_are_paid_independently_and_interleaved() {
    let g = setup();
    let id = g.create(Spec::new(usdc(25_000), 3).floors(&[
        (1, usdc(12_000)),
        (2, usdc(8_000)),
        (3, usdc(5_000)),
    ]));
    let (a, b, c) = (g.someone(), g.someone(), g.someone());
    g.select(
        id,
        &[
            (a.clone(), 1, usdc(12_000)),
            (b.clone(), 2, usdc(8_000)),
            (c.clone(), 3, usdc(5_000)),
        ],
    );

    g.pay(id, &b, 0, usdc(8_000) / 3);
    g.pay(id, &a, 0, usdc(4_000));
    g.pay(id, &c, 1, usdc(5_000) / 3);
    g.pay(id, &b, 2, usdc(8_000) / 3);
    g.pay(id, &a, 1, usdc(4_000));
    g.pay(id, &c, 0, usdc(5_000) / 3);
    g.pay(id, &a, 2, usdc(4_000));
    assert_eq!(g.status(id), EventStatus::Active);
    g.pay(id, &c, 2, usdc(5_000) - 2 * (usdc(5_000) / 3));
    g.pay(id, &b, 1, usdc(8_000) - 2 * (usdc(8_000) / 3));

    assert_eq!(g.balance(&a), usdc(12_000));
    assert_eq!(g.balance(&b), usdc(8_000));
    assert_eq!(g.balance(&c), usdc(5_000));
    assert_eq!(g.status(id), EventStatus::Completed);
    assert_eq!(g.escrow_balance(), 0);
}

#[test]
fn every_payment_is_recorded_as_a_paid_row() {
    let g = setup();
    let id = g.create(Spec::new(usdc(900), 3));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(900))]);
    g.env.ledger().with_mut(|l| l.timestamp = 1_000);
    g.pay(id, &a, 1, usdc(300));

    assert_eq!(g.events.get_winner_count(&id), 2);
    let paid = g.events.get_winner_at(&id, &1).unwrap();
    assert_eq!(paid.recipient, a);
    assert_eq!(paid.position, 1);
    assert_eq!(paid.amount, usdc(300));
    assert_eq!(paid.milestone, Some(1));
    assert_eq!(paid.paid_at, Some(1_000));
}

#[test]
fn payouts_register_earnings_and_reputation_on_the_profile() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    g.events.claim_milestone(&id, &a, &0, &10, &g.op());
    g.events.claim_milestone(&id, &a, &1, &10, &g.op());

    let profile = g.profile.get_profile(&a).unwrap();
    assert_eq!(profile.reputation, 20);
    assert_eq!(g.profile.get_earnings(&a, &g.token.address), usdc(1_000));
}

#[test]
fn the_same_milestone_cannot_be_paid_twice() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    g.pay(id, &a, 0, usdc(500));
    fails_with(
        g.events.try_claim_milestone(&id, &a, &0, &0, &g.op()),
        Error::MilestoneAlreadyClaimed,
    );
    assert_eq!(g.balance(&a), usdc(500));
}

#[test]
fn milestone_index_must_be_in_range() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    for m in [2, u32::MAX] {
        fails_with(
            g.events.try_claim_milestone(&id, &a, &m, &0, &g.op()),
            Error::InvalidMilestone,
        );
    }
}

#[test]
fn only_a_selected_recipient_can_be_paid() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();

    fails_with(
        g.events.try_claim_milestone(&id, &a, &0, &0, &g.op()),
        Error::NoSubmissions,
    );

    g.select(id, &[(a, 1, usdc(1_000))]);
    fails_with(
        g.events
            .try_claim_milestone(&id, &g.someone(), &0, &0, &g.op()),
        Error::NoSubmissions,
    );
    assert_eq!(g.owed(id), usdc(1_000));
}

#[test]
fn claim_op_id_cannot_be_replayed() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    let op = g.op();
    g.events.claim_milestone(&id, &a, &0, &0, &op);
    fails_with(
        g.events.try_claim_milestone(&id, &a, &1, &0, &op),
        Error::OpAlreadySeen,
    );
}

#[test]
fn nothing_can_be_paid_after_completion() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(100))]);
    g.pay(id, &a, 0, usdc(100));
    fails_with(
        g.events.try_claim_milestone(&id, &a, &0, &0, &g.op()),
        Error::EventNotActive,
    );
}

#[test]
fn claim_prize_is_not_a_grant_path() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let a = g.someone();
    g.select(id, &[(a, 1, usdc(100))]);
    fails_with(
        g.events.try_claim_prize(&id, &1, &g.op()),
        Error::InvalidReleaseKind,
    );
}

#[test]
fn unknown_grant_is_refused() {
    let g = setup();
    fails_with(
        g.events
            .try_claim_milestone(&404, &g.someone(), &0, &0, &g.op()),
        Error::EventNotFound,
    );
}

#[test]
fn a_rejected_milestone_is_forfeited_and_comes_back_on_close() {
    let g = setup();
    let award = usdc(10) + 1;
    let id = g.create(Spec::new(award, 3));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, award)]);

    let part = award / 3;
    g.pay(id, &a, 0, part);
    g.forfeit(id, &a, 1, part);
    // the last milestone settled still takes the rounding remainder
    g.pay(id, &a, 2, award - 2 * part);
    assert_eq!(g.owed(id), 0);
    assert_eq!(
        g.status(id),
        EventStatus::Active,
        "the forfeited share is still held"
    );

    g.cancel(id);
    assert_eq!(g.balance(&g.owner), part);
    assert_eq!(g.balance(&a), award - part);
}
