// boundless-events: admin tests.

#![cfg(test)]

use super::common::setup;

#[test]
fn initializes_with_expected_config() {
    let ctx = setup(250);
    assert_eq!(ctx.client.get_admin(), ctx.admin);
    assert_eq!(ctx.client.get_fee_account(), ctx.fee_account);
    assert_eq!(ctx.client.get_fee_bps(), 250);
    assert_eq!(ctx.client.get_profile_contract(), ctx.profile_contract);
    assert_eq!(ctx.client.is_paused(), false);
}

#[test]
fn pause_and_unpause_round_trip() {
    let ctx = setup(250);
    ctx.client.pause();
    assert_eq!(ctx.client.is_paused(), true);
    ctx.client.unpause();
    assert_eq!(ctx.client.is_paused(), false);
}

#[test]
fn id_base_encodes_deployment_sequence() {
    let ctx = setup(250);
    let base = ctx.client.id_base();
    // id_base should be (seq << 32); lower 32 bits zero.
    assert_eq!(base & 0xFFFF_FFFF, 0);
}
