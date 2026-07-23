use soroban_sdk::{contractclient, Address, BytesN, Env, Symbol};

#[allow(dead_code)]
#[contractclient(name = "ProfileClient")]
pub trait ProfileInterface {
    fn bootstrap(env: Env, user: Address, op_id: BytesN<32>);
    fn bump_reputation(env: Env, user: Address, delta: u32, reason: Symbol, op_id: BytesN<32>);
    fn slash_reputation(env: Env, user: Address, delta: u32, reason: Symbol, op_id: BytesN<32>);
    fn register_earnings(env: Env, user: Address, token: Address, amount: i128, op_id: BytesN<32>);
}

pub fn client<'a>(env: &Env) -> ProfileClient<'a> {
    let addr = crate::storage::get_profile_contract(env);
    ProfileClient::new(env, &addr)
}
