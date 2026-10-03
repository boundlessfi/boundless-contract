//! Threat-model findings on the grant path. Each test states the behaviour
//! the fix guarantees; the mapping to findings lives in docs/threat-model.md.

use super::*;
use crate::event_ops::MAX_REPUTATION_BUMP;
use crate::events::{ManagerProposed, PendingManagerCancelled};

#[test]
fn one_recipient_cannot_hold_two_awards_in_a_grant() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2).floors(&[(1, usdc(300)), (2, usdc(400))]));
    let a = g.someone();
    fails_with(
        g.events.try_select_winners(
            &id,
            &g.specs(&[(a.clone(), 1, usdc(300)), (a.clone(), 2, usdc(400))]),
            &g.op(),
        ),
        Error::DuplicateRecipient,
    );
    assert_eq!(g.owed(id), 0);
    assert_eq!(g.events.get_winner_count(&id), 0);
}

#[test]
fn an_award_too_small_to_split_is_refused() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 10).floors(&[(1, 1)]));
    fails_with(
        g.events
            .try_select_winners(&id, &g.specs(&[(g.someone(), 1, 9)]), &g.op()),
        Error::InvalidDistribution,
    );
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, 10)]);
    for m in 0..10 {
        g.pay(id, &a, m, 1);
    }
}

#[test]
fn a_release_costs_the_same_however_large_the_grant_grows() {
    let g = setup();
    let n = 4u32;
    let people: std::vec::Vec<Address> = (0..crate::event_ops::MAX_GRANT_AWARDS)
        .map(|_| g.someone())
        .collect();
    let id = g.create(Spec::new(usdc(4_000), n).floors(&[(1, usdc(100))]));
    let awards: std::vec::Vec<(Address, u32, i128)> = people
        .iter()
        .enumerate()
        .map(|(i, p)| (p.clone(), i as u32 + 1, usdc(100)))
        .collect();
    g.select(id, &awards);

    let mut first = None;
    let mut last = None;
    for m in 0..n {
        for p in people.iter() {
            g.events.claim_milestone(&id, p, &m, &0, &g.op());
            let reads = g.env.cost_estimate().resources().memory_read_entries;
            first.get_or_insert(reads);
            last = Some(reads);
        }
    }
    let (first, last) = (first.unwrap(), last.unwrap());
    assert!(
        last <= first + 2,
        "the 160th release read {last} entries against {first} for the first"
    );
    assert!(
        last < 45,
        "a release must stay far inside the read limit: {last}"
    );
    assert_eq!(g.status(id), EventStatus::Completed);
    for p in people.iter() {
        assert_eq!(g.balance(p), usdc(100));
    }
}

#[test]
fn a_paused_profile_does_not_hold_up_payouts() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);

    g.profile.pause();
    g.events.claim_milestone(&id, &a, &0, &10, &g.op());
    assert_eq!(g.balance(&a), usdc(500));
    assert!(g.profile.get_profile(&a).is_none());

    g.profile.unpause();
    g.events.claim_milestone(&id, &a, &1, &10, &g.op());
    assert_eq!(g.balance(&a), usdc(1_000));
    assert_eq!(g.profile.get_profile(&a).unwrap().reputation, 10);
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn a_broken_profile_binding_does_not_hold_up_payouts() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    g.events.set_profile_contract(&g.someone());
    g.pay(id, &a, 0, usdc(1_000));
}

#[test]
fn someone_reusing_an_op_id_cannot_block_or_strip_another_payout() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);

    let attacker = g.someone();
    g.token_admin.mint(&attacker, &usdc(20));
    let mut params = g.params(&Spec::new(usdc(10), 1));
    params.owner = attacker.clone();
    let own = g.events.create_event(&params, &g.op());
    g.events
        .select_winners(&own, &g.specs(&[(attacker.clone(), 1, usdc(10))]), &g.op());

    let shared = g.op();
    g.events.claim_milestone(&own, &attacker, &0, &5, &shared);
    g.events.claim_milestone(&id, &a, &0, &5, &shared);

    assert_eq!(g.balance(&a), usdc(500));
    assert_eq!(g.profile.get_profile(&a).unwrap().reputation, 5);
    assert_eq!(g.profile.get_earnings(&a, &g.token.address), usdc(500));
}

#[test]
fn reputation_per_award_and_per_release_is_capped() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();

    let mut specs = g.specs(&[(a.clone(), 1, usdc(1_000))]);
    let mut spec = specs.get(0).unwrap();
    spec.reputation_bump = MAX_REPUTATION_BUMP + 1;
    specs.set(0, spec.clone());
    fails_with(
        g.events.try_select_winners(&id, &specs, &g.op()),
        Error::ReputationBumpTooLarge,
    );
    spec.reputation_bump = MAX_REPUTATION_BUMP;
    specs.set(0, spec);
    g.events.select_winners(&id, &specs, &g.op());

    fails_with(
        g.events
            .try_claim_milestone(&id, &a, &0, &(MAX_REPUTATION_BUMP + 1), &g.op()),
        Error::ReputationBumpTooLarge,
    );
    fails_with(
        g.events
            .try_claim_milestone(&id, &a, &0, &u32::MAX, &g.op()),
        Error::ReputationBumpTooLarge,
    );
    g.events
        .claim_milestone(&id, &a, &0, &MAX_REPUTATION_BUMP, &g.op());
    assert_eq!(
        g.profile.get_profile(&a).unwrap().reputation,
        MAX_REPUTATION_BUMP as u64
    );
}

#[test]
fn an_empty_refund_batch_cannot_burn_the_crank_op_id() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 1));
    let partner = g.someone();
    g.contribute(id, &partner, usdc(10));
    g.events.start_cancel(&id, &g.op());

    let op = g.op();
    fails_with(
        g.events.try_process_cancel_batch(&id, &0, &op),
        Error::InvalidBatchSize,
    );
    assert_eq!(g.events.process_cancel_batch(&id, &25, &op), 0);
    g.events.finalize_cancel(&id, &g.op());
    assert_eq!(g.balance(&partner), usdc(10));
}

#[test]
fn replacing_a_pending_manager_is_on_the_record() {
    let g = setup();
    let first = g.someone();
    let id = g.create(Spec::new(usdc(1_000), 1).manager(&first));
    let second = g.someone();
    g.events.propose_manager(&id, &second);

    assert!(g.emitted(PendingManagerCancelled { event_id: id }));
    let expires = g.events.get_pending_manager(&id).unwrap().expires_at_ledger;
    assert!(g.emitted(ManagerProposed {
        event_id: id,
        target: second,
        expires_at_ledger: expires,
    }));
}

#[test]
fn a_frozen_recipient_is_not_marked_paid_and_can_be_paid_later() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    g.token_admin.set_authorized(&a, &false);

    assert!(g
        .events
        .try_claim_milestone(&id, &a, &0, &0, &g.op())
        .is_err());
    assert!(!g.milestone_claimed(id, &a, 0));
    assert_eq!(g.owed(id), usdc(1_000));
    assert_eq!(g.event(id).remaining_escrow, usdc(1_000));

    g.token_admin.set_authorized(&a, &true);
    g.pay(id, &a, 0, usdc(500));
}

#[test]
fn a_frozen_partner_cannot_stall_everyone_else_s_refund() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let (p1, frozen, p3) = (g.someone(), g.someone(), g.someone());
    g.contribute(id, &p1, usdc(100));
    g.contribute(id, &frozen, usdc(100));
    g.contribute(id, &p3, usdc(100));
    g.token_admin.set_authorized(&frozen, &false);

    g.cancel(id);
    assert!(g.emitted(crate::events::RefundDeferred {
        event_id: id,
        contributor: frozen.clone(),
        amount: usdc(100),
    }));
    assert_eq!(g.balance(&p1), usdc(100));
    assert_eq!(g.balance(&p3), usdc(100));
    assert_eq!(g.balance(&g.owner), usdc(1_000));
    assert_eq!(g.events.get_unclaimed_refund(&id, &frozen), usdc(100));
    assert_eq!(g.escrow_balance(), usdc(100), "held for the frozen partner");

    assert!(g.events.try_claim_refund(&id, &frozen, &g.op()).is_err());
    g.token_admin.set_authorized(&frozen, &true);

    let op = g.op();
    g.sign_only(
        &p1,
        "claim_refund",
        args(&g, (id, frozen.clone(), op.clone())),
    );
    fails_auth(g.events.try_claim_refund(&id, &frozen, &op));
    g.restore_auth();

    g.events.claim_refund(&id, &frozen, &g.op());
    assert_eq!(g.signers(), std::vec![frozen.clone()]);
    assert_eq!(g.balance(&frozen), usdc(100));
    assert_eq!(g.escrow_balance(), 0);
    fails_with(
        g.events.try_claim_refund(&id, &frozen, &g.op()),
        Error::NoRefundOwed,
    );
    fails_with(
        g.events.try_claim_refund(&id, &p1, &g.op()),
        Error::NoRefundOwed,
    );
}

#[test]
fn each_award_and_each_cancel_branch_is_on_the_record() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let partners = [g.someone(), g.someone(), g.someone()];
    for p in partners.iter() {
        g.contribute(id, p, usdc(10));
    }
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(130) - 2)]);
    assert!(g.emitted(crate::events::WinnerAwarded {
        event_id: id,
        recipient: a.clone(),
        position: 1,
        amount: usdc(130) - 2,
    }));
    g.pay(id, &a, 0, usdc(130) - 2);

    g.events.start_cancel(&id, &g.op());
    assert!(g.emitted(crate::events::CancellationStarted {
        event_id: id,
        branch: crate::types::CancellationBranch::ProRataPartners,
        remaining: 2,
        non_owner_total: usdc(30),
    }));
    g.finish_cancel(id);
    for p in partners.iter() {
        assert!(g.emitted(crate::events::ContributorRefunded {
            event_id: id,
            contributor: p.clone(),
            amount: 0,
        }));
    }
    assert_eq!(
        g.escrow_balance(),
        2,
        "two stroops of dust, each share floored to zero"
    );
}

#[test]
fn a_partner_who_dropped_their_trustline_cannot_stall_the_others() {
    use super::refusing_token::{RefusingToken, RefusingTokenClient};

    let g = setup();
    let token_id = g.env.register(RefusingToken, ());
    let token = RefusingTokenClient::new(&g.env, &token_id);
    g.events.register_supported_token(&token_id);

    let budget = usdc(1_000);
    token.mint(&g.owner, &(budget + fee_on(budget, FEE_BPS)));
    let mut params = g.params(&Spec::new(budget, 2));
    params.token = token_id.clone();
    let id = g.events.create_event(&params, &g.op());

    let (p1, gone, p3) = (g.someone(), g.someone(), g.someone());
    for p in [&p1, &gone, &p3] {
        token.mint(p, &(usdc(100) + fee_on(usdc(100), FEE_BPS)));
        g.events.add_funds(&id, p, &usdc(100), &g.op());
    }
    token.refuse(&gone, &true);

    g.events.start_cancel(&id, &g.op());
    assert_eq!(g.events.process_cancel_batch(&id, &15, &g.op()), 0);
    assert!(g.emitted(crate::events::RefundDeferred {
        event_id: id,
        contributor: gone.clone(),
        amount: usdc(100),
    }));
    g.events.finalize_cancel(&id, &g.op());

    assert_eq!(token.balance(&p1), usdc(100));
    assert_eq!(token.balance(&p3), usdc(100));
    assert_eq!(token.balance(&g.owner), budget);
    assert_eq!(token.balance(&gone), 0);
    assert_eq!(g.events.get_unclaimed_refund(&id, &gone), usdc(100));
    assert_eq!(token.balance(&g.events_id), usdc(100), "held for them");

    assert!(g.events.try_claim_refund(&id, &gone, &g.op()).is_err());
    assert_eq!(g.events.get_unclaimed_refund(&id, &gone), usdc(100));

    token.refuse(&gone, &false);
    g.events.claim_refund(&id, &gone, &g.op());
    assert_eq!(token.balance(&gone), usdc(100));
    assert_eq!(token.balance(&g.events_id), 0);
}
