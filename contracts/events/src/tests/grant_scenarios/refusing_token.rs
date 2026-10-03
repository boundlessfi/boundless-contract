//! A token whose transfers to a chosen address fail, the way a transfer to an
//! account that removed its trustline does. The Stellar asset contract in the
//! test host has no way to drop a trustline, so this stands in for it.

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RefusingTokenError {
    NoTrustline = 1,
    InsufficientBalance = 2,
}

#[contracttype]
enum Key {
    Balance(Address),
    Refuses(Address),
}

#[contract]
pub struct RefusingToken;

#[contractimpl]
impl RefusingToken {
    pub fn mint(env: Env, to: Address, amount: i128) {
        let balance = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&Key::Balance(to), &(balance + amount));
    }

    pub fn refuse(env: Env, who: Address, refuses: bool) {
        env.storage().persistent().set(&Key::Refuses(who), &refuses);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&Key::Balance(id))
            .unwrap_or(0)
    }

    pub fn transfer(
        env: Env,
        from: Address,
        to: Address,
        amount: i128,
    ) -> Result<(), RefusingTokenError> {
        from.require_auth();
        let refuses: bool = env
            .storage()
            .persistent()
            .get(&Key::Refuses(to.clone()))
            .unwrap_or(false);
        if refuses {
            return Err(RefusingTokenError::NoTrustline);
        }
        let from_balance = Self::balance(env.clone(), from.clone());
        if from_balance < amount {
            return Err(RefusingTokenError::InsufficientBalance);
        }
        env.storage()
            .persistent()
            .set(&Key::Balance(from), &(from_balance - amount));
        let to_balance = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&Key::Balance(to), &(to_balance + amount));
        Ok(())
    }
}
