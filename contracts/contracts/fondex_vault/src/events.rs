use soroban_sdk::{contractevent, Address, Env};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deposit {
    #[topic]
    pub from: Address,
    #[topic]
    pub receiver: Address,
    pub assets: i128,
    pub shares: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Withdraw {
    #[topic]
    pub owner: Address,
    #[topic]
    pub receiver: Address,
    pub assets: i128,
    pub shares: i128,
}

pub fn emit_deposit(e: &Env, from: &Address, receiver: &Address, assets: i128, shares: i128) {
    Deposit {
        from: from.clone(),
        receiver: receiver.clone(),
        assets,
        shares,
    }
    .publish(e);
}

pub fn emit_withdraw(e: &Env, owner: &Address, receiver: &Address, assets: i128, shares: i128) {
    Withdraw {
        owner: owner.clone(),
        receiver: receiver.clone(),
        assets,
        shares,
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyWithdraw {
    #[topic]
    pub strategy: Address,
    pub amount: i128,
}

pub fn emit_strategy_withdraw(e: &Env, strategy: &Address, amount: i128) {
    StrategyWithdraw {
        strategy: strategy.clone(),
        amount,
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyDeposit {
    #[topic]
    pub strategy: Address,
    pub amount: i128,
}

pub fn emit_strategy_deposit(e: &Env, strategy: &Address, amount: i128) {
    StrategyDeposit {
        strategy: strategy.clone(),
        amount,
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rebalance {
    pub total_assets_before: i128,
    pub total_assets_after: i128,
}

pub fn emit_rebalance(e: &Env, total_assets_before: i128, total_assets_after: i128) {
    Rebalance {
        total_assets_before,
        total_assets_after,
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyProposed {
    #[topic]
    pub strategy: Address,
    pub target_bps: u32,
    pub eta: u64,
}

pub fn emit_strategy_proposed(e: &Env, strategy: &Address, target_bps: u32, eta: u64) {
    StrategyProposed {
        strategy: strategy.clone(),
        target_bps,
        eta,
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyCancelled {
    #[topic]
    pub strategy: Address,
}

pub fn emit_strategy_cancelled(e: &Env, strategy: &Address) {
    StrategyCancelled {
        strategy: strategy.clone(),
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyAccepted {
    #[topic]
    pub strategy: Address,
    pub target_bps: u32,
}

pub fn emit_strategy_accepted(e: &Env, strategy: &Address, target_bps: u32) {
    StrategyAccepted {
        strategy: strategy.clone(),
        target_bps,
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyRemoved {
    #[topic]
    pub strategy: Address,
    pub dust_left: i128,
}

pub fn emit_strategy_removed(e: &Env, strategy: &Address, dust_left: i128) {
    StrategyRemoved {
        strategy: strategy.clone(),
        dust_left,
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetChanged {
    #[topic]
    pub strategy: Address,
    pub old_bps: u32,
    pub new_bps: u32,
}

pub fn emit_target_changed(e: &Env, strategy: &Address, old_bps: u32, new_bps: u32) {
    TargetChanged {
        strategy: strategy.clone(),
        old_bps,
        new_bps,
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyPaused {
    #[topic]
    pub strategy: Address,
}

pub fn emit_strategy_paused(e: &Env, strategy: &Address) {
    StrategyPaused {
        strategy: strategy.clone(),
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyUnpaused {
    #[topic]
    pub strategy: Address,
}

pub fn emit_strategy_unpaused(e: &Env, strategy: &Address) {
    StrategyUnpaused {
        strategy: strategy.clone(),
    }
    .publish(e);
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositCapChanged {
    pub old_cap: i128,
    pub new_cap: i128,
}

pub fn emit_deposit_cap_changed(e: &Env, old_cap: i128, new_cap: i128) {
    DepositCapChanged { old_cap, new_cap }.publish(e);
}
