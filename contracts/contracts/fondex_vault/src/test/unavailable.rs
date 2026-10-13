extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::FondexError;

use super::{
    fondex_error, fund_vault_in_strategy, mint, rebalance_without_mock_auths, register_mock,
    register_usdc, register_vault, set_vault_strategies, strategy_info, usdc_client, ONE_USDC,
};

#[test]
fn unavailable_strategy_blocks_deposit_and_preview_deposit() {
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
    fund_vault_in_strategy(&e, &mock, &vault.address, 50 * ONE_USDC);
    mock.set_balance_fails(&true);

    assert_eq!(
        vault.try_preview_deposit(&(10 * ONE_USDC)).unwrap_err(),
        Ok(fondex_error(FondexError::StrategyUnavailable))
    );

    e.mock_all_auths();
    assert_eq!(
        vault
            .try_deposit(&(10 * ONE_USDC), &user, &user)
            .unwrap_err(),
        Ok(fondex_error(FondexError::StrategyUnavailable))
    );
}

#[test]
fn max_deposit_is_zero_when_strategy_unavailable() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    mint(&e, &usdc, &user, ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&ONE_USDC, &user, &user);
    mock.set_balance_fails(&true);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    assert_eq!(vault.max_deposit(&user), 0);
}

#[test]
fn unavailable_strategy_withdraw_uses_reserve_and_values_it_at_zero() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &alice, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &alice, &alice);
    rebalance_without_mock_auths(&e, &vault);
    mock.set_balance_fails(&true);

    // After 90/10 rebalance, conservative total is 10 USDC idle (strategy valued at 0).
    let shares = vault.balance(&alice);
    e.mock_all_auths();
    let assets = vault.redeem(&shares, &alice, &alice);
    assert_eq!(assets, 10 * ONE_USDC);
    assert_eq!(usdc_client(&e, &usdc).balance(&alice), 10 * ONE_USDC);
}

#[test]
fn unavailable_strategy_rebalance_skips_it() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let bad = register_mock(&e, &usdc);
    let good = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![
            &e,
            strategy_info(&bad.address, 4_500, false),
            strategy_info(&good.address, 4_500, false),
        ],
    );

    mint(&e, &usdc, &user, 1_000 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(1_000 * ONE_USDC), &user, &user);
    fund_vault_in_strategy(&e, &bad, &vault.address, 900 * ONE_USDC);
    bad.set_balance_fails(&true);

    rebalance_without_mock_auths(&e, &vault);

    let balances = vault.strategy_balances();
    assert_eq!(balances.get(0).unwrap().1, 0);
    assert_eq!(balances.get(1).unwrap().1, 45 * ONE_USDC);
}

#[test]
fn recovered_strategy_restores_full_value_for_remaining_holders() {
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
    vault.deposit(&(100 * ONE_USDC), &bob, &bob);
    rebalance_without_mock_auths(&e, &vault);

    mock.set_balance_fails(&true);
    let alice_shares = vault.balance(&alice);
    e.mock_all_auths();
    let alice_out = vault.redeem(&alice_shares, &alice, &alice);
    assert_eq!(alice_out, 10 * ONE_USDC);

    mock.set_balance_fails(&false);
    let bob_shares = vault.balance(&bob);
    e.mock_all_auths();
    let bob_out = vault.redeem(&bob_shares, &bob, &bob);
    assert!(bob_out >= 190 * ONE_USDC - 1);
    assert!(bob_out <= 190 * ONE_USDC);
}
