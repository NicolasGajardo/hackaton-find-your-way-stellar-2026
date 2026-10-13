#![cfg(test)]
extern crate std;

mod admin;
mod deposit_withdraw;
mod invariants;
mod mock_strategy;
mod rebalance;
mod strategies;
mod unavailable;

use soroban_sdk::{
    testutils::{Ledger, MockAuth, MockAuthInvoke},
    token, Address, Env, Error, IntoVal, Val, Vec,
};

use crate::storage::{self, StrategyInfo};
use crate::{FondexError, FondexVault, FondexVaultClient};

use mock_strategy::{MockStrategy, MockStrategyClient};

pub(crate) const USDC_DECIMALS: u32 = 7;
pub(crate) const DECIMALS_OFFSET: u32 = 10;
pub(crate) const ONE_USDC: i128 = 10_i128.pow(USDC_DECIMALS);
pub(crate) const VIRTUAL_SHARES: i128 = 10_i128.pow(DECIMALS_OFFSET);
pub(crate) const SHARES_PER_USDC: i128 = ONE_USDC * VIRTUAL_SHARES;

pub(crate) fn register_usdc(e: &Env, admin: &Address) -> Address {
    e.register_stellar_asset_contract_v2(admin.clone())
        .address()
}

pub(crate) fn register_vault<'a>(
    e: &'a Env,
    usdc: &Address,
    owner: &Address,
) -> FondexVaultClient<'a> {
    register_vault_with_config(e, usdc, owner, 60_u64, i128::MAX)
}

pub(crate) fn register_vault_with_config<'a>(
    e: &'a Env,
    usdc: &Address,
    owner: &Address,
    strategy_timelock: u64,
    deposit_cap: i128,
) -> FondexVaultClient<'a> {
    let id = e.register(
        FondexVault,
        (usdc.clone(), owner.clone(), strategy_timelock, deposit_cap),
    );
    FondexVaultClient::new(e, &id)
}

pub(crate) fn mint(e: &Env, usdc: &Address, to: &Address, amount: i128) {
    e.mock_all_auths();
    token::StellarAssetClient::new(e, usdc).mint(to, &amount);
}

pub(crate) fn usdc_client<'a>(e: &'a Env, usdc: &Address) -> token::Client<'a> {
    token::Client::new(e, usdc)
}

pub(crate) fn fondex_error(err: FondexError) -> Error {
    Error::from_contract_error(err as u32)
}

pub(crate) fn set_vault_strategies(
    e: &Env,
    vault: &FondexVaultClient,
    strategies: Vec<StrategyInfo>,
) {
    e.as_contract(&vault.address, || storage::set_strategies(e, &strategies));
}

pub(crate) fn register_mock<'a>(e: &'a Env, usdc: &Address) -> MockStrategyClient<'a> {
    let id = e.register(MockStrategy, (usdc.clone(),));
    MockStrategyClient::new(e, &id)
}

pub(crate) fn strategy_info(address: &Address, target_bps: u32, paused: bool) -> StrategyInfo {
    StrategyInfo {
        address: address.clone(),
        target_bps,
        paused,
    }
}

pub(crate) fn fund_vault_in_strategy(
    e: &Env,
    mock: &MockStrategyClient,
    vault: &Address,
    amount: i128,
) {
    e.mock_all_auths();
    mock.deposit(&amount, vault);
}

pub(crate) fn rebalance_without_mock_auths(e: &Env, vault: &FondexVaultClient) {
    e.set_auths(&[]);
    vault.rebalance();
}

pub(crate) fn mock_owner_auth(
    e: &Env,
    vault: &FondexVaultClient,
    owner: &Address,
    fn_name: &str,
    args: impl IntoVal<Env, Vec<Val>>,
) {
    e.set_auths(&[]);
    e.mock_auths(&[MockAuth {
        address: owner,
        invoke: &MockAuthInvoke {
            contract: &vault.address,
            fn_name,
            args: args.into_val(e),
            sub_invokes: &[],
        },
    }]);
}

pub(crate) use strategies::add_yield;

pub(crate) fn add_strategy(
    e: &Env,
    vault: &FondexVaultClient,
    owner: &Address,
    strategy: &Address,
    target_bps: u32,
) {
    mock_owner_auth(
        e,
        vault,
        owner,
        "propose_strategy",
        (strategy.clone(), target_bps),
    );
    vault.propose_strategy(strategy, &target_bps);
    let pending = vault.pending_strategy().expect("pending strategy");
    e.ledger().with_mut(|ledger| {
        ledger.timestamp = pending.eta;
    });
    mock_owner_auth(e, vault, owner, "accept_strategy", ());
    vault.accept_strategy();
}
