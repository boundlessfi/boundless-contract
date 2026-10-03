use soroban_sdk::testutils::Ledger as _;

use super::*;

fn awarded<'a>() -> (G<'a>, u64, Address) {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    (g, id, a)
}

#[test]
fn publishing_is_signed_by_the_owner_alone() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(2_000));
    g.events
        .create_event(&g.params(&Spec::new(usdc(1_000), 2)), &g.op());
    assert_eq!(g.signers(), std::vec![g.owner.clone()]);
}

#[test]
fn publishing_without_the_owner_fails() {
    let g = setup();
    g.token_admin.mint(&g.owner, &usdc(2_000));
    g.env.mock_auths(&[]);
    fails_auth(
        g.events
            .try_create_event(&g.params(&Spec::new(usdc(1_000), 2)), &g.op()),
    );
    g.restore_auth();
    assert_eq!(g.balance(&g.owner), usdc(2_000));
}

#[test]
fn milestone_release_is_signed_by_the_owner_alone() {
    let (g, id, a) = awarded();
    g.events.claim_milestone(&id, &a, &0, &0, &g.op());
    assert_eq!(g.signers(), std::vec![g.owner.clone()]);
}

#[test]
fn owner_signature_is_sufficient_to_release() {
    let (g, id, a) = awarded();
    let op = g.op();
    g.sign_only(
        &g.owner,
        "claim_milestone",
        args(&g, (id, a.clone(), 0u32, 0u32, op.clone())),
    );
    g.events.claim_milestone(&id, &a, &0, &0, &op);
    g.restore_auth();
    assert_eq!(g.balance(&a), usdc(500));
}

#[test]
fn recipient_cannot_release_their_own_milestone() {
    let (g, id, a) = awarded();
    let op = g.op();
    g.sign_only(
        &a,
        "claim_milestone",
        args(&g, (id, a.clone(), 0u32, 0u32, op.clone())),
    );
    fails_auth(g.events.try_claim_milestone(&id, &a, &0, &0, &op));
    g.restore_auth();
    assert_eq!(g.balance(&a), 0);
    assert!(!g.milestone_claimed(id, &a, 0));
}

#[test]
fn admin_cannot_release_a_milestone() {
    let (g, id, a) = awarded();
    let op = g.op();
    g.sign_only(
        &g.admin,
        "claim_milestone",
        args(&g, (id, a.clone(), 0u32, 0u32, op.clone())),
    );
    fails_auth(g.events.try_claim_milestone(&id, &a, &0, &0, &op));
    g.restore_auth();
    assert_eq!(g.balance(&a), 0);
}

#[test]
fn delegated_manager_cannot_release_a_milestone() {
    let g = setup();
    let manager = g.someone();
    let id = g.create(Spec::new(usdc(1_000), 2).manager(&manager));
    g.events.accept_manager(&id);
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);

    let op = g.op();
    g.sign_only(
        &manager,
        "claim_milestone",
        args(&g, (id, a.clone(), 0u32, 0u32, op.clone())),
    );
    fails_auth(g.events.try_claim_milestone(&id, &a, &0, &0, &op));
    g.restore_auth();
    g.pay(id, &a, 0, usdc(500));
}

#[test]
fn selection_is_signed_by_the_owner_until_a_manager_accepts() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    g.events
        .select_winners(&id, &g.specs(&[(g.someone(), 1, usdc(1_000))]), &g.op());
    assert_eq!(g.signers(), std::vec![g.owner.clone()]);
}

#[test]
fn after_handover_only_the_manager_selects_and_cancels() {
    let g = setup();
    let manager = g.someone();
    let id = g.create(Spec::new(usdc(1_000), 2).manager(&manager));
    g.events.accept_manager(&id);
    assert_eq!(g.signers(), std::vec![manager.clone()]);

    let a = g.someone();
    let specs = g.specs(&[(a.clone(), 1, usdc(1_000))]);
    let op = g.op();
    g.sign_only(
        &g.owner,
        "select_winners",
        args(&g, (id, specs.clone(), op.clone())),
    );
    fails_auth(g.events.try_select_winners(&id, &specs, &op));

    g.sign_only(
        &manager,
        "select_winners",
        args(&g, (id, specs.clone(), op.clone())),
    );
    g.events.select_winners(&id, &specs, &op);
    g.restore_auth();
    g.forfeit(id, &a, 0, usdc(500));
    g.forfeit(id, &a, 1, usdc(500));

    let op = g.op();
    g.sign_only(&g.owner, "start_cancel", args(&g, (id, op.clone())));
    fails_auth(g.events.try_start_cancel(&id, &op));

    g.restore_auth();
    g.events.start_cancel(&id, &op);
    assert_eq!(g.signers(), std::vec![manager.clone()]);
    assert_eq!(
        g.balance(&g.owner),
        usdc(1_000),
        "refunds still go to the owner"
    );
}

#[test]
fn a_stranger_cannot_select_or_cancel() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let stranger = g.someone();
    let specs = g.specs(&[(stranger.clone(), 1, usdc(1_000))]);
    let op = g.op();
    g.sign_only(
        &stranger,
        "select_winners",
        args(&g, (id, specs.clone(), op.clone())),
    );
    fails_auth(g.events.try_select_winners(&id, &specs, &op));
    g.sign_only(&stranger, "start_cancel", args(&g, (id, op.clone())));
    fails_auth(g.events.try_start_cancel(&id, &op));
    g.restore_auth();
    assert_eq!(g.owed(id), 0);
    assert_eq!(g.status(id), EventStatus::Active);
}

#[test]
fn a_top_up_is_signed_by_the_contributor() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let partner = g.someone();
    g.token_admin.mint(&partner, &usdc(100));
    g.events.add_funds(&id, &partner, &usdc(10), &g.op());
    assert_eq!(g.signers(), std::vec![partner.clone()]);

    let other = g.someone();
    let op = g.op();
    g.sign_only(
        &other,
        "add_funds",
        args(&g, (id, partner.clone(), usdc(10), op.clone())),
    );
    fails_auth(g.events.try_add_funds(&id, &partner, &usdc(10), &op));
    g.restore_auth();
}

#[test]
fn the_refund_crank_needs_no_signature() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let partner = g.someone();
    g.contribute(id, &partner, usdc(10));
    g.events.start_cancel(&id, &g.op());

    g.env.mock_auths(&[]);
    assert_eq!(g.events.process_cancel_batch(&id, &25, &g.op()), 0);
    g.events.finalize_cancel(&id, &g.op());
    g.restore_auth();
    assert_eq!(g.balance(&partner), usdc(10));
    assert_eq!(g.balance(&g.owner), usdc(1_000));
}

#[test]
fn manager_handover_needs_both_sides() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let manager = g.someone();

    g.sign_only(&manager, "propose_manager", args(&g, (id, manager.clone())));
    fails_auth(g.events.try_propose_manager(&id, &manager));

    g.sign_only(&g.owner, "propose_manager", args(&g, (id, manager.clone())));
    g.events.propose_manager(&id, &manager);

    g.sign_only(&g.owner, "accept_manager", args(&g, (id,)));
    fails_auth(g.events.try_accept_manager(&id));

    g.sign_only(&manager, "accept_manager", args(&g, (id,)));
    g.events.accept_manager(&id);
    g.restore_auth();
    assert_eq!(g.events.get_manager(&id), manager);
}

#[test]
fn an_unaccepted_proposal_expires() {
    let g = setup();
    let manager = g.someone();
    let id = g.create(Spec::new(usdc(1_000), 2).manager(&manager));
    let expires = g.events.get_pending_manager(&id).unwrap().expires_at_ledger;
    g.env.ledger().with_mut(|l| l.sequence_number = expires + 1);
    fails_with(
        g.events.try_accept_manager(&id),
        Error::PendingRotationExpired,
    );
    assert_eq!(g.events.get_manager(&id), g.owner);
}

#[test]
fn a_pending_proposal_can_be_withdrawn() {
    let g = setup();
    let manager = g.someone();
    let id = g.create(Spec::new(usdc(1_000), 2).manager(&manager));
    g.events.cancel_pending_manager(&id);
    assert_eq!(g.signers(), std::vec![g.owner.clone()]);
    fails_with(
        g.events.try_accept_manager(&id),
        Error::PendingRotationMismatch,
    );
}

#[test]
fn the_owner_takes_management_back_before_any_award() {
    let g = setup();
    let manager = g.someone();
    let id = g.create(Spec::new(usdc(1_000), 2).manager(&manager));
    g.events.accept_manager(&id);
    let next = g.someone();
    g.events.propose_manager(&id, &next);

    g.sign_only(&manager, "reclaim_management", args(&g, (id,)));
    fails_auth(g.events.try_reclaim_management(&id));
    g.sign_only(&g.owner, "reclaim_management", args(&g, (id,)));
    g.events.reclaim_management(&id);
    g.capture();
    g.restore_auth();

    assert_eq!(g.events.get_manager(&id), g.owner);
    assert!(g.events.get_pending_manager(&id).is_none());
    assert!(g.emitted(crate::events::ManagementReclaimed {
        event_id: id,
        owner: g.owner.clone(),
    }));
    fails_with(
        g.events.try_accept_manager(&id),
        Error::PendingRotationMismatch,
    );

    // the owner runs it from here, cancel included
    g.events.start_cancel(&id, &g.op());
    assert_eq!(g.signers(), std::vec![g.owner.clone()]);
    assert_eq!(g.balance(&g.owner), usdc(1_000));
}

#[test]
fn after_selection_management_stays_with_the_manager() {
    let g = setup();
    let manager = g.someone();
    let id = g.create(Spec::new(usdc(1_000), 2).manager(&manager));
    g.events.accept_manager(&id);
    g.select(id, &[(g.someone(), 1, usdc(1_000))]);
    fails_with(
        g.events.try_reclaim_management(&id),
        Error::WinnersAlreadySelected,
    );
    assert_eq!(g.events.get_manager(&id), manager);
}

#[test]
fn reclaiming_needs_an_open_event() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    g.cancel(id);
    fails_with(g.events.try_reclaim_management(&id), Error::EventNotActive);
}
