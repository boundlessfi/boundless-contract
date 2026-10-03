use super::*;
use crate::events::WinnersSelected;

#[test]
fn selection_reserves_each_award_and_moves_nothing() {
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

    assert_eq!(g.owed(id), usdc(25_000));
    assert_eq!(g.events.get_winner_count(&id), 3);
    let row = g.events.get_winner_at(&id, &1).unwrap();
    assert_eq!(row.recipient, b);
    assert_eq!(row.amount, usdc(8_000));
    assert_eq!(row.milestone, None);
    assert_eq!(row.paid_at, None);
    assert!(g.emitted(WinnersSelected {
        event_id: id,
        count: 3
    }));
}

#[test]
fn award_may_exceed_its_floor_out_of_headroom() {
    let g = setup();
    let id = g.create(Spec::new(usdc(20_000), 2).floors(&[(1, usdc(5_000)), (2, usdc(5_000))]));
    let (a, b) = (g.someone(), g.someone());
    g.select(id, &[(a, 1, usdc(12_000)), (b, 2, usdc(8_000))]);
    assert_eq!(g.owed(id), usdc(20_000));
}

#[test]
fn position_without_a_floor_takes_any_positive_amount() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1).floors(&[(1, usdc(500))]));
    let (a, b) = (g.someone(), g.someone());
    g.select(id, &[(a, 1, usdc(500)), (b, 7, usdc(500))]);
}

#[test]
fn award_below_its_floor_is_refused() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2).floors(&[(1, usdc(600))]));
    let a = g.someone();
    fails_with(
        g.events
            .try_select_winners(&id, &g.specs(&[(a, 1, usdc(599))]), &g.op()),
        Error::InvalidDistribution,
    );
    assert_eq!(g.owed(id), 0);
}

#[test]
fn zero_or_negative_award_is_refused() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1).floors(&[(1, usdc(1))]));
    for amount in [0, -usdc(1)] {
        fails_with(
            g.events
                .try_select_winners(&id, &g.specs(&[(g.someone(), 5, amount)]), &g.op()),
            Error::InvalidDistribution,
        );
    }
}

#[test]
fn awards_beyond_the_escrow_are_refused() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2).floors(&[(1, usdc(500))]));
    let (a, b) = (g.someone(), g.someone());
    fails_with(
        g.events.try_select_winners(
            &id,
            &g.specs(&[(a.clone(), 1, usdc(600)), (b.clone(), 2, usdc(401))]),
            &g.op(),
        ),
        Error::InsufficientEscrow,
    );
    g.select(id, &[(a, 1, usdc(600)), (b, 2, usdc(400))]);
}

#[test]
fn awards_overflowing_i128_are_refused_not_wrapped() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1).floors(&[(1, usdc(1))]));
    fails_with(
        g.events.try_select_winners(
            &id,
            &g.specs(&[(g.someone(), 1, i128::MAX), (g.someone(), 2, i128::MAX)]),
            &g.op(),
        ),
        Error::InsufficientEscrow,
    );
}

#[test]
fn empty_selection_is_refused() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1));
    fails_with(
        g.events.try_select_winners(&id, &g.specs(&[]), &g.op()),
        Error::NoSubmissions,
    );
}

#[test]
fn duplicate_position_is_refused() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1).floors(&[(1, usdc(100))]));
    fails_with(
        g.events.try_select_winners(
            &id,
            &g.specs(&[(g.someone(), 1, usdc(100)), (g.someone(), 1, usdc(100))]),
            &g.op(),
        ),
        Error::DuplicateWinnerPosition,
    );
}

#[test]
fn a_grant_takes_up_to_the_award_cap_in_its_one_selection() {
    let g = setup();
    let cap = crate::event_ops::MAX_GRANT_AWARDS;
    let id = g.create(Spec::new(usdc(cap as i128 + 1), 1).floors(&[(1, usdc(1))]));
    let mut awards = std::vec::Vec::new();
    for position in 1..=cap + 1 {
        awards.push((g.someone(), position, usdc(1)));
    }
    fails_with(
        g.events.try_select_winners(&id, &g.specs(&awards), &g.op()),
        Error::InvalidWinnerPosition,
    );
    awards.pop();
    g.select(id, &awards);
    assert_eq!(g.events.get_winner_count(&id), cap);
}

#[test]
fn selection_happens_once() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2).floors(&[(1, usdc(500))]));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(500))]);

    fails_with(
        g.events
            .try_select_winners(&id, &g.specs(&[(g.someone(), 2, usdc(500))]), &g.op()),
        Error::WinnersAlreadySelected,
    );

    g.pay(id, &a, 0, usdc(250));
    fails_with(
        g.events
            .try_select_winners(&id, &g.specs(&[(g.someone(), 2, usdc(500))]), &g.op()),
        Error::WinnersAlreadySelected,
    );
    assert_eq!(g.owed(id), usdc(250));
}

#[test]
fn a_floored_position_left_out_is_never_awarded_and_comes_back_on_cancel() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1).floors(&[(1, usdc(600)), (2, usdc(400))]));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(600))]);
    g.pay(id, &a, 0, usdc(600));

    assert_eq!(g.status(id), EventStatus::Active);
    assert_eq!(g.event(id).remaining_escrow, usdc(400));
    assert_eq!(g.owed(id), 0);

    g.cancel(id);
    assert_eq!(g.balance(&g.owner), usdc(400));
}

#[test]
fn selection_on_a_closed_grant_is_refused() {
    let g = setup();
    let done = g.create(Spec::new(usdc(100), 1));
    let a = g.someone();
    g.select(done, &[(a.clone(), 1, usdc(100))]);
    g.pay(done, &a, 0, usdc(100));
    assert_eq!(g.status(done), EventStatus::Completed);
    fails_with(
        g.events
            .try_select_winners(&done, &g.specs(&[(g.someone(), 2, usdc(1))]), &g.op()),
        Error::EventNotActive,
    );

    let cancelled = g.create(Spec::new(usdc(100), 1));
    g.cancel(cancelled);
    fails_with(
        g.events.try_select_winners(
            &cancelled,
            &g.specs(&[(g.someone(), 1, usdc(100))]),
            &g.op(),
        ),
        Error::EventNotActive,
    );
}

#[test]
fn selection_on_an_unknown_grant_is_refused() {
    let g = setup();
    fails_with(
        g.events
            .try_select_winners(&999, &g.specs(&[(g.someone(), 1, usdc(1))]), &g.op()),
        Error::EventNotFound,
    );
}

#[test]
fn selection_op_id_cannot_be_replayed() {
    let g = setup();
    let first = g.create(Spec::new(usdc(100), 1));
    let second = g.create(Spec::new(usdc(100), 1));
    let op = g.op();
    g.events
        .select_winners(&first, &g.specs(&[(g.someone(), 1, usdc(100))]), &op);
    fails_with(
        g.events
            .try_select_winners(&second, &g.specs(&[(g.someone(), 1, usdc(100))]), &op),
        Error::OpAlreadySeen,
    );
}
