use super::*;

#[test]
fn pause_stops_every_grant_operation_and_unpause_resumes_them() {
    let g = setup();
    let id = g.create(Spec::new(usdc(1_000), 2));
    let a = g.someone();
    g.select(id, &[(a.clone(), 1, usdc(1_000))]);
    g.pay(id, &a, 0, usdc(500));
    let partner = g.someone();
    g.token_admin.mint(&partner, &usdc(100));
    g.token_admin.mint(&g.owner, &usdc(2_000));
    let manager = g.someone();

    g.events.pause();
    assert!(g.events.is_paused());

    fails_with(
        g.events
            .try_create_event(&g.params(&Spec::new(usdc(1_000), 2)), &g.op()),
        Error::Paused,
    );
    fails_with(
        g.events.try_add_funds(&id, &partner, &usdc(10), &g.op()),
        Error::Paused,
    );
    fails_with(
        g.events
            .try_select_winners(&id, &g.specs(&[(g.someone(), 2, usdc(1))]), &g.op()),
        Error::Paused,
    );
    fails_with(
        g.events.try_claim_milestone(&id, &a, &1, &0, &g.op()),
        Error::Paused,
    );
    fails_with(g.events.try_start_cancel(&id, &g.op()), Error::Paused);
    fails_with(
        g.events.try_process_cancel_batch(&id, &25, &g.op()),
        Error::Paused,
    );
    fails_with(g.events.try_finalize_cancel(&id, &g.op()), Error::Paused);
    fails_with(g.events.try_propose_manager(&id, &manager), Error::Paused);
    fails_with(g.events.try_accept_manager(&id), Error::Paused);
    fails_with(g.events.try_cancel_pending_manager(&id), Error::Paused);

    // reads keep working for the indexer and the UI
    assert_eq!(g.event(id).remaining_escrow, usdc(500));
    assert_eq!(g.events.get_winner_count(&id), 2);

    g.events.unpause();
    g.pay(id, &a, 1, usdc(500));
    assert_eq!(g.status(id), EventStatus::Completed);
}

#[test]
fn a_cancel_paused_halfway_finishes_after_unpause() {
    let g = setup();
    let id = g.create(Spec::new(usdc(100), 1));
    let partners: std::vec::Vec<Address> = (0..30).map(|_| g.someone()).collect();
    for p in partners.iter() {
        g.contribute(id, p, usdc(10));
    }
    g.events.start_cancel(&id, &g.op());
    g.events.process_cancel_batch(&id, &25, &g.op());

    g.events.pause();
    fails_with(
        g.events.try_process_cancel_batch(&id, &25, &g.op()),
        Error::Paused,
    );
    g.events.unpause();

    g.finish_cancel(id);
    for p in partners.iter() {
        assert_eq!(g.balance(p), usdc(10));
    }
    assert_eq!(g.balance(&g.owner), usdc(100));
}

#[test]
fn only_the_admin_pauses() {
    let g = setup();
    g.sign_only(&g.owner, "pause", args(&g, ()));
    fails_auth(g.events.try_pause());
    g.restore_auth();
    assert!(!g.events.is_paused());
}
