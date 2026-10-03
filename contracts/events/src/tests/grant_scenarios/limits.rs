//! The test host enforces mainnet's per-transaction limits, so each scenario
//! here proves the largest call the contract accepts still fits on chain.

use super::*;
use crate::event_ops::{
    MAX_AWARDS_PER_SELECT, MAX_GRANT_AWARDS, MAX_REFUNDS_PER_BATCH, VIEW_PAGE_LIMIT,
};

#[test]
fn the_largest_grant_selects_and_releases_every_milestone() {
    let g = setup();
    let n = 10u32;
    let people: std::vec::Vec<Address> = (0..MAX_GRANT_AWARDS).map(|_| g.someone()).collect();
    let award = usdc(10) + 3;
    let budget = award * MAX_GRANT_AWARDS as i128;
    let id = g.create(Spec::new(budget, n).floors(&[(1, award)]));
    let awards: std::vec::Vec<(Address, u32, i128)> = people
        .iter()
        .enumerate()
        .map(|(i, p)| (p.clone(), i as u32 + 1, award))
        .collect();
    g.select(id, &awards);

    let mut heaviest = 0;
    for m in 0..n {
        for p in people.iter() {
            g.events.claim_milestone(&id, p, &m, &5, &g.op());
            let r = g.env.cost_estimate().resources();
            heaviest = heaviest.max(r.memory_read_entries + r.write_entries);
        }
    }
    assert!(
        heaviest <= 60,
        "heaviest release touched {heaviest} entries"
    );
    for p in people.iter() {
        assert_eq!(g.balance(p), award);
    }
    assert_eq!(g.status(id), EventStatus::Completed);
    g.books();
}

#[test]
fn a_full_single_release_batch_fits() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(1_000));
    let mut params = g.params(&Spec::new(usdc(100), 1).floors(&[(1, 1)]));
    params.pillar = Pillar::Hackathon;
    params.release_kind = ReleaseKind::Single;
    let id = g.events.create_event(&params, &g.op());

    let mut awards: std::vec::Vec<(Address, u32, i128)> = (1..=MAX_AWARDS_PER_SELECT + 1)
        .map(|p| (g.someone(), p, usdc(1)))
        .collect();
    fails_with(
        g.events.try_select_winners(&id, &g.specs(&awards), &g.op()),
        Error::InvalidWinnerPosition,
    );
    awards.pop();
    g.events.select_winners(&id, &g.specs(&awards), &g.op());

    let next: std::vec::Vec<(Address, u32, i128)> = (MAX_AWARDS_PER_SELECT + 1
        ..=2 * MAX_AWARDS_PER_SELECT)
        .map(|p| (g.someone(), p, usdc(1)))
        .collect();
    g.events.select_winners(&id, &g.specs(&next), &g.op());
    assert_eq!(g.events.get_winner_count(&id), 2 * MAX_AWARDS_PER_SELECT);
}

#[test]
fn a_full_refund_batch_fits() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let partners: std::vec::Vec<Address> =
        (0..MAX_REFUNDS_PER_BATCH).map(|_| g.someone()).collect();
    for p in partners.iter() {
        g.contribute(id, p, usdc(10));
    }
    g.events.start_cancel(&id, &g.op());
    assert_eq!(
        g.events
            .process_cancel_batch(&id, &MAX_REFUNDS_PER_BATCH, &g.op()),
        0
    );
    g.events.finalize_cancel(&id, &g.op());
    for p in partners.iter() {
        assert_eq!(g.balance(p), usdc(10));
    }
}

#[test]
fn a_full_page_of_winners_can_be_read() {
    let g = setup();
    let people: std::vec::Vec<Address> = (0..MAX_GRANT_AWARDS).map(|_| g.someone()).collect();
    let id = g.create(Spec::new(usdc(400), 3).floors(&[(1, usdc(10))]));
    let awards: std::vec::Vec<(Address, u32, i128)> = people
        .iter()
        .enumerate()
        .map(|(i, p)| (p.clone(), i as u32 + 1, usdc(10)))
        .collect();
    g.select(id, &awards);
    for m in 0..3 {
        for p in people.iter() {
            g.events.claim_milestone(&id, p, &m, &0, &g.op());
        }
    }
    let rows = g.events.get_winner_count(&id);
    assert_eq!(rows, 4 * MAX_GRANT_AWARDS);

    let page = g.events.get_winners_page(&id, &0, &VIEW_PAGE_LIMIT);
    assert_eq!(page.len(), VIEW_PAGE_LIMIT);
    let all = g.events.get_winners(&id);
    assert_eq!(all.len(), VIEW_PAGE_LIMIT);
    let tail = g
        .events
        .get_winners_page(&id, &(rows - 5), &VIEW_PAGE_LIMIT);
    assert_eq!(tail.len(), 5);
}

#[test]
fn a_full_page_of_contributors_can_be_read() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    for _ in 0..VIEW_PAGE_LIMIT + 5 {
        g.contribute(id, &g.someone(), usdc(10));
    }
    let page = g.events.get_contributors_page(&id, &0, &VIEW_PAGE_LIMIT);
    assert_eq!(page.len(), VIEW_PAGE_LIMIT);
    assert_eq!(g.events.get_contributors(&id).len(), VIEW_PAGE_LIMIT);
}
