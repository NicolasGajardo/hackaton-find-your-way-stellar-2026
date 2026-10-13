extern crate std;

use soroban_sdk::{
    testutils::{
        Address as _, AuthorizedFunction, AuthorizedInvocation, Events, MockAuth, MockAuthInvoke,
    },
    vec, xdr, Address, Env, Error, IntoVal, MuxedAddress, String, Symbol, TryFromVal, Val, Vec,
};
use stellar_tokens::fungible::FungibleTokenError;

use crate::{FondexError, FondexVault, MAX_TIMELOCK_SECS};

use super::{
    fondex_error, mint, register_usdc, register_vault, register_vault_with_config, usdc_client,
    DECIMALS_OFFSET, ONE_USDC, SHARES_PER_USDC, USDC_DECIMALS, VIRTUAL_SHARES,
};

fn collapsed_host_error() -> Error {
    Error::from_type_and_code(xdr::ScErrorType::Context, xdr::ScErrorCode::InvalidAction)
}

fn contract_invocation(
    e: &Env,
    contract: &Address,
    fn_name: &str,
    args: impl IntoVal<Env, Vec<Val>>,
    sub_invocations: std::vec::Vec<AuthorizedInvocation>,
) -> AuthorizedInvocation {
    AuthorizedInvocation {
        function: AuthorizedFunction::Contract((
            contract.clone(),
            Symbol::new(e, fn_name),
            args.into_val(e),
        )),
        sub_invocations,
    }
}

fn fungible_insufficient_balance() -> Error {
    Error::from_contract_error(FungibleTokenError::InsufficientBalance as u32)
}

#[test]
fn first_deposit_one_usdc_mints_expected_shares() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &user, ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&ONE_USDC, &user, &user);
    let expected = ONE_USDC * VIRTUAL_SHARES;

    assert_eq!(shares, expected);
    assert_eq!(vault.balance(&user), shares);
    assert_eq!(vault.total_supply(), shares);
    assert_eq!(vault.total_assets(), ONE_USDC);
    assert_eq!(vault.decimals(), USDC_DECIMALS + DECIMALS_OFFSET);
    assert_eq!(vault.name(), String::from_str(&e, "Fondex Fund Share"));
    assert_eq!(vault.symbol(), String::from_str(&e, "fxFUND"));
    assert_eq!(vault.query_asset(), usdc);
    assert_eq!(vault.preview_deposit(&ONE_USDC), shares);
}

#[test]
fn multiple_users_deposit_and_withdraw_their_own() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    mint(&e, &usdc, &alice, 10 * ONE_USDC);
    mint(&e, &usdc, &bob, 30 * ONE_USDC);

    e.mock_all_auths();
    let alice_shares = vault.deposit(&(10 * ONE_USDC), &alice, &alice);
    let bob_shares = vault.deposit(&(30 * ONE_USDC), &bob, &bob);
    let alice_out = vault.redeem(&alice_shares, &alice, &alice);
    let bob_out = vault.redeem(&bob_shares, &bob, &bob);

    assert_eq!(alice_out, 10 * ONE_USDC);
    assert_eq!(bob_out, 30 * ONE_USDC);
    assert_eq!(vault.balance(&alice), 0);
    assert_eq!(vault.balance(&bob), 0);
    assert_eq!(token.balance(&alice), 10 * ONE_USDC);
    assert_eq!(token.balance(&bob), 30 * ONE_USDC);
}

#[test]
fn extra_usdc_raises_share_price_pro_rata() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let donor = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    mint(&e, &usdc, &alice, 10 * ONE_USDC);
    mint(&e, &usdc, &bob, 10 * ONE_USDC);
    mint(&e, &usdc, &donor, 10 * ONE_USDC);

    e.mock_all_auths();
    vault.deposit(&(10 * ONE_USDC), &alice, &alice);
    vault.deposit(&(10 * ONE_USDC), &bob, &bob);
    token.transfer(&donor, &vault.address, &(10 * ONE_USDC));
    assert_eq!(vault.total_assets(), 30 * ONE_USDC);

    let alice_out = vault.redeem(&vault.balance(&alice), &alice, &alice);
    let bob_out = vault.redeem(&vault.balance(&bob), &bob, &bob);

    assert!(alice_out > 10 * ONE_USDC);
    assert!(bob_out > 10 * ONE_USDC);
    assert!((alice_out - bob_out).abs() <= 1);
    assert_eq!(alice_out + bob_out + vault.total_assets(), 30 * ONE_USDC);
}

#[test]
fn full_redeem_clears_user_shares_without_error() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &user, 5 * ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&(5 * ONE_USDC), &user, &user);
    let out = vault.redeem(&shares, &user, &user);

    assert_eq!(vault.balance(&user), 0);
    assert_eq!(vault.total_supply(), 0);
    assert_eq!(vault.total_assets(), 0);
    assert_eq!(vault.max_redeem(&user), 0);
    assert_eq!(vault.max_withdraw(&user), 0);
    assert_eq!(out, 5 * ONE_USDC);
}

#[test]
fn cannot_withdraw_more_than_owned() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &alice, 10 * ONE_USDC);
    mint(&e, &usdc, &bob, 10 * ONE_USDC);

    e.mock_all_auths();
    vault.deposit(&(10 * ONE_USDC), &alice, &alice);
    vault.deposit(&(10 * ONE_USDC), &bob, &bob);

    assert_eq!(
        vault
            .try_withdraw(&(11 * ONE_USDC), &alice, &alice)
            .unwrap_err(),
        Ok(fungible_insufficient_balance())
    );
}

#[test]
fn cannot_redeem_another_users_shares_without_auth() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let victim = Address::generate(&e);
    let attacker = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &victim, ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&ONE_USDC, &victim, &victim);

    e.set_auths(&[]);
    e.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &vault.address,
            fn_name: "redeem",
            args: (shares, attacker.clone(), victim.clone()).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    assert_eq!(
        vault.try_redeem(&shares, &attacker, &victim).unwrap_err(),
        Ok(collapsed_host_error())
    );
}

#[test]
fn cannot_redeem_more_shares_than_owned() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &alice, 10 * ONE_USDC);
    mint(&e, &usdc, &bob, 10 * ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&(10 * ONE_USDC), &alice, &alice);
    vault.deposit(&(10 * ONE_USDC), &bob, &bob);

    assert_eq!(
        vault.try_redeem(&(shares + 1), &alice, &alice).unwrap_err(),
        Ok(fungible_insufficient_balance())
    );
}

#[test]
fn cannot_deposit_from_another_account_without_auth() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let victim = Address::generate(&e);
    let attacker = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    mint(&e, &usdc, &victim, ONE_USDC);

    e.set_auths(&[]);
    e.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &vault.address,
            fn_name: "deposit",
            args: (ONE_USDC, attacker.clone(), victim.clone()).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    assert_eq!(
        vault
            .try_deposit(&ONE_USDC, &attacker, &victim)
            .unwrap_err(),
        Ok(collapsed_host_error())
    );
    assert_eq!(token.balance(&victim), ONE_USDC);
    assert_eq!(token.balance(&attacker), 0);
    assert_eq!(vault.balance(&attacker), 0);
    assert_eq!(vault.total_assets(), 0);

    let usdc_transfer = MockAuthInvoke {
        contract: &usdc,
        fn_name: "transfer",
        args: (victim.clone(), MuxedAddress::from(&vault.address), ONE_USDC).into_val(&e),
        sub_invokes: &[],
    };
    e.set_auths(&[]);
    e.mock_auths(&[MockAuth {
        address: &victim,
        invoke: &MockAuthInvoke {
            contract: &vault.address,
            fn_name: "deposit",
            args: (ONE_USDC, attacker.clone(), victim.clone()).into_val(&e),
            sub_invokes: &[usdc_transfer],
        },
    }]);

    let shares = vault.deposit(&ONE_USDC, &attacker, &victim);
    assert_eq!(shares, SHARES_PER_USDC);
    assert_eq!(vault.balance(&attacker), SHARES_PER_USDC);
    assert_eq!(token.balance(&victim), 0);
    assert_eq!(vault.total_assets(), ONE_USDC);
}

#[test]
fn deposit_requires_from_auth() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let from = Address::generate(&e);
    let receiver = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &from, ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&ONE_USDC, &receiver, &from);
    let auths = e.auths();

    assert_eq!(shares, SHARES_PER_USDC);
    assert_eq!(vault.balance(&receiver), SHARES_PER_USDC);
    assert_eq!(vault.balance(&from), 0);
    assert_eq!(
        auths,
        std::vec![(
            from.clone(),
            contract_invocation(
                &e,
                &vault.address,
                "deposit",
                (ONE_USDC, receiver.clone(), from.clone()),
                std::vec![contract_invocation(
                    &e,
                    &usdc,
                    "transfer",
                    (from.clone(), MuxedAddress::from(&vault.address), ONE_USDC,),
                    std::vec![],
                )],
            ),
        )]
    );
}

#[test]
fn redeem_to_another_receiver_requires_owner_auth() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    mint(&e, &usdc, &alice, ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&ONE_USDC, &alice, &alice);
    let assets = vault.redeem(&shares, &bob, &alice);
    let auths = e.auths();

    assert_eq!(shares, SHARES_PER_USDC);
    assert_eq!(assets, ONE_USDC);
    assert_eq!(token.balance(&bob), ONE_USDC);
    assert_eq!(token.balance(&alice), 0);
    assert_eq!(vault.balance(&alice), 0);
    assert_eq!(
        auths,
        std::vec![(
            alice.clone(),
            contract_invocation(
                &e,
                &vault.address,
                "redeem",
                (shares, bob.clone(), alice.clone()),
                std::vec![],
            ),
        )]
    );
}

#[test]
fn share_transfer_requires_sender_auth() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &alice, ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&ONE_USDC, &alice, &alice);
    let to = MuxedAddress::from(&bob);
    vault.transfer(&alice, &to, &shares);
    let auths = e.auths();

    assert_eq!(shares, SHARES_PER_USDC);
    assert_eq!(vault.balance(&alice), 0);
    assert_eq!(vault.balance(&bob), SHARES_PER_USDC);
    assert_eq!(
        auths,
        std::vec![(
            alice.clone(),
            contract_invocation(
                &e,
                &vault.address,
                "transfer",
                (alice.clone(), to, shares),
                std::vec![],
            ),
        )]
    );
}

#[test]
fn inflation_attack_does_not_steal_victim_funds() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let attacker = Address::generate(&e);
    let victim = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    let donation = 1_000_000 * ONE_USDC;
    let victim_deposit = 100 * ONE_USDC;

    mint(&e, &usdc, &attacker, 1 + donation);
    mint(&e, &usdc, &victim, victim_deposit);

    e.mock_all_auths();
    vault.deposit(&1, &attacker, &attacker);
    token.transfer(&attacker, &vault.address, &donation);

    let victim_shares = vault.deposit(&victim_deposit, &victim, &victim);
    assert!(victim_shares > 0);

    let recovered = vault.redeem(&victim_shares, &victim, &victim);
    let lost = victim_deposit - recovered;
    assert!(lost * 1000 <= victim_deposit);
}

#[test]
fn zero_and_negative_amounts_are_rejected() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &user, ONE_USDC);
    e.mock_all_auths();

    let expected = fondex_error(FondexError::AmountMustBePositive);

    assert_eq!(
        vault.try_deposit(&0, &user, &user).unwrap_err(),
        Ok(expected)
    );
    assert_eq!(
        vault.try_deposit(&-1, &user, &user).unwrap_err(),
        Ok(expected)
    );
    assert_eq!(
        vault.try_withdraw(&0, &user, &user).unwrap_err(),
        Ok(expected)
    );
    assert_eq!(
        vault.try_withdraw(&-5, &user, &user).unwrap_err(),
        Ok(expected)
    );
    assert_eq!(
        vault.try_redeem(&0, &user, &user).unwrap_err(),
        Ok(expected)
    );
    assert_eq!(
        vault.try_redeem(&-2, &user, &user).unwrap_err(),
        Ok(expected)
    );
}

#[test]
fn deposit_then_immediate_withdraw_never_returns_more() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let deposit_amount = 123_456_789i128;
    mint(&e, &usdc, &user, deposit_amount);

    e.mock_all_auths();
    let shares = vault.deposit(&deposit_amount, &user, &user);
    let returned = vault.redeem(&shares, &user, &user);
    assert!(returned <= deposit_amount);
}

#[test]
fn shares_are_transferable_and_new_owner_can_redeem() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    mint(&e, &usdc, &alice, 8 * ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&(8 * ONE_USDC), &alice, &alice);
    vault.transfer(&alice, MuxedAddress::from(&bob), &shares);

    assert_eq!(vault.balance(&alice), 0);
    assert_eq!(vault.balance(&bob), shares);

    let out = vault.redeem(&shares, &bob, &bob);
    assert_eq!(out, 8 * ONE_USDC);
    assert_eq!(token.balance(&bob), 8 * ONE_USDC);
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")]
fn constructor_rejects_timelock_above_max() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    e.register(FondexVault, (usdc, owner, MAX_TIMELOCK_SECS + 1, i128::MAX));
}

#[test]
fn constructor_accepts_timelock_equal_to_max() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault_with_config(&e, &usdc, &owner, MAX_TIMELOCK_SECS, i128::MAX);
    assert_eq!(vault.strategy_timelock(), MAX_TIMELOCK_SECS);
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn constructor_rejects_zero_deposit_cap() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    e.register(FondexVault, (usdc, owner, 60_u64, 0_i128));
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn constructor_rejects_negative_deposit_cap() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    e.register(FondexVault, (usdc, owner, 60_u64, -1_i128));
}

#[test]
fn constructor_stores_config() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let cap = 5_000_000 * ONE_USDC;
    let vault = register_vault_with_config(&e, &usdc, &owner, 120_u64, cap);

    assert_eq!(vault.query_asset(), usdc);
    assert_eq!(vault.strategy_timelock(), 120);
    assert_eq!(vault.deposit_cap(), cap);
}

#[test]
fn deposit_too_small_for_one_share_fails_with_zero_shares() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    let donation = 10_i128.pow(11);
    mint(&e, &usdc, &user, donation + 1);

    e.mock_all_auths();
    token.transfer(&user, &vault.address, &donation);

    assert_eq!(
        vault.try_deposit(&1, &user, &user).unwrap_err(),
        Ok(fondex_error(FondexError::ZeroShares))
    );
}

#[test]
fn redeem_too_small_for_one_unit_fails_with_zero_assets() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &user, ONE_USDC);

    e.mock_all_auths();
    vault.deposit(&ONE_USDC, &user, &user);

    assert_eq!(
        vault.try_redeem(&1, &user, &user).unwrap_err(),
        Ok(fondex_error(FondexError::ZeroAssets))
    );
}

#[test]
fn withdraw_more_than_reserve_fails_with_insufficient_liquidity() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    mint(&e, &usdc, &user, ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&ONE_USDC, &user, &user);
    let usdc_before = token.balance(&user);

    assert_eq!(
        vault
            .try_withdraw(&(ONE_USDC + 1), &user, &user)
            .unwrap_err(),
        Ok(fondex_error(FondexError::InsufficientLiquidity))
    );
    assert_eq!(vault.balance(&user), shares);
    assert_eq!(token.balance(&user), usdc_before);
    assert_eq!(vault.total_assets(), ONE_USDC);
}

#[test]
fn preview_functions_round_in_favor_of_vault() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let donor = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    let deposit = 10 * ONE_USDC;
    let donation = 7 * ONE_USDC;
    mint(&e, &usdc, &alice, deposit);
    mint(&e, &usdc, &donor, donation);

    e.mock_all_auths();
    vault.deposit(&deposit, &alice, &alice);
    token.transfer(&donor, &vault.address, &donation);

    let assets = 3 * ONE_USDC;
    let total_assets = deposit + donation;
    let total_supply = vault.total_supply();
    let numerator_supply = total_supply + VIRTUAL_SHARES;
    let denominator_assets = total_assets + 1;
    let floor_shares = assets * numerator_supply / denominator_assets;
    let ceil_shares = (assets * numerator_supply + denominator_assets - 1) / denominator_assets;

    assert_eq!(vault.convert_to_shares(&assets), floor_shares);
    assert_eq!(vault.preview_withdraw(&assets), ceil_shares);
    assert!(vault.preview_withdraw(&assets) >= vault.convert_to_shares(&assets));

    let shares = vault.balance(&alice) / 4;
    let numerator_assets = total_assets + 1;
    let denominator_supply = total_supply + VIRTUAL_SHARES;
    let expected_assets = shares * numerator_assets / denominator_supply;
    assert_eq!(vault.preview_redeem(&shares), expected_assets);
    assert_eq!(vault.convert_to_assets(&shares), expected_assets);
}

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

fn event_data_assets_shares(e: &Env, event: &xdr::ContractEvent) -> (i128, i128) {
    let xdr::ContractEventBody::V0(body) = &event.body;
    let xdr::ScVal::Map(Some(map)) = &body.data else {
        panic!("expected map event data");
    };
    let mut assets = None;
    let mut shares = None;
    for entry in map.iter() {
        let xdr::ScVal::Symbol(sym) = &entry.key else {
            continue;
        };
        let field = sym.to_utf8_string().unwrap();
        let amount = i128::try_from_val(e, &scval_to_val(e, &entry.val)).unwrap();
        if field == "assets" {
            assets = Some(amount);
        } else if field == "shares" {
            shares = Some(amount);
        }
    }
    (assets.unwrap(), shares.unwrap())
}

#[test]
fn withdraw_to_another_receiver_requires_owner_auth() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    let token = usdc_client(&e, &usdc);
    mint(&e, &usdc, &alice, ONE_USDC);

    e.mock_all_auths();
    let deposited = vault.deposit(&ONE_USDC, &alice, &alice);
    let burned = vault.withdraw(&ONE_USDC, &bob, &alice);
    let auths = e.auths();

    assert_eq!(deposited, SHARES_PER_USDC);
    assert_eq!(burned, SHARES_PER_USDC);
    assert_eq!(token.balance(&bob), ONE_USDC);
    assert_eq!(token.balance(&alice), 0);
    assert_eq!(vault.balance(&alice), 0);
    assert_eq!(
        auths,
        std::vec![(
            alice.clone(),
            contract_invocation(
                &e,
                &vault.address,
                "withdraw",
                (ONE_USDC, bob.clone(), alice.clone()),
                std::vec![],
            ),
        )]
    );
}

#[test]
fn deposit_and_withdraw_emit_events() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let depositor = Address::generate(&e);
    let share_receiver = Address::generate(&e);
    let share_owner = Address::generate(&e);
    let payout_receiver = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &depositor, ONE_USDC);

    e.mock_all_auths();
    let shares = vault.deposit(&ONE_USDC, &share_receiver, &depositor);
    let deposit_name = Symbol::new(&e, "deposit");
    let deposit_events = e.events().all().filter_by_contract(&vault.address);
    let deposit_event = deposit_events
        .events()
        .iter()
        .find(|ev| event_name(&e, ev) == deposit_name)
        .expect("deposit event");
    assert_eq!(topic_address(&e, deposit_event, 1), depositor);
    assert_eq!(topic_address(&e, deposit_event, 2), share_receiver);
    assert_eq!(
        event_data_assets_shares(&e, deposit_event),
        (ONE_USDC, shares)
    );

    vault.transfer(&share_receiver, MuxedAddress::from(&share_owner), &shares);

    let withdraw_amount = ONE_USDC / 2;
    let burned = vault.withdraw(&withdraw_amount, &payout_receiver, &share_owner);
    let withdraw_name = Symbol::new(&e, "withdraw");
    let withdraw_events = e.events().all().filter_by_contract(&vault.address);
    let withdraw_event = withdraw_events
        .events()
        .iter()
        .find(|ev| event_name(&e, ev) == withdraw_name)
        .expect("withdraw event");
    assert_eq!(topic_address(&e, withdraw_event, 1), share_owner);
    assert_eq!(topic_address(&e, withdraw_event, 2), payout_receiver);
    assert_eq!(
        event_data_assets_shares(&e, withdraw_event),
        (withdraw_amount, burned)
    );
}

fn pausable_enforced_pause() -> Error {
    Error::from_contract_error(1000)
}

#[test]
fn deposit_total_plus_assets_overflow_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault_with_config(&e, &usdc, &owner, 60, i128::MAX);
    let huge = i128::MAX - 1;
    mint(&e, &usdc, &vault.address, huge);
    mint(&e, &usdc, &user, 2);
    e.mock_all_auths();
    assert_eq!(
        vault.try_deposit(&2, &user, &user).unwrap_err(),
        Ok(fondex_error(FondexError::DepositCapExceeded))
    );
}

#[test]
fn deposit_up_to_exact_cap_works_one_more_unit_fails() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let cap = 100 * ONE_USDC;
    let vault = register_vault_with_config(&e, &usdc, &owner, 60, cap);
    mint(&e, &usdc, &user, 200 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&cap, &user, &user);
    assert_eq!(
        vault.try_deposit(&ONE_USDC, &user, &user).unwrap_err(),
        Ok(fondex_error(FondexError::DepositCapExceeded))
    );
}

#[test]
fn deposit_cap_counts_current_value_not_deposits() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault_with_config(&e, &usdc, &owner, 60, 90 * ONE_USDC);
    let mock = super::register_mock(&e, &usdc);
    super::set_vault_strategies(
        &e,
        &vault,
        vec![&e, super::strategy_info(&mock.address, 9_000, false)],
    );
    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(50 * ONE_USDC), &user, &user);
    super::rebalance_without_mock_auths(&e, &vault);
    super::add_yield(&e, &usdc, &mock, &vault.address, 10_000);
    assert_eq!(vault.max_deposit(&user), 0);
}

#[test]
fn lowering_cap_below_total_blocks_deposits_but_not_withdrawals() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &user, 100 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(50 * ONE_USDC), &user, &user);
    super::mock_owner_auth(&e, &vault, &owner, "set_deposit_cap", (10 * ONE_USDC,));
    vault.set_deposit_cap(&(10 * ONE_USDC));
    e.mock_all_auths();
    assert_eq!(
        vault.try_deposit(&ONE_USDC, &user, &user).unwrap_err(),
        Ok(fondex_error(FondexError::DepositCapExceeded))
    );
    vault.withdraw(&(10 * ONE_USDC), &user, &user);
}

#[test]
fn max_deposit_matches_what_can_be_deposited() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault_with_config(&e, &usdc, &owner, 60, 100 * ONE_USDC);
    mint(&e, &usdc, &user, 200 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(40 * ONE_USDC), &user, &user);
    let room = vault.max_deposit(&user);
    vault.deposit(&room, &user, &user);
    assert_eq!(
        vault.try_deposit(&ONE_USDC, &user, &user).unwrap_err(),
        Ok(fondex_error(FondexError::DepositCapExceeded))
    );
    super::mock_owner_auth(&e, &vault, &owner, "pause", ());
    vault.pause();
    assert_eq!(vault.max_deposit(&user), 0);
}

#[test]
fn set_deposit_cap_rejects_non_positive() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    super::mock_owner_auth(&e, &vault, &owner, "set_deposit_cap", (0_i128,));
    assert_eq!(
        vault.try_set_deposit_cap(&0).unwrap_err(),
        Ok(fondex_error(FondexError::InvalidDepositCap))
    );
}

#[test]
fn paused_blocks_deposit_and_rebalance() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &user, ONE_USDC);
    super::mock_owner_auth(&e, &vault, &owner, "pause", ());
    vault.pause();
    e.mock_all_auths();
    assert_eq!(
        vault.try_deposit(&ONE_USDC, &user, &user).unwrap_err(),
        Ok(pausable_enforced_pause())
    );
    assert_eq!(
        vault.try_rebalance().unwrap_err(),
        Ok(pausable_enforced_pause())
    );
}

#[test]
fn paused_still_allows_withdraw_and_redeem() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &user, 10 * ONE_USDC);
    e.mock_all_auths();
    vault.deposit(&(10 * ONE_USDC), &user, &user);
    super::mock_owner_auth(&e, &vault, &owner, "pause", ());
    vault.pause();
    e.mock_all_auths();
    vault.withdraw(&(5 * ONE_USDC), &user, &user);
    let shares = vault.balance(&user);
    vault.redeem(&shares, &user, &user);
}

#[test]
fn unpause_restores_deposits() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let user = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault(&e, &usdc, &owner);
    mint(&e, &usdc, &user, ONE_USDC);
    super::mock_owner_auth(&e, &vault, &owner, "pause", ());
    vault.pause();
    super::mock_owner_auth(&e, &vault, &owner, "unpause", ());
    vault.unpause();
    e.mock_all_auths();
    vault.deposit(&ONE_USDC, &user, &user);
}
