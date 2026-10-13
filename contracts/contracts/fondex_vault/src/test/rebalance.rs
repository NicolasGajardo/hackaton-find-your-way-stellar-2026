extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events},
    vec, xdr, Address, Env, Symbol, TryFromVal, Val, Vec,
};

use crate::{storage, WITHDRAW_ROUNDING_BUFFER};

use super::{
    mint, rebalance_without_mock_auths, register_mock, register_usdc, register_vault,
    set_vault_strategies, strategy_info, usdc_client, ONE_USDC,
};

fn event_name(e: &Env, event: &xdr::ContractEvent) -> Symbol {
    let xdr::ContractEventBody::V0(body) = &event.body;
    Symbol::try_from_val(e, &Val::try_from_val(e, &body.topics[0]).unwrap()).unwrap()
}

fn event_i128_field(e: &Env, event: &xdr::ContractEvent, field: &str) -> i128 {
    let xdr::ContractEventBody::V0(body) = &event.body;
    let val = Val::try_from_val(e, &body.data).unwrap();
    let map: soroban_sdk::Map<Symbol, i128> = soroban_sdk::Map::try_from_val(e, &val).unwrap();
    map.get(Symbol::new(e, field)).unwrap()
}

fn topic_address(e: &Env, event: &xdr::ContractEvent) -> Address {
    let xdr::ContractEventBody::V0(body) = &event.body;
    Address::try_from_val(e, &Val::try_from_val(e, &body.topics[1]).unwrap()).unwrap()
}

fn set_strategy_target_bps(
    e: &Env,
    vault: &crate::FondexVaultClient,
    strategy: &Address,
    target_bps: u32,
) {
    e.as_contract(&vault.address, || {
        let strategies = storage::get_strategies(e);
        let mut updated = Vec::new(e);
        for info in strategies.iter() {
            let mut entry = info.clone();
            if entry.address == *strategy {
                entry.target_bps = target_bps;
            }
            updated.push_back(entry);
        }
        storage::set_strategies(e, &updated);
    });
}

fn deposit_to_vault(e: &Env, vault: &crate::FondexVaultClient, user: &Address, amount: i128) {
    e.mock_all_auths();
    vault.deposit(&amount, user, user);
}

#[test]
fn rebalance_invests_target_and_keeps_reserve() {
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

    mint(&e, &usdc, &user, 1_000 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 1_000 * ONE_USDC);

    rebalance_without_mock_auths(&e, &vault);

    assert_eq!(mock.balance(&vault.address), 900 * ONE_USDC);
    assert_eq!(vault.idle_assets(), 100 * ONE_USDC);
    assert_eq!(vault.total_assets(), 1_000 * ONE_USDC);
}

#[test]
fn rebalance_needs_no_signature() {
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
    deposit_to_vault(&e, &vault, &user, 100 * ONE_USDC);

    rebalance_without_mock_auths(&e, &vault);

    assert_eq!(mock.balance(&vault.address), 90 * ONE_USDC);
    assert_eq!(vault.idle_assets(), 10 * ONE_USDC);
}

#[test]
fn rebalance_authorizes_only_the_exact_transfer() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    mock.set_deposit_overpull(&1);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 100 * ONE_USDC);
    let idle_before = vault.idle_assets();

    rebalance_without_mock_auths(&e, &vault);

    assert_eq!(mock.balance(&vault.address), 0);
    assert_eq!(vault.idle_assets(), idle_before);
    assert_eq!(vault.total_assets(), 100 * ONE_USDC);
}

#[test]
fn rebalance_below_tolerance_moves_nothing() {
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

    mint(&e, &usdc, &user, 1_015 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 1_000 * ONE_USDC);
    rebalance_without_mock_auths(&e, &vault);

    deposit_to_vault(&e, &vault, &user, 15 * ONE_USDC);
    let strategy_before = mock.balance(&vault.address);
    let idle_before = vault.idle_assets();
    let total_before = vault.total_assets();

    rebalance_without_mock_auths(&e, &vault);

    let events = e.events().all().filter_by_contract(&vault.address);
    let rebalance_event = events
        .events()
        .iter()
        .find(|ev| event_name(&e, ev) == Symbol::new(&e, "rebalance"))
        .expect("rebalance event");

    assert_eq!(mock.balance(&vault.address), strategy_before);
    assert_eq!(vault.idle_assets(), idle_before);
    assert_eq!(vault.total_assets(), total_before);
    assert_eq!(
        event_i128_field(&e, rebalance_event, "total_assets_before"),
        total_before
    );
    assert_eq!(
        event_i128_field(&e, rebalance_event, "total_assets_after"),
        total_before
    );
}

#[test]
fn rebalance_skips_moves_below_min_strategy_move() {
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

    mint(&e, &usdc, &user, ONE_USDC);
    deposit_to_vault(&e, &vault, &user, ONE_USDC);

    rebalance_without_mock_auths(&e, &vault);

    assert_eq!(mock.balance(&vault.address), 0);
    assert_eq!(vault.idle_assets(), ONE_USDC);
}

#[test]
fn rebalance_with_deposit_refund_counts_measured_amount() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    let refund = 5_000_000_i128;
    mock.set_deposit_refund(&refund);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 100 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 100 * ONE_USDC);
    let total_before = vault.total_assets();

    rebalance_without_mock_auths(&e, &vault);

    let events = e.events().all().filter_by_contract(&vault.address);
    let deposit_event = events
        .events()
        .iter()
        .find(|ev| event_name(&e, ev) == Symbol::new(&e, "strategy_deposit"))
        .expect("strategy_deposit event");
    assert_eq!(topic_address(&e, deposit_event), mock.address);
    let expected_in_strategy = 90 * ONE_USDC - refund;
    assert_eq!(
        event_i128_field(&e, deposit_event, "amount"),
        expected_in_strategy
    );
    assert_eq!(mock.balance(&vault.address), expected_in_strategy);
    assert_eq!(vault.total_assets(), total_before);
}

#[test]
fn rebalance_with_withdraw_shortfall_loses_at_most_rounding() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    let shortfall = 30_i128;
    mock.set_withdraw_shortfall(&shortfall);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 1_000 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 1_000 * ONE_USDC);
    rebalance_without_mock_auths(&e, &vault);

    set_strategy_target_bps(&e, &vault, &mock.address, 5_000);
    let total_before = vault.total_assets();

    rebalance_without_mock_auths(&e, &vault);

    assert_eq!(vault.total_assets(), total_before - shortfall);
}

#[test]
fn rebalance_empties_paused_strategy_and_does_not_deposit_into_it() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_active = register_mock(&e, &usdc);
    let mock_paused = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![
            &e,
            strategy_info(&mock_active.address, 9_000, false),
            strategy_info(&mock_paused.address, 9_000, true),
        ],
    );

    mint(&e, &usdc, &user, 1_000 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 1_000 * ONE_USDC);
    super::fund_vault_in_strategy(&e, &mock_paused, &vault.address, 200 * ONE_USDC);

    rebalance_without_mock_auths(&e, &vault);

    assert_eq!(mock_paused.balance(&vault.address), 0);
    assert_eq!(mock_active.balance(&vault.address), 900 * ONE_USDC);
    assert_eq!(vault.idle_assets(), 100 * ONE_USDC);
}

#[test]
fn rebalance_moves_from_overweight_to_underweight() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock_b = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![
            &e,
            strategy_info(&mock_a.address, 4_500, false),
            strategy_info(&mock_b.address, 4_500, false),
        ],
    );

    mint(&e, &usdc, &user, 1_000 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 1_000 * ONE_USDC);
    super::fund_vault_in_strategy(&e, &mock_a, &vault.address, 1_000 * ONE_USDC);

    rebalance_without_mock_auths(&e, &vault);

    assert_eq!(mock_a.balance(&vault.address), 450 * ONE_USDC);
    assert_eq!(mock_b.balance(&vault.address), 450 * ONE_USDC);
    assert_eq!(vault.idle_assets(), 100 * ONE_USDC);
    assert_eq!(vault.total_assets(), 1_000 * ONE_USDC);
}

#[test]
fn rebalance_with_no_strategies_or_empty_vault_does_nothing() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);

    rebalance_without_mock_auths(&e, &vault);
    assert_eq!(vault.total_assets(), 0);

    let mock = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );
    rebalance_without_mock_auths(&e, &vault);
    assert_eq!(vault.total_assets(), 0);
    assert_eq!(mock.balance(&vault.address), 0);
}

#[test]
fn rebalance_emits_strategy_and_rebalance_events() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock_b = register_mock(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![
            &e,
            strategy_info(&mock_a.address, 4_500, false),
            strategy_info(&mock_b.address, 4_500, false),
        ],
    );

    mint(&e, &usdc, &user, 1_000 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 1_000 * ONE_USDC);
    super::fund_vault_in_strategy(&e, &mock_a, &vault.address, 1_000 * ONE_USDC);
    let total_before = vault.total_assets();

    rebalance_without_mock_auths(&e, &vault);

    let events = e.events().all().filter_by_contract(&vault.address);
    let names: std::vec::Vec<Symbol> = events
        .events()
        .iter()
        .map(|ev| event_name(&e, ev))
        .collect();
    assert!(names.contains(&Symbol::new(&e, "strategy_withdraw")));
    assert!(names.contains(&Symbol::new(&e, "strategy_deposit")));
    assert!(names.contains(&Symbol::new(&e, "rebalance")));

    let rebalance_event = events
        .events()
        .iter()
        .find(|ev| event_name(&e, ev) == Symbol::new(&e, "rebalance"))
        .expect("rebalance event");
    assert_eq!(
        event_i128_field(&e, rebalance_event, "total_assets_before"),
        total_before
    );
    assert_eq!(
        event_i128_field(&e, rebalance_event, "total_assets_after"),
        total_before
    );
}

#[test]
fn withdraw_after_rebalance_pulls_from_strategy() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    let token = usdc_client(&e, &usdc);
    set_vault_strategies(
        &e,
        &vault,
        vec![&e, strategy_info(&mock.address, 9_000, false)],
    );

    mint(&e, &usdc, &user, 1_000 * ONE_USDC);
    deposit_to_vault(&e, &vault, &user, 1_000 * ONE_USDC);
    rebalance_without_mock_auths(&e, &vault);

    let withdraw_amount = 150 * ONE_USDC;
    e.mock_all_auths();
    vault.withdraw(&withdraw_amount, &user, &user);

    assert_eq!(token.balance(&user), withdraw_amount);
    let pulled_from_strategy = withdraw_amount - 100 * ONE_USDC + WITHDRAW_ROUNDING_BUFFER;
    assert_eq!(
        mock.balance(&vault.address),
        900 * ONE_USDC - pulled_from_strategy
    );
    assert_eq!(vault.idle_assets(), WITHDRAW_ROUNDING_BUFFER);
}
