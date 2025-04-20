use crate::{
    datatypes::{BoundlessError, DataKey},
    interface::ContractManagement,
    BoundlessContract,BoundlessContractArgs, BoundlessContractClient
};
use soroban_sdk::{contractimpl, Address, BytesN, Env};

#[contractimpl]
impl ContractManagement for BoundlessContract {
    fn initialize(env: Env, admin: Address) -> Result<(), BoundlessError> {
        if env.storage().persistent().get(&DataKey::Initialized).unwrap_or(false) {
            return Err(BoundlessError::AlreadyInitialized);
        }
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(&DataKey::Version, &1u32);
        env.storage().persistent().set(&DataKey::Initialized, &true);
        Ok(())
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), BoundlessError> {
        let admin = Self::get_admin(&env);
        admin.require_auth();
        
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        let version = Self::get_version(&env);
        let new_version = version + 1;
        env.storage()
            .persistent()
            .set(&DataKey::Version, &new_version);
        Ok(())
    }

    fn get_admin(e: &Env) -> Address {
        e.storage()
            .persistent()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| {
                panic!("Admin not set");
            })
    }
    fn get_version(e: &Env) -> u32 {
        e.storage().persistent().get(&DataKey::Version).unwrap_or(0)
    }
}
