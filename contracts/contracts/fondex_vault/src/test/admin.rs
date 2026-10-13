extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke},
    xdr, Address, Env, IntoVal, Symbol, TryFromVal, Val,
};

use crate::{FondexError, BPS, MAX_STRATEGIES, MIN_IDLE_BPS};

use super::mock_strategy::MockStrategyClient;
use super::{
    add_strategy, fondex_error, mint, mock_owner_auth, rebalance_without_mock_auths, register_mock,
    register_usdc, register_vault, set_vault_strategies, strategy_info, ONE_USDC,
};

fn scval_to_val(e: &Env, v: &xdr::ScVal) -> Val {
    Val::try_from_val(e, v).unwrap()
}

fn event_name(e: &Env, event: &xdr::ContractEvent) -> Symbol {
    let xdr::ContractEventBody::V0(body) = &event.body;
    Symbol::try_from_val(e, &scval_to_val(e, &body.topics[0])).unwrap()
}

fn topic_address(e: &Env, event: &xdr::ContractEvent, index: usize) -> Address {
    let xdr::ContractEventBody::V0(body) = &event.body;
    Address::try_from_val(e, &scval_to_val(e, &body.topics[index])).unwrap()
}

fn event_map_u32(e: &Env, event: &xdr::ContractEvent, field: &str) -> u32 {
    let xdr::ContractEventBody::V0(body) = &event.body;
    let val = Val::try_from_val(e, &body.data).unwrap();
    let map: soroban_sdk::Map<Symbol, u32> = soroban_sdk::Map::try_from_val(e, &val).unwrap();
    map.get(Symbol::new(e, field)).unwrap()
}

fn event_map_u64(e: &Env, event: &xdr::ContractEvent, field: &str) -> u64 {
    let xdr::ContractEventBody::V0(body) = &event.body;
    let val = Val::try_from_val(e, &body.data).unwrap();
    let map: soroban_sdk::Map<Symbol, u64> = soroban_sdk::Map::try_from_val(e, &val).unwrap();
    map.get(Symbol::new(e, field)).unwrap()
}

fn event_map_i128(e: &Env, event: &xdr::ContractEvent, field: &str) -> i128 {
    let xdr::ContractEventBody::V0(body) = &event.body;
    let val = Val::try_from_val(e, &body.data).unwrap();
    let map: soroban_sdk::Map<Symbol, i128> = soroban_sdk::Map::try_from_val(e, &val).unwrap();
    map.get(Symbol::new(e, field)).unwrap()
}

fn latest_event_named(
    e: &Env,
    vault: &Address,
    name: &Symbol,
    label: &'static str,
) -> xdr::ContractEvent {
    e.events()
        .all()
        .filter_by_contract(vault)
        .events()
        .iter()
        .rfind(|ev| event_name(e, ev) == *name)
        .unwrap_or_else(|| panic!("missing event: {label}"))
        .clone()
}

fn collapsed_host_error() -> soroban_sdk::Error {
    soroban_sdk::Error::from_type_and_code(
        xdr::ScErrorType::Context,
        xdr::ScErrorCode::InvalidAction,
    )
}

fn assert_outsider_cannot_admin(
    e: &Env,
    vault: &crate::FondexVaultClient,
    outsider: &Address,
    fn_name: &str,
    args: impl IntoVal<Env, soroban_sdk::Vec<Val>>,
) {
    e.set_auths(&[]);
    e.mock_auths(&[MockAuth {
        address: outsider,
        invoke: &MockAuthInvoke {
            contract: &vault.address,
            fn_name,
            args: args.into_val(e),
            sub_invokes: &[],
        },
    }]);
}

#[test]
fn admin_functions_require_owner_auth() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let outsider = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);

    let cases: [(&str, soroban_sdk::Vec<Val>); 11] = [
        (
            "propose_strategy",
            (mock.address.clone(), 1_000u32).into_val(&e),
        ),
        ("cancel_strategy", ().into_val(&e)),
        ("accept_strategy", ().into_val(&e)),
        ("set_target", (mock.address.clone(), 1_000u32).into_val(&e)),
        ("pause_strategy", (mock.address.clone(),).into_val(&e)),
        ("unpause_strategy", (mock.address.clone(),).into_val(&e)),
        ("remove_strategy", (mock.address.clone(),).into_val(&e)),
        ("set_deposit_cap", (100 * ONE_USDC,).into_val(&e)),
        ("pause", ().into_val(&e)),
        ("unpause", ().into_val(&e)),
        ("transfer_ownership", (outsider.clone(), 0u32).into_val(&e)),
    ];

    for (fn_name, args) in cases {
        assert_outsider_cannot_admin(&e, &vault, &outsider, fn_name, args);
        let err = match fn_name {
            "propose_strategy" => vault
                .try_propose_strategy(&mock.address, &1_000u32)
                .unwrap_err(),
            "cancel_strategy" => vault.try_cancel_strategy().unwrap_err(),
            "accept_strategy" => vault.try_accept_strategy().unwrap_err(),
            "set_target" => vault.try_set_target(&mock.address, &1_000u32).unwrap_err(),
            "pause_strategy" => vault.try_pause_strategy(&mock.address).unwrap_err(),
            "unpause_strategy" => vault.try_unpause_strategy(&mock.address).unwrap_err(),
            "remove_strategy" => vault.try_remove_strategy(&mock.address).unwrap_err(),
            "set_deposit_cap" => vault.try_set_deposit_cap(&(100 * ONE_USDC)).unwrap_err(),
            "pause" => vault.try_pause().unwrap_err(),
            "unpause" => vault.try_unpause().unwrap_err(),
            "transfer_ownership" => vault.try_transfer_ownership(&outsider, &0u32).unwrap_err(),
            _ => unreachable!(),
        };
        assert_eq!(err, Ok(collapsed_host_error()));
    }
}

#[test]
fn accept_strategy_before_timelock_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);

    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 4_500u32),
    );
    vault.propose_strategy(&mock.address, &4_500u32);
    mock_owner_auth(&e, &vault, &owner, "accept_strategy", ());
    assert_eq!(
        vault.try_accept_strategy().unwrap_err(),
        Ok(fondex_error(FondexError::TimelockNotExpired))
    );
}

#[test]
fn accept_strategy_after_timelock_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);

    add_strategy(&e, &vault, &owner, &mock.address, 4_500);
    assert_eq!(vault.strategies().len(), 1);
    assert!(vault.pending_strategy().is_none());
}

#[test]
fn propose_strategy_with_other_asset_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let other = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &other);

    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 1_000u32),
    );
    assert_eq!(
        vault
            .try_propose_strategy(&mock.address, &1_000u32)
            .unwrap_err(),
        Ok(fondex_error(FondexError::AssetMismatch))
    );
}

#[test]
fn allocation_above_max_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock = register_mock(&e, &usdc);
    let max_bps = BPS - MIN_IDLE_BPS;

    add_strategy(&e, &vault, &owner, &mock_a.address, max_bps - 1_000);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 2_000u32),
    );
    assert_eq!(
        vault
            .try_propose_strategy(&mock.address, &2_000u32)
            .unwrap_err(),
        Ok(fondex_error(FondexError::AllocationTooHigh))
    );

    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "set_target",
        (mock_a.address.clone(), max_bps + 1),
    );
    assert_eq!(
        vault
            .try_set_target(&mock_a.address, &(max_bps + 1))
            .unwrap_err(),
        Ok(fondex_error(FondexError::AllocationTooHigh))
    );
}

#[test]
fn fifth_strategy_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mocks: std::vec::Vec<MockStrategyClient> = (0..MAX_STRATEGIES)
        .map(|_| register_mock(&e, &usdc))
        .collect();
    for mock in &mocks {
        add_strategy(&e, &vault, &owner, &mock.address, 1_000);
    }
    let extra = register_mock(&e, &usdc);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (extra.address.clone(), 500u32),
    );
    assert_eq!(
        vault
            .try_propose_strategy(&extra.address, &500u32)
            .unwrap_err(),
        Ok(fondex_error(FondexError::TooManyStrategies))
    );
}

#[test]
fn duplicate_strategy_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    add_strategy(&e, &vault, &owner, &mock.address, 1_000);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 500u32),
    );
    assert_eq!(
        vault
            .try_propose_strategy(&mock.address, &500u32)
            .unwrap_err(),
        Ok(fondex_error(FondexError::StrategyAlreadyExists))
    );
}

#[test]
fn second_pending_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock = register_mock(&e, &usdc);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock_a.address.clone(), 1_000u32),
    );
    vault.propose_strategy(&mock_a.address, &1_000u32);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 1_000u32),
    );
    assert_eq!(
        vault
            .try_propose_strategy(&mock.address, &1_000u32)
            .unwrap_err(),
        Ok(fondex_error(FondexError::PendingStrategyExists))
    );
}

#[test]
fn cancel_or_accept_without_pending_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mock_owner_auth(&e, &vault, &owner, "cancel_strategy", ());
    assert_eq!(
        vault.try_cancel_strategy().unwrap_err(),
        Ok(fondex_error(FondexError::NoPendingStrategy))
    );
    mock_owner_auth(&e, &vault, &owner, "accept_strategy", ());
    assert_eq!(
        vault.try_accept_strategy().unwrap_err(),
        Ok(fondex_error(FondexError::NoPendingStrategy))
    );
}

#[test]
fn set_target_unknown_strategy_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "set_target",
        (mock.address.clone(), 1_000u32),
    );
    assert_eq!(
        vault.try_set_target(&mock.address, &1_000u32).unwrap_err(),
        Ok(fondex_error(FondexError::StrategyNotFound))
    );
}

#[test]
fn paused_strategy_still_counts_in_allocation() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock = register_mock(&e, &usdc);
    add_strategy(&e, &vault, &owner, &mock_a.address, 4_000);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "pause_strategy",
        (mock_a.address.clone(),),
    );
    vault.pause_strategy(&mock_a.address);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 6_000u32),
    );
    assert_eq!(
        vault
            .try_propose_strategy(&mock.address, &6_000u32)
            .unwrap_err(),
        Ok(fondex_error(FondexError::AllocationTooHigh))
    );
}

#[test]
fn remove_strategy_divests_and_removes() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    add_strategy(&e, &vault, &owner, &mock.address, 9_000);
    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    rebalance_without_mock_auths(&e, &vault);
    let total_before = vault.total_assets();
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "remove_strategy",
        (mock.address.clone(),),
    );
    vault.remove_strategy(&mock.address);
    assert_eq!(vault.strategies().len(), 0);
    assert_eq!(vault.total_assets(), total_before);
    assert_eq!(vault.idle_assets(), total_before);
}

#[test]
fn remove_strategy_with_balance_above_dust_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    mock.set_withdraw_cap(&0);
    add_strategy(&e, &vault, &owner, &mock.address, 9_000);
    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(100 * ONE_USDC), &user, &user);
    rebalance_without_mock_auths(&e, &vault);
    let strategies_before = vault.strategies();
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "remove_strategy",
        (mock.address.clone(),),
    );
    assert_eq!(
        vault.try_remove_strategy(&mock.address).unwrap_err(),
        Ok(fondex_error(FondexError::StrategyNotEmpty))
    );
    assert_eq!(vault.strategies(), strategies_before);
}

#[test]
fn unavailable_strategy_cannot_be_removed() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    add_strategy(&e, &vault, &owner, &mock.address, 9_000);
    mock.set_balance_fails(&true);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "remove_strategy",
        (mock.address.clone(),),
    );
    assert_eq!(
        vault.try_remove_strategy(&mock.address).unwrap_err(),
        Ok(fondex_error(FondexError::StrategyUnavailable))
    );
}

#[test]
fn ownership_transfer_two_steps() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let new_owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);

    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "transfer_ownership",
        (new_owner.clone(), 1_000_000u32),
    );
    vault.transfer_ownership(&new_owner, &1_000_000u32);
    e.set_auths(&[]);
    e.mock_auths(&[MockAuth {
        address: &new_owner,
        invoke: &MockAuthInvoke {
            contract: &vault.address,
            fn_name: "accept_ownership",
            args: ().into_val(&e),
            sub_invokes: &[],
        },
    }]);
    vault.accept_ownership();
    assert_eq!(vault.get_owner(), Some(new_owner.clone()));

    mock_owner_auth(
        &e,
        &vault,
        &new_owner,
        "propose_strategy",
        (mock.address.clone(), 1_000u32),
    );
    vault.propose_strategy(&mock.address, &1_000u32);
}

#[test]
fn admin_functions_emit_events() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock = register_mock(&e, &usdc);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 4_000u32),
    );
    vault.propose_strategy(&mock.address, &4_000u32);
    let proposed = Symbol::new(&e, "strategy_proposed");
    let ev = latest_event_named(&e, &vault.address, &proposed, "proposed");
    let pending = vault.pending_strategy().expect("pending");
    assert_eq!(topic_address(&e, &ev, 1), mock.address);
    assert_eq!(event_map_u32(&e, &ev, "target_bps"), 4_000);
    assert_eq!(event_map_u64(&e, &ev, "eta"), pending.eta);

    mock_owner_auth(&e, &vault, &owner, "cancel_strategy", ());
    vault.cancel_strategy();
    let cancelled = Symbol::new(&e, "strategy_cancelled");
    let ev = latest_event_named(&e, &vault.address, &cancelled, "cancelled");
    assert_eq!(topic_address(&e, &ev, 1), mock.address);

    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 4_000u32),
    );
    vault.propose_strategy(&mock.address, &4_000u32);
    let pending_accept = vault.pending_strategy().expect("pending");
    e.ledger().with_mut(|ledger| {
        ledger.timestamp = pending_accept.eta;
    });
    mock_owner_auth(&e, &vault, &owner, "accept_strategy", ());
    vault.accept_strategy();
    let accepted = Symbol::new(&e, "strategy_accepted");
    let ev = latest_event_named(&e, &vault.address, &accepted, "accepted");
    assert_eq!(topic_address(&e, &ev, 1), mock.address);
    assert_eq!(event_map_u32(&e, &ev, "target_bps"), 4_000);

    let mock_a = register_mock(&e, &usdc);
    add_strategy(&e, &vault, &owner, &mock_a.address, 3_000);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "set_target",
        (mock_a.address.clone(), 4_000u32),
    );
    vault.set_target(&mock_a.address, &4_000u32);
    let target_changed = Symbol::new(&e, "target_changed");
    let ev = latest_event_named(&e, &vault.address, &target_changed, "target_changed");
    assert_eq!(topic_address(&e, &ev, 1), mock_a.address);
    assert_eq!(event_map_u32(&e, &ev, "old_bps"), 3_000);
    assert_eq!(event_map_u32(&e, &ev, "new_bps"), 4_000);

    let paused = Symbol::new(&e, "strategy_paused");
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "pause_strategy",
        (mock_a.address.clone(),),
    );
    vault.pause_strategy(&mock_a.address);
    let paused_events_after_first = e
        .events()
        .all()
        .filter_by_contract(&vault.address)
        .events()
        .iter()
        .filter(|ev| event_name(&e, ev) == paused)
        .count();
    assert_eq!(paused_events_after_first, 1);
    let ev = latest_event_named(&e, &vault.address, &paused, "paused");
    assert_eq!(topic_address(&e, &ev, 1), mock_a.address);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "pause_strategy",
        (mock_a.address.clone(),),
    );
    vault.pause_strategy(&mock_a.address);
    let still_paused = vault
        .strategies()
        .iter()
        .find(|info| info.address == mock_a.address)
        .expect("strategy")
        .paused;
    assert!(still_paused);

    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "unpause_strategy",
        (mock_a.address.clone(),),
    );
    vault.unpause_strategy(&mock_a.address);
    let unpaused = Symbol::new(&e, "strategy_unpaused");
    let ev = latest_event_named(&e, &vault.address, &unpaused, "unpaused");
    assert_eq!(topic_address(&e, &ev, 1), mock_a.address);

    let new_cap = 50 * ONE_USDC;
    let old_cap = vault.deposit_cap();
    mock_owner_auth(&e, &vault, &owner, "set_deposit_cap", (new_cap,));
    vault.set_deposit_cap(&new_cap);
    let cap_changed = Symbol::new(&e, "deposit_cap_changed");
    let ev = latest_event_named(&e, &vault.address, &cap_changed, "cap_changed");
    assert_eq!(event_map_i128(&e, &ev, "old_cap"), old_cap);
    assert_eq!(event_map_i128(&e, &ev, "new_cap"), new_cap);

    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "remove_strategy",
        (mock.address.clone(),),
    );
    vault.remove_strategy(&mock.address);
    let removed = Symbol::new(&e, "strategy_removed");
    let ev = latest_event_named(&e, &vault.address, &removed, "removed");
    assert_eq!(topic_address(&e, &ev, 1), mock.address);
    assert_eq!(event_map_i128(&e, &ev, "dust_left"), 0);
}

#[test]
fn accept_strategy_revalidates_allocation() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock = register_mock(&e, &usdc);
    add_strategy(&e, &vault, &owner, &mock_a.address, 5_000);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "propose_strategy",
        (mock.address.clone(), 4_000u32),
    );
    vault.propose_strategy(&mock.address, &4_000u32);
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "set_target",
        (mock_a.address.clone(), 6_000u32),
    );
    vault.set_target(&mock_a.address, &6_000u32);
    let pending = vault.pending_strategy().expect("pending");
    e.ledger().with_mut(|ledger| {
        ledger.timestamp = pending.eta;
    });
    mock_owner_auth(&e, &vault, &owner, "accept_strategy", ());
    assert_eq!(
        vault.try_accept_strategy().unwrap_err(),
        Ok(fondex_error(FondexError::AllocationTooHigh))
    );
}

#[test]
fn set_target_bps_sum_overflow_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let mock_a = register_mock(&e, &usdc);
    let mock = register_mock(&e, &usdc);
    let half = u32::MAX / 2 + 1;
    set_vault_strategies(
        &e,
        &vault,
        soroban_sdk::vec![
            &e,
            strategy_info(&mock_a.address, half, false),
            strategy_info(&mock.address, half, false),
        ],
    );
    mock_owner_auth(
        &e,
        &vault,
        &owner,
        "set_target",
        (mock_a.address.clone(), half - 1),
    );
    assert_eq!(
        vault
            .try_set_target(&mock_a.address, &(half - 1))
            .unwrap_err(),
        Ok(fondex_error(FondexError::AllocationTooHigh))
    );
}
