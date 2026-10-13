use soroban_sdk::{panic_with_error, Env};
use stellar_contract_utils::math::i128_fixed_point::mul_div_with_rounding;
use stellar_contract_utils::math::Rounding;
use stellar_tokens::fungible::Base;

use crate::{storage, strategy, FondexError};

pub const DECIMALS_OFFSET: u32 = 10;

fn virtual_shares() -> i128 {
    10_i128.pow(DECIMALS_OFFSET)
}

pub fn idle_assets_internal(e: &Env) -> i128 {
    let asset = storage::get_asset(e);
    let token = soroban_sdk::token::Client::new(e, &asset);
    token.balance(&e.current_contract_address())
}

pub fn total_assets_conservative(e: &Env) -> i128 {
    let mut total = idle_assets_internal(e);
    for info in storage::get_strategies(e).iter() {
        total += strategy::strategy_balance(e, &info.address).unwrap_or(0);
    }
    total
}

pub fn try_total_assets_strict(e: &Env) -> Option<i128> {
    let mut total = idle_assets_internal(e);
    for info in storage::get_strategies(e).iter() {
        match strategy::strategy_balance(e, &info.address) {
            Some(bal) => total += bal,
            None => return None,
        }
    }
    Some(total)
}

pub fn total_assets_strict(e: &Env) -> i128 {
    match try_total_assets_strict(e) {
        Some(total) => total,
        None => panic_with_error!(e, FondexError::StrategyUnavailable),
    }
}

pub fn total_assets_internal(e: &Env) -> i128 {
    total_assets_conservative(e)
}

pub fn convert_to_shares(e: &Env, assets: i128, total_assets: i128, rounding: Rounding) -> i128 {
    if assets <= 0 {
        return 0;
    }
    let total_supply = Base::total_supply(e);
    let numerator_supply = total_supply + virtual_shares();
    let denominator = total_assets + 1;
    mul_div_with_rounding(e, assets, numerator_supply, denominator, rounding)
}

pub fn convert_to_assets(e: &Env, shares: i128, total_assets: i128, rounding: Rounding) -> i128 {
    if shares <= 0 {
        return 0;
    }
    let total_supply = Base::total_supply(e);
    let numerator_assets = total_assets + 1;
    let denominator = total_supply + virtual_shares();
    mul_div_with_rounding(e, shares, numerator_assets, denominator, rounding)
}
