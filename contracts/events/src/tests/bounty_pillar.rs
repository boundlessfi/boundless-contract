#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _},
    token, Address, BytesN, Env, Map, String,
};

use crate::errors::Error;
use crate::types::{CreateEventParams, Pillar, ReleaseKind};
use crate::{EventsContract, EventsContractClient};

use boundless_profile::{ProfileContract, ProfileContractClient};

const FEE_BPS: u32 = 250;
const TOTAL_BUDGET: i128 = 10_000_0000000_i128;

struct Ctx<'a> {
    env: Env,
    events: EventsContractClient<'a>,
    owner: Address,
    token_addr: Address,
}

fn setup<'a>() -> Ctx<'a> {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let profile_admin = Address::generate(&env);
    let profile_id = env.register(ProfileContract, (profile_admin.clone(),));
    let profile = ProfileContractClient::new(&env, &profile_id);

    let events_admin = Address::generate(&env);
    let fee_account = Address::generate(&env);
    let events_id = env.register(
        EventsContract,
        (
            events_admin.clone(),
            fee_account.clone(),
            FEE_BPS,
            profile_id.clone(),
        ),
    );
    let events = EventsContractClient::new(&env, &events_id);
    profile.set_events_contract(&events_id);

    let issuer = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(issuer);
    let token_addr = sac.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_addr);
    token_admin.mint(&fee_account, &0);

    let owner = Address::generate(&env);
    token_admin.mint(&owner, &1_000_000_0000000_i128);

    events.register_supported_token(&token_addr);

    Ctx {
        env,
        events,
        owner,
        token_addr,
    }
}

fn one_winner_distribution(env: &Env) -> Map<u32, i128> {
    let mut m = Map::new(env);
    m.set(1, 100000000000_i128);
    m
}

fn expect_op_err<T, E>(
    result: Result<Result<T, E>, Result<Error, soroban_sdk::InvokeError>>,
) -> Error {
    match result {
        Err(Ok(e)) => e,
        _ => panic!("expected contract error"),
    }
}

// ============================================================
// validate_create
// ============================================================

#[test]
fn create_rejects_multi_release_kind() {
    let ctx = setup();
    let params = CreateEventParams {
        pillar: Pillar::Bounty,
        owner: ctx.owner.clone(),
        token: ctx.token_addr.clone(),
        total_budget: TOTAL_BUDGET,
        release_kind: ReleaseKind::Multi(3),
        content_uri: String::from_str(&ctx.env, "uri"),
        title: String::from_str(&ctx.env, "Bad Bounty"),
        deadline: Some(ctx.env.ledger().timestamp() + 86_400),
        prize_floors: one_winner_distribution(&ctx.env),
        fee_bps_override: None,
        manager: None,
    };
    let op = BytesN::random(&ctx.env);
    let err = expect_op_err(ctx.events.try_create_event(&params, &op));
    assert_eq!(err, Error::InvalidReleaseKind);
}
