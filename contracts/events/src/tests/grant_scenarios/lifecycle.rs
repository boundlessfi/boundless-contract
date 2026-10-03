//! The grant shapes the organizer wizard offers, run start to finish the way
//! the backend drives them.

use super::*;

#[test]
fn micro_grants_five_recipients_paid_in_one_go() {
    let g = setup();
    let floors: std::vec::Vec<(u32, i128)> = (1..=5).map(|p| (p, usdc(200))).collect();
    let id = g.create(Spec::new(usdc(1_000), 1).floors(&floors));
    let people: std::vec::Vec<Address> = (0..5).map(|_| g.someone()).collect();
    let awards: std::vec::Vec<(Address, u32, i128)> = people
        .iter()
        .enumerate()
        .map(|(i, p)| (p.clone(), i as u32 + 1, usdc(200)))
        .collect();
    g.select(id, &awards);
    for p in people.iter() {
        g.pay(id, p, 0, usdc(200));
    }
    assert_eq!(g.status(id), EventStatus::Completed);
    assert_eq!(g.escrow_balance(), 0);
}

#[test]
fn builder_grant_with_a_rejected_milestone_and_a_returned_remainder() {
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
    let b_part = usdc(8_000) / 3;
    let c_part = usdc(5_000) / 3;
    for m in 0..3 {
        g.pay(id, &a, m, usdc(4_000));
    }
    g.pay(id, &b, 0, b_part);
    g.pay(id, &b, 1, b_part);
    g.pay(id, &b, 2, usdc(8_000) - 2 * b_part);
    // c delivers the first milestone, then the organizer rejects the rest
    g.pay(id, &c, 0, c_part);
    g.forfeit(id, &c, 1, c_part);
    g.forfeit(id, &c, 2, usdc(5_000) - 2 * c_part);

    assert_eq!(g.status(id), EventStatus::Active);
    assert_eq!(g.owed(id), 0);
    g.cancel(id);
    assert_eq!(g.balance(&g.owner), usdc(5_000) - c_part);
    assert_eq!(g.balance(&a), usdc(12_000));
    assert_eq!(g.balance(&b), usdc(8_000));
    assert_eq!(g.balance(&c), c_part);
}

#[test]
fn infrastructure_grant_topped_up_by_a_partner_mid_way() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100_000), 5));
    let partner = g.someone();
    g.contribute(id, &partner, usdc(20_000));

    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(110_000))]);
    for m in 0..5 {
        g.pay(id, &a, m, usdc(22_000));
    }
    assert_eq!(g.balance(&a), usdc(110_000));
    assert_eq!(g.event(id).remaining_escrow, usdc(10_000));

    g.cancel(id);
    assert_eq!(
        g.balance(&partner),
        usdc(10_000),
        "partner shares the shortfall"
    );
    assert_eq!(g.balance(&g.owner), 0);
}

#[test]
fn research_grant_awards_above_floors_from_headroom() {
    let g = setup();
    let id = g.create(Spec::new(usdc(20_000), 2).floors(&[
        (1, usdc(5_000)),
        (2, usdc(5_000)),
        (3, usdc(5_000)),
    ]));
    let people = [g.someone(), g.someone(), g.someone()];
    g.select(
        id,
        &[
            (people[0].clone(), 1, usdc(7_000)),
            (people[1].clone(), 2, usdc(6_500)),
            (people[2].clone(), 3, usdc(6_500)),
        ],
    );
    for p in people.iter() {
        let award = if *p == people[0] {
            usdc(7_000)
        } else {
            usdc(6_500)
        };
        g.pay(id, p, 1, award / 2);
        g.pay(id, p, 0, award - award / 2);
    }
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn org_treasury_funds_and_the_org_wallet_runs_it() {
    let g = setup();
    let org_wallet = g.someone();
    let id = g.create(Spec::new(usdc(6_000), 2).floors(&[
        (1, usdc(1_500)),
        (2, usdc(1_500)),
        (3, usdc(1_500)),
        (4, usdc(1_500)),
    ]));
    g.events.propose_manager(&id, &org_wallet);
    g.events.accept_manager(&id);

    let people: std::vec::Vec<Address> = (0..4).map(|_| g.someone()).collect();
    let awards: std::vec::Vec<(Address, u32, i128)> = people
        .iter()
        .enumerate()
        .map(|(i, p)| (p.clone(), i as u32 + 1, usdc(1_500)))
        .collect();
    g.events.select_winners(&id, &g.specs(&awards), &g.op());
    assert_eq!(g.signers(), std::vec![org_wallet.clone()]);

    for p in people.iter().take(3) {
        g.pay(id, p, 0, usdc(750));
        g.pay(id, p, 1, usdc(750));
    }
    g.pay(id, &people[3], 0, usdc(750));
    fails_with(
        g.events.try_start_cancel(&id, &g.op()),
        Error::AwardsOutstanding,
    );
    g.forfeit(id, &people[3], 1, usdc(750));
    g.events.start_cancel(&id, &g.op());
    assert_eq!(g.signers(), std::vec![org_wallet.clone()]);
    assert_eq!(g.balance(&g.owner), usdc(750));
    assert_eq!(
        g.balance(&org_wallet),
        0,
        "the manager never receives funds"
    );
    g.books();
}

#[test]
fn many_grants_on_one_contract_keep_separate_books() {
    let g = setup();
    let first = g.create(Spec::new(usdc(1_000), 2));
    let second = g.create(Spec::new(usdc(3_000), 3));
    let a = g.someone();
    let b = g.someone();
    g.select(first, &[(a.clone(), 1, usdc(1_000))]);
    g.select(second, &[(a.clone(), 1, usdc(3_000))]);

    fails_with(
        g.events.try_claim_milestone(&first, &b, &0, &0, &g.op()),
        Error::NoSubmissions,
    );
    g.pay(first, &a, 0, usdc(500));
    g.pay(second, &a, 0, usdc(1_000));
    g.forfeit(first, &a, 1, usdc(500));
    g.cancel(first);
    assert_eq!(g.event(second).remaining_escrow, usdc(2_000));
    g.pay(second, &a, 1, usdc(1_000));
    g.pay(second, &a, 2, usdc(1_000));
    assert_eq!(g.balance(&a), usdc(3_500));
    assert_eq!(g.balance(&g.owner), usdc(500));
    assert_eq!(g.escrow_balance(), 0);
}
