use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contractclient, vec, Address, Bytes, Env, IntoVal, Symbol,
};

use crate::{events, math, storage};

#[contractclient(name = "StrategyClient")]
pub trait Strategy {
    fn asset(e: Env) -> Address;
    fn deposit(e: Env, amount: i128, from: Address) -> i128;
    fn withdraw(e: Env, amount: i128, from: Address, to: Address) -> i128;
    fn balance(e: Env, from: Address) -> i128;
    fn harvest(e: Env, from: Address, data: Option<Bytes>);
}

pub fn strategy_balance(e: &Env, strategy: &Address) -> Option<i128> {
    let vault = e.current_contract_address();
    match StrategyClient::new(e, strategy).try_balance(&vault) {
        Ok(Ok(bal)) => Some(bal),
        _ => None,
    }
}

pub fn divest(e: &Env, strategy: &Address, amount: i128) -> i128 {
    if amount <= 0 {
        return 0;
    }
    let idle_before = math::idle_assets_internal(e);
    let vault = e.current_contract_address();
    let client = StrategyClient::new(e, strategy);
    match client.try_withdraw(&amount, &vault, &vault) {
        Ok(_) => {
            let idle_after = math::idle_assets_internal(e);
            let received = idle_after - idle_before;
            if received > 0 {
                events::emit_strategy_withdraw(e, strategy, received);
            }
            received
        }
        Err(_) => 0,
    }
}

pub fn invest(e: &Env, strategy: &Address, amount: i128) -> i128 {
    if amount <= 0 {
        return 0;
    }
    let idle_before = math::idle_assets_internal(e);
    let vault = e.current_contract_address();
    let usdc = storage::get_asset(e);
    e.authorize_as_current_contract(vec![
        e,
        InvokerContractAuthEntry::Contract(SubContractInvocation {
            context: ContractContext {
                contract: usdc.clone(),
                fn_name: Symbol::new(e, "transfer"),
                args: (vault.clone(), strategy.clone(), amount).into_val(e),
            },
            sub_invocations: vec![e],
        }),
    ]);
    let client = StrategyClient::new(e, strategy);
    match client.try_deposit(&amount, &vault) {
        Ok(_) => {
            let idle_after = math::idle_assets_internal(e);
            let invested = idle_before - idle_after;
            if invested > 0 {
                events::emit_strategy_deposit(e, strategy, invested);
            }
            invested
        }
        Err(_) => 0,
    }
}
