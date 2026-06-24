// boundless-events: client for the boundless-profile contract.
//
// The events contract calls the profile contract for credit / reputation
// mutations triggered by event-side flows. We declare the profile interface as
// a trait and let Soroban generate a typed client. The actual implementation
// lives in the separately-deployed profile contract; this is just the call
// surface from the events side.
//
// Spec: boundless-platform-contract-prd.md Section 4 (cross-contract dance).

use soroban_sdk::{contractclient, Address, BytesN, Env, Symbol};

// The trait body is consumed by the contractclient macro to generate
// ProfileClient. The trait itself has no other callers.
#[allow(dead_code)]
#[contractclient(name = "ProfileClient")]
pub trait ProfileInterface {
    fn bootstrap(env: Env, user: Address, op_id: BytesN<32>);
    fn spend_credits(env: Env, user: Address, amount: u32, reason: Symbol, op_id: BytesN<32>);
    fn earn_credits(env: Env, user: Address, amount: u32, reason: Symbol, op_id: BytesN<32>);
    fn refund_credits(env: Env, user: Address, amount: u32, reason: Symbol, op_id: BytesN<32>);
    fn bump_reputation(env: Env, user: Address, delta: u32, reason: Symbol, op_id: BytesN<32>);
    fn slash_reputation(env: Env, user: Address, delta: u32, reason: Symbol, op_id: BytesN<32>);
    fn register_earnings(env: Env, user: Address, token: Address, amount: i128, op_id: BytesN<32>);
}

/// Helper to build a typed ProfileClient pointing at the currently-configured
/// profile contract address.
pub fn client<'a>(env: &Env) -> ProfileClient<'a> {
    let addr = crate::storage::get_profile_contract(env);
    ProfileClient::new(env, &addr)
}
