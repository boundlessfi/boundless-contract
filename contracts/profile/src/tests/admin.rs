// boundless-profile: admin tests.

#![cfg(test)]

use super::common::setup;

#[test]
fn initializes_with_expected_config() {
    let ctx = setup(10);
    assert_eq!(ctx.client.get_admin(), ctx.admin);
    assert_eq!(ctx.client.get_default_bootstrap_credits(), 10);
    assert_eq!(ctx.client.is_paused(), false);
    assert_eq!(ctx.client.get_events_contract(), None);
}

#[test]
fn pause_and_unpause_round_trip() {
    let ctx = setup(10);
    ctx.client.pause();
    assert_eq!(ctx.client.is_paused(), true);
    ctx.client.unpause();
    assert_eq!(ctx.client.is_paused(), false);
}
