extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events},
    vec, xdr, Address, Env, Symbol, TryFromVal, Val,
};

use crate::{FondexError, WITHDRAW_ROUNDING_BUFFER};

use super::mock_strategy::MockStrategyClient;
use super::{
    fondex_error, fund_vault_in_strategy, mint, register_mock, register_usdc, register_vault,
    set_vault_strategies, strategy_info, usdc_client, ONE_USDC,
};

fn snapshot_balances(
    e: &Env,
    usdc: &Address,
    vault: &crate::FondexVaultClient,
    user: &Address,
    mock: &MockStrategyClient,
) -> (i128, i128, i128, i128) {
    (
        usdc_client(e, usdc).balance(user),
        vault.balance(user),
        mock.balance(&vault.address),
        vault.total_assets(),
    )
}

pub(crate) fn add_yield(
    e: &Env,
    usdc: &Address,
    mock: &MockStrategyClient,
    vault: &Address,
    bps: u32,
) {
    let before = mock.balance(vault);
    mock.simulate_yield(vault, &bps);
    let after = mock.balance(vault);
    let delta = after - before;
    if delta > 0 {
        mint(e, usdc, &mock.address, delta);
    }
}

#[test]
fn total_assets_includes_strategy_balances() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 90 * ONE_USDC);

    assert_eq!(vault.idle_assets(), 10 * ONE_USDC);
    assert_eq!(vault.total_assets(), 100 * ONE_USDC);
}

#[test]
fn idle_assets_excludes_strategies() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 50 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(50 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 40 * ONE_USDC);

    assert_eq!(vault.idle_assets(), 10 * ONE_USDC);
    assert_eq!(mock.balance(&vault.address), 40 * ONE_USDC);
}

#[test]
fn strategy_yield_raises_share_price() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &alice, 100 * ONE_USDC);
    mint(&e, &usdc, &bob, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &alice, &alice);
    fund_vault_in_strategy(&e, &mock, &vault.address, 100 * ONE_USDC);
    add_yield(&e, &usdc, &mock, &vault.address, 1_000);

    let alice_shares = vault.balance(&alice);
    let bob_shares = vault.deposit(&(100 * ONE_USDC), &bob, &bob);
    assert!(bob_shares < alice_shares);
}

#[test]
fn strategy_loss_lowers_share_price() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &alice, 100 * ONE_USDC);
    mint(&e, &usdc, &bob, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &alice, &alice);
    fund_vault_in_strategy(&e, &mock, &vault.address, 100 * ONE_USDC);
    mock.simulate_loss(&vault.address, &500);

    let alice_shares = vault.balance(&alice);
    let bob_shares = vault.deposit(&(100 * ONE_USDC), &bob, &bob);
    assert!(bob_shares > alice_shares);
}

#[test]
fn withdraw_within_reserve_does_not_touch_strategies() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 90 * ONE_USDC);

    let strategy_before = mock.balance(&vault.address);
    vault.withdraw(&(5 * ONE_USDC), &user, &user);
    assert_eq!(mock.balance(&vault.address), strategy_before);
}

#[test]
fn withdraw_larger_than_reserve_pulls_shortfall_from_strategy() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 90 * ONE_USDC);

    let withdraw_amount = 50 * ONE_USDC;
    let request = 40 * ONE_USDC + WITHDRAW_ROUNDING_BUFFER;
    let strategy_before = mock.balance(&vault.address);
    let user_usdc_before = usdc_client(&e, &usdc).balance(&user);

    vault.withdraw(&withdraw_amount, &user, &user);

    assert_eq!(
        usdc_client(&e, &usdc).balance(&user),
        user_usdc_before + withdraw_amount
    );
    assert_eq!(mock.balance(&vault.address), strategy_before - request);
    assert_eq!(vault.idle_assets(), WITHDRAW_ROUNDING_BUFFER);
}

#[test]
fn redeem_all_pulls_everything_needed_from_strategy() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    let shares = vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 90 * ONE_USDC);

    let out = vault.redeem(&shares, &user, &user);
    assert_eq!(out, 100 * ONE_USDC);
    assert_eq!(vault.balance(&user), 0);
    assert_eq!(mock.balance(&vault.address), 0);
    assert_eq!(vault.idle_assets(), 0);
}

#[test]
fn withdraw_skips_failing_strategy_and_uses_next() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock_b = register_mock(&e, &usdc);
    mock_a.set_withdraw_cap(&0);
    set_vault_strategies(
        &e,
        &vault,
        vec![
            &e,
            strategy_info(&mock_a.address, 9_000, false),
            strategy_info(&mock_b.address, 9_000, false),
        ],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock_a, &vault.address, 45 * ONE_USDC);
    fund_vault_in_strategy(&e, &mock_b, &vault.address, 45 * ONE_USDC);

    vault.withdraw(&(55 * ONE_USDC), &user, &user);

    assert_eq!(mock_a.balance(&vault.address), 45 * ONE_USDC);
    assert_eq!(mock_b.balance(&vault.address), 0);
}

#[test]
fn withdraw_uses_paused_strategy() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, true)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 90 * ONE_USDC);

    vault.withdraw(&(50 * ONE_USDC), &user, &user);
    assert_eq!(
        mock.balance(&vault.address),
        50 * ONE_USDC - WITHDRAW_ROUNDING_BUFFER
    );
}

#[test]
fn withdraw_shortfall_within_buffer_still_pays_exact_amount() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    mock.set_withdraw_shortfall(&50);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 90 * ONE_USDC);

    let withdraw_amount = 50 * ONE_USDC;
    let user_before = usdc_client(&e, &usdc).balance(&user);
    vault.withdraw(&withdraw_amount, &user, &user);
    assert_eq!(
        usdc_client(&e, &usdc).balance(&user),
        user_before + withdraw_amount
    );
}

#[test]
fn withdraw_without_enough_liquidity_fails_with_insufficient_liquidity() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    let shares = vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 90 * ONE_USDC);
    mock.set_withdraw_cap(&0);

    let before = snapshot_balances(&e, &usdc, &vault, &user, &mock);
    assert_eq!(
        vault
            .try_withdraw(&(50 * ONE_USDC), &user, &user)
            .unwrap_err(),
        Ok(fondex_error(FondexError::InsufficientLiquidity))
    );
    let after = snapshot_balances(&e, &usdc, &vault, &user, &mock);
    assert_eq!(before, after);
    assert_eq!(vault.balance(&user), shares);
}

#[test]
fn strategies_and_strategy_balances_views() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock_b = register_mock(&e, &usdc);
    let info_a = strategy_info(&mock_a.address, 9_000, false);
    let info_b = strategy_info(&mock_b.address, 9_000, true);
    set_vault_strategies(&e, &vault, vec![&e, info_a.clone(), info_b.clone()]);

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock_a, &vault.address, 30 * ONE_USDC);
    fund_vault_in_strategy(&e, &mock_b, &vault.address, 20 * ONE_USDC);

    assert_eq!(vault.strategies(), vec![&e, info_a, info_b]);
    assert_eq!(
        vault.strategy_balances(),
        vec![
            &e,
            (mock_a.address.clone(), 30 * ONE_USDC),
            (mock_b.address.clone(), 20 * ONE_USDC),
        ]
    );
}

fn strategy_withdraw_amount(e: &Env, event: &xdr::ContractEvent) -> i128 {
    let xdr::ContractEventBody::V0(body) = &event.body;
    let val = Val::try_from_val(e, &body.data).unwrap();
    let map: soroban_sdk::Map<Symbol, i128> = soroban_sdk::Map::try_from_val(e, &val).unwrap();
    map.get(Symbol::new(e, "amount")).unwrap()
}

#[test]
fn divest_emits_strategy_withdraw_with_measured_amount() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    mock.set_withdraw_shortfall(&30);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &mock, &vault.address, 90 * ONE_USDC);

    let request = 40 * ONE_USDC + WITHDRAW_ROUNDING_BUFFER;
    let expected_received = request - 30;
    vault.withdraw(&(50 * ONE_USDC), &user, &user);

    let events = e.events().all().filter_by_contract(&vault.address);
    let event = events
        .events()
        .iter()
        .find(|ev| {
            let xdr::ContractEventBody::V0(body) = &ev.body;
            let name =
                Symbol::try_from_val(&e, &Val::try_from_val(&e, &body.topics[0]).unwrap()).unwrap();
            name == Symbol::new(&e, "strategy_withdraw")
        })
        .expect("strategy_withdraw event");
    assert_eq!(strategy_withdraw_amount(&e, event), expected_received);
}
