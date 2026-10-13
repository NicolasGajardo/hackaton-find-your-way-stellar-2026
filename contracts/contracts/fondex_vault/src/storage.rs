use soroban_sdk::{contracttype, Address, Env, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyInfo {
    pub address: Address,
    pub target_bps: u32,
    pub paused: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingStrategy {
    pub address: Address,
    pub target_bps: u32,
    pub eta: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Asset,
    StrategyTimelock,
    DepositCap,
    Strategies,
    PendingStrategy,
}

pub fn get_asset(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Asset).unwrap()
}

pub fn set_asset(e: &Env, asset: &Address) {
    e.storage().instance().set(&DataKey::Asset, asset);
}

pub fn get_strategy_timelock(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&DataKey::StrategyTimelock)
        .unwrap()
}

pub fn set_strategy_timelock(e: &Env, secs: u64) {
    e.storage()
        .instance()
        .set(&DataKey::StrategyTimelock, &secs);
}

pub fn get_deposit_cap(e: &Env) -> i128 {
    e.storage().instance().get(&DataKey::DepositCap).unwrap()
}

pub fn set_deposit_cap(e: &Env, cap: i128) {
    e.storage().instance().set(&DataKey::DepositCap, &cap);
}

pub fn get_strategies(e: &Env) -> Vec<StrategyInfo> {
    e.storage()
        .instance()
        .get(&DataKey::Strategies)
        .unwrap_or_else(|| Vec::new(e))
}

pub fn set_strategies(e: &Env, strategies: &Vec<StrategyInfo>) {
    e.storage().instance().set(&DataKey::Strategies, strategies);
}

pub fn get_pending_strategy(e: &Env) -> Option<PendingStrategy> {
    e.storage().instance().get(&DataKey::PendingStrategy)
}

pub fn set_pending_strategy(e: &Env, pending: &PendingStrategy) {
    e.storage()
        .instance()
        .set(&DataKey::PendingStrategy, pending);
}

pub fn clear_pending_strategy(e: &Env) {
    e.storage().instance().remove(&DataKey::PendingStrategy);
}
