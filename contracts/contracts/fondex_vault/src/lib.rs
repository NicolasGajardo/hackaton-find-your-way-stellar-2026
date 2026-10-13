#![no_std]

mod errors;

pub use errors::FondexError;
mod events;
mod math;
mod storage;
mod strategy;

pub use storage::{PendingStrategy, StrategyInfo};
pub use strategy::Strategy;

use strategy::StrategyClient;

use math::{
    idle_assets_internal, total_assets_conservative, total_assets_internal, total_assets_strict,
    try_total_assets_strict, DECIMALS_OFFSET,
};
use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Env, MuxedAddress, String, Vec,
};
use stellar_access::ownable;
use stellar_contract_utils::math::i128_fixed_point::mul_div_with_rounding;
use stellar_contract_utils::math::Rounding;
use stellar_contract_utils::pausable;
use stellar_tokens::fungible::{
    Base, FungibleToken, INSTANCE_EXTEND_AMOUNT, INSTANCE_TTL_THRESHOLD,
};

pub const SHARE_NAME: &str = "Fondex Fund Share";
pub const SHARE_SYMBOL: &str = "fxFUND";
pub const MAX_TIMELOCK_SECS: u64 = 604_800;
pub const WITHDRAW_ROUNDING_BUFFER: i128 = 100;
pub const BPS: u32 = 10_000;
pub const REBALANCE_TOLERANCE_BPS: u32 = 200;
pub const MIN_STRATEGY_MOVE: i128 = 10_000_000;
pub const MIN_IDLE_BPS: u32 = 500;
pub const MAX_STRATEGIES: u32 = 4;
pub const STRATEGY_DUST: i128 = 1_000;

#[contract]
pub struct FondexVault;

fn extend_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_EXTEND_AMOUNT);
}

fn require_positive(e: &Env, amount: i128) {
    if amount <= 0 {
        panic_with_error!(e, FondexError::AmountMustBePositive);
    }
}

fn ensure_liquidity(e: &Env, needed: i128) {
    let mut idle = math::idle_assets_internal(e);
    if idle >= needed {
        return;
    }
    for info in storage::get_strategies(e).iter() {
        let bal = match strategy::strategy_balance(e, &info.address) {
            Some(b) if b > 0 => b,
            _ => continue,
        };
        let shortfall = needed - idle;
        let request = shortfall
            .checked_add(WITHDRAW_ROUNDING_BUFFER)
            .unwrap_or(i128::MAX);
        let request = core::cmp::min(request, bal);
        strategy::divest(e, &info.address, request);
        idle = math::idle_assets_internal(e);
        if idle >= needed {
            return;
        }
    }
    if idle < needed {
        panic_with_error!(e, FondexError::InsufficientLiquidity);
    }
}

fn usdc_client(e: &Env) -> token::Client<'_> {
    token::Client::new(e, &storage::get_asset(e))
}

fn bps_floor(e: &Env, value: i128, bps: u32) -> i128 {
    mul_div_with_rounding(e, value, i128::from(bps), i128::from(BPS), Rounding::Floor)
}

fn active_target_bps_sum(strategies: &Vec<StrategyInfo>) -> u32 {
    let mut sum: u32 = 0;
    for info in strategies.iter() {
        if !info.paused {
            sum += info.target_bps;
        }
    }
    sum
}

fn total_target_bps_sum(e: &Env, strategies: &Vec<StrategyInfo>) -> u32 {
    let mut sum: u32 = 0;
    for info in strategies.iter() {
        sum = match sum.checked_add(info.target_bps) {
            Some(s) => s,
            None => panic_with_error!(e, FondexError::AllocationTooHigh),
        };
    }
    sum
}

fn checked_add_target_bps(e: &Env, sum: u32, add: u32) -> u32 {
    match sum.checked_add(add) {
        Some(s) => s,
        None => panic_with_error!(e, FondexError::AllocationTooHigh),
    }
}

fn max_allocation_bps() -> u32 {
    BPS - MIN_IDLE_BPS
}

fn find_strategy_index(strategies: &Vec<StrategyInfo>, strategy: &Address) -> Option<u32> {
    for (index, info) in (0_u32..).zip(strategies.iter()) {
        if info.address == *strategy {
            return Some(index);
        }
    }
    None
}

fn strategy_registered(strategies: &Vec<StrategyInfo>, strategy: &Address) -> bool {
    find_strategy_index(strategies, strategy).is_some()
}

fn strategy_asset_matches(e: &Env, strategy: &Address) -> bool {
    StrategyClient::new(e, strategy).asset() == storage::get_asset(e)
}

fn set_strategy_paused(e: &Env, strategy: &Address, paused: bool) {
    let strategies = storage::get_strategies(e);
    let index = find_strategy_index(&strategies, strategy);
    let Some(index) = index else {
        panic_with_error!(e, FondexError::StrategyNotFound);
    };
    let Some(current) = strategies.get(index) else {
        panic_with_error!(e, FondexError::StrategyNotFound);
    };
    if current.paused == paused {
        return;
    }
    let mut updated = Vec::new(e);
    for (i, info) in (0_u32..).zip(strategies.iter()) {
        if i == index {
            updated.push_back(StrategyInfo {
                address: info.address.clone(),
                target_bps: info.target_bps,
                paused,
            });
        } else {
            updated.push_back(info);
        }
    }
    storage::set_strategies(e, &updated);
    if paused {
        events::emit_strategy_paused(e, strategy);
    } else {
        events::emit_strategy_unpaused(e, strategy);
    }
}

#[contractimpl]
impl FondexVault {
    pub fn __constructor(
        e: &Env,
        asset: Address,
        owner: Address,
        strategy_timelock: u64,
        deposit_cap: i128,
    ) {
        if strategy_timelock > MAX_TIMELOCK_SECS {
            panic_with_error!(e, FondexError::InvalidTimelock);
        }
        if deposit_cap <= 0 {
            panic_with_error!(e, FondexError::InvalidDepositCap);
        }
        storage::set_asset(e, &asset);
        storage::set_strategy_timelock(e, strategy_timelock);
        storage::set_deposit_cap(e, deposit_cap);
        storage::set_strategies(e, &Vec::new(e));
        ownable::set_owner(e, &owner);
        let asset_decimals = token::Client::new(e, &asset).decimals();
        Base::set_metadata(
            e,
            asset_decimals + DECIMALS_OFFSET,
            String::from_str(e, SHARE_NAME),
            String::from_str(e, SHARE_SYMBOL),
        );
        extend_instance_ttl(e);
    }

    pub fn query_asset(e: &Env) -> Address {
        extend_instance_ttl(e);
        storage::get_asset(e)
    }

    pub fn idle_assets(e: &Env) -> i128 {
        extend_instance_ttl(e);
        idle_assets_internal(e)
    }

    pub fn total_assets(e: &Env) -> i128 {
        extend_instance_ttl(e);
        total_assets_conservative(e)
    }

    pub fn convert_to_shares(e: &Env, assets: i128) -> i128 {
        extend_instance_ttl(e);
        let total = total_assets_conservative(e);
        math::convert_to_shares(e, assets, total, Rounding::Floor)
    }

    pub fn convert_to_assets(e: &Env, shares: i128) -> i128 {
        extend_instance_ttl(e);
        let total = total_assets_conservative(e);
        math::convert_to_assets(e, shares, total, Rounding::Floor)
    }

    pub fn preview_deposit(e: &Env, assets: i128) -> i128 {
        extend_instance_ttl(e);
        let total = total_assets_strict(e);
        math::convert_to_shares(e, assets, total, Rounding::Floor)
    }

    pub fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        extend_instance_ttl(e);
        let total = total_assets_conservative(e);
        math::convert_to_shares(e, assets, total, Rounding::Ceil)
    }

    pub fn preview_redeem(e: &Env, shares: i128) -> i128 {
        extend_instance_ttl(e);
        let total = total_assets_conservative(e);
        math::convert_to_assets(e, shares, total, Rounding::Floor)
    }

    pub fn max_withdraw(e: &Env, owner: Address) -> i128 {
        extend_instance_ttl(e);
        Self::preview_redeem(e, Base::balance(e, &owner))
    }

    pub fn max_redeem(e: &Env, owner: Address) -> i128 {
        extend_instance_ttl(e);
        Base::balance(e, &owner)
    }

    pub fn strategy_timelock(e: &Env) -> u64 {
        extend_instance_ttl(e);
        storage::get_strategy_timelock(e)
    }

    pub fn deposit_cap(e: &Env) -> i128 {
        extend_instance_ttl(e);
        storage::get_deposit_cap(e)
    }

    pub fn max_deposit(e: &Env, _receiver: Address) -> i128 {
        extend_instance_ttl(e);
        if pausable::paused(e) {
            return 0;
        }
        let Some(total) = try_total_assets_strict(e) else {
            return 0;
        };
        let cap = storage::get_deposit_cap(e);
        core::cmp::max(cap - total, 0)
    }

    pub fn paused(e: &Env) -> bool {
        extend_instance_ttl(e);
        pausable::paused(e)
    }

    pub fn pause(e: &Env) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        pausable::pause(e);
    }

    pub fn unpause(e: &Env) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        pausable::unpause(e);
    }

    pub fn get_owner(e: &Env) -> Option<Address> {
        extend_instance_ttl(e);
        ownable::get_owner(e)
    }

    pub fn transfer_ownership(e: &Env, new_owner: Address, live_until_ledger: u32) {
        extend_instance_ttl(e);
        ownable::transfer_ownership(e, &new_owner, live_until_ledger);
    }

    pub fn accept_ownership(e: &Env) {
        extend_instance_ttl(e);
        ownable::accept_ownership(e);
    }

    pub fn pending_strategy(e: &Env) -> Option<PendingStrategy> {
        extend_instance_ttl(e);
        storage::get_pending_strategy(e)
    }

    pub fn propose_strategy(e: &Env, strategy: Address, target_bps: u32) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        if storage::get_pending_strategy(e).is_some() {
            panic_with_error!(e, FondexError::PendingStrategyExists);
        }
        let strategies = storage::get_strategies(e);
        if strategy_registered(&strategies, &strategy) {
            panic_with_error!(e, FondexError::StrategyAlreadyExists);
        }
        if strategies.len() >= MAX_STRATEGIES {
            panic_with_error!(e, FondexError::TooManyStrategies);
        }
        if !strategy_asset_matches(e, &strategy) {
            panic_with_error!(e, FondexError::AssetMismatch);
        }
        let new_sum = checked_add_target_bps(e, total_target_bps_sum(e, &strategies), target_bps);
        if new_sum > max_allocation_bps() {
            panic_with_error!(e, FondexError::AllocationTooHigh);
        }
        let eta = e.ledger().timestamp() + storage::get_strategy_timelock(e);
        let pending = PendingStrategy {
            address: strategy.clone(),
            target_bps,
            eta,
        };
        storage::set_pending_strategy(e, &pending);
        events::emit_strategy_proposed(e, &strategy, target_bps, eta);
    }

    pub fn cancel_strategy(e: &Env) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        let pending = storage::get_pending_strategy(e);
        let Some(pending) = pending else {
            panic_with_error!(e, FondexError::NoPendingStrategy);
        };
        storage::clear_pending_strategy(e);
        events::emit_strategy_cancelled(e, &pending.address);
    }

    pub fn accept_strategy(e: &Env) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        let pending = storage::get_pending_strategy(e);
        let Some(pending) = pending else {
            panic_with_error!(e, FondexError::NoPendingStrategy);
        };
        if e.ledger().timestamp() < pending.eta {
            panic_with_error!(e, FondexError::TimelockNotExpired);
        }
        let strategies = storage::get_strategies(e);
        if strategies.len() >= MAX_STRATEGIES {
            panic_with_error!(e, FondexError::TooManyStrategies);
        }
        if strategy_registered(&strategies, &pending.address) {
            panic_with_error!(e, FondexError::StrategyAlreadyExists);
        }
        let new_sum =
            checked_add_target_bps(e, total_target_bps_sum(e, &strategies), pending.target_bps);
        if new_sum > max_allocation_bps() {
            panic_with_error!(e, FondexError::AllocationTooHigh);
        }
        if !strategy_asset_matches(e, &pending.address) {
            panic_with_error!(e, FondexError::AssetMismatch);
        }
        let mut updated = strategies;
        updated.push_back(StrategyInfo {
            address: pending.address.clone(),
            target_bps: pending.target_bps,
            paused: false,
        });
        storage::set_strategies(e, &updated);
        storage::clear_pending_strategy(e);
        events::emit_strategy_accepted(e, &pending.address, pending.target_bps);
    }

    pub fn set_target(e: &Env, strategy: Address, target_bps: u32) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        let strategies = storage::get_strategies(e);
        let index = find_strategy_index(&strategies, &strategy);
        let Some(index) = index else {
            panic_with_error!(e, FondexError::StrategyNotFound);
        };
        let Some(old) = strategies.get(index) else {
            panic_with_error!(e, FondexError::StrategyNotFound);
        };
        let old_bps = old.target_bps;
        let base = total_target_bps_sum(e, &strategies);
        let without_old = match base.checked_sub(old_bps) {
            Some(s) => s,
            None => panic_with_error!(e, FondexError::AllocationTooHigh),
        };
        let new_sum = checked_add_target_bps(e, without_old, target_bps);
        if new_sum > max_allocation_bps() {
            panic_with_error!(e, FondexError::AllocationTooHigh);
        }
        let mut updated = Vec::new(e);
        for (i, info) in (0_u32..).zip(strategies.iter()) {
            if i == index {
                updated.push_back(StrategyInfo {
                    address: info.address.clone(),
                    target_bps,
                    paused: info.paused,
                });
            } else {
                updated.push_back(info);
            }
        }
        storage::set_strategies(e, &updated);
        events::emit_target_changed(e, &strategy, old_bps, target_bps);
    }

    pub fn pause_strategy(e: &Env, strategy: Address) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        set_strategy_paused(e, &strategy, true);
    }

    pub fn unpause_strategy(e: &Env, strategy: Address) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        set_strategy_paused(e, &strategy, false);
    }

    pub fn remove_strategy(e: &Env, strategy: Address) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        let strategies = storage::get_strategies(e);
        let index = find_strategy_index(&strategies, &strategy);
        let Some(index) = index else {
            panic_with_error!(e, FondexError::StrategyNotFound);
        };
        let bal = match strategy::strategy_balance(e, &strategy) {
            Some(b) => b,
            None => panic_with_error!(e, FondexError::StrategyUnavailable),
        };
        if bal > 0 {
            strategy::divest(e, &strategy, bal);
        }
        let remaining = match strategy::strategy_balance(e, &strategy) {
            Some(b) => b,
            None => panic_with_error!(e, FondexError::StrategyUnavailable),
        };
        if remaining > STRATEGY_DUST {
            panic_with_error!(e, FondexError::StrategyNotEmpty);
        }
        let mut updated = Vec::new(e);
        for (i, info) in (0_u32..).zip(strategies.iter()) {
            if i != index {
                updated.push_back(info);
            }
        }
        storage::set_strategies(e, &updated);
        events::emit_strategy_removed(e, &strategy, remaining);
    }

    pub fn set_deposit_cap(e: &Env, cap: i128) {
        ownable::enforce_owner_auth(e);
        extend_instance_ttl(e);
        if cap <= 0 {
            panic_with_error!(e, FondexError::InvalidDepositCap);
        }
        let old_cap = storage::get_deposit_cap(e);
        storage::set_deposit_cap(e, cap);
        events::emit_deposit_cap_changed(e, old_cap, cap);
    }

    pub fn strategies(e: &Env) -> Vec<StrategyInfo> {
        extend_instance_ttl(e);
        storage::get_strategies(e)
    }

    pub fn strategy_balances(e: &Env) -> Vec<(Address, i128)> {
        extend_instance_ttl(e);
        let mut balances = Vec::new(e);
        for info in storage::get_strategies(e).iter() {
            balances.push_back((
                info.address.clone(),
                strategy::strategy_balance(e, &info.address).unwrap_or(0),
            ));
        }
        balances
    }

    pub fn deposit(e: &Env, assets: i128, receiver: Address, from: Address) -> i128 {
        from.require_auth();
        require_positive(e, assets);
        pausable::when_not_paused(e);
        extend_instance_ttl(e);
        let total = total_assets_strict(e);
        let after_deposit = match total.checked_add(assets) {
            Some(t) => t,
            None => panic_with_error!(e, FondexError::DepositCapExceeded),
        };
        if after_deposit > storage::get_deposit_cap(e) {
            panic_with_error!(e, FondexError::DepositCapExceeded);
        }
        let shares = math::convert_to_shares(e, assets, total, Rounding::Floor);
        if shares == 0 {
            panic_with_error!(e, FondexError::ZeroShares);
        }
        usdc_client(e).transfer(&from, e.current_contract_address(), &assets);
        Base::mint(e, &receiver, shares);
        events::emit_deposit(e, &from, &receiver, assets, shares);
        shares
    }

    pub fn withdraw(e: &Env, assets: i128, receiver: Address, owner: Address) -> i128 {
        owner.require_auth();
        require_positive(e, assets);
        extend_instance_ttl(e);
        let shares = Self::preview_withdraw(e, assets);
        ensure_liquidity(e, assets);
        Base::update(e, Some(&owner), None, shares);
        usdc_client(e).transfer(&e.current_contract_address(), &receiver, &assets);
        events::emit_withdraw(e, &owner, &receiver, assets, shares);
        shares
    }

    pub fn redeem(e: &Env, shares: i128, receiver: Address, owner: Address) -> i128 {
        owner.require_auth();
        require_positive(e, shares);
        extend_instance_ttl(e);
        let assets = Self::preview_redeem(e, shares);
        if assets == 0 {
            panic_with_error!(e, FondexError::ZeroAssets);
        }
        ensure_liquidity(e, assets);
        Base::update(e, Some(&owner), None, shares);
        usdc_client(e).transfer(&e.current_contract_address(), &receiver, &assets);
        events::emit_withdraw(e, &owner, &receiver, assets, shares);
        assets
    }

    pub fn rebalance(e: &Env) {
        pausable::when_not_paused(e);
        extend_instance_ttl(e);
        let total = total_assets_internal(e);
        let tol = bps_floor(e, total, REBALANCE_TOLERANCE_BPS);
        let strategies = storage::get_strategies(e);

        for info in strategies.iter() {
            let bal = match strategy::strategy_balance(e, &info.address) {
                Some(b) => b,
                None => continue,
            };
            let target = if info.paused {
                0
            } else {
                bps_floor(e, total, info.target_bps)
            };
            if bal > target + tol {
                let excess = bal - target;
                if excess >= MIN_STRATEGY_MOVE {
                    strategy::divest(e, &info.address, excess);
                }
            }
        }

        let active_bps = active_target_bps_sum(&strategies);
        let idle_target = bps_floor(e, total, BPS - active_bps);
        let mut disponible = math::idle_assets_internal(e) - idle_target;

        for info in strategies.iter() {
            if info.paused {
                continue;
            }
            if disponible < MIN_STRATEGY_MOVE {
                break;
            }
            let bal = match strategy::strategy_balance(e, &info.address) {
                Some(b) => b,
                None => continue,
            };
            let target = bps_floor(e, total, info.target_bps);
            if bal < target - tol {
                let needed = target - bal;
                let amount = core::cmp::min(needed, disponible);
                if amount >= MIN_STRATEGY_MOVE {
                    let invested = strategy::invest(e, &info.address, amount);
                    disponible -= invested;
                }
            }
        }

        let total_after = total_assets_internal(e);
        events::emit_rebalance(e, total, total_after);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for FondexVault {
    type ContractType = Base;
}

#[cfg(test)]
mod test;
