extern crate std;

use std::string::ToString;

use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig},
    Address, ConversionError, Env, Error, I256,
};
use stellar_tokens::fungible::FungibleTokenError;

use crate::{FondexVaultClient, BPS, MIN_IDLE_BPS};

use super::mock_strategy::MockStrategyClient;
use super::{
    add_strategy, add_yield, fondex_error, mint, mock_owner_auth, register_mock, register_usdc,
    register_vault_with_config, usdc_client, FondexError, ONE_USDC, VIRTUAL_SHARES,
};

const NUM_USERS: usize = 4;
const USER_USDC: i128 = 1_000_000 * ONE_USDC;
const OP_COUNT_CI: u32 = 40;
const SEED_COUNT_CI: u64 = 64;
const OP_COUNT_LONG: u32 = 60;
const SEED_COUNT_LONG: u64 = 500;

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn gen_i128_up_to(&mut self, max_inclusive: i128) -> i128 {
        if max_inclusive <= 0 {
            return 1;
        }
        let span = max_inclusive as u64;
        (self.next_u64() % span.max(1) + 1) as i128
    }
}

#[derive(Clone, Copy, Debug)]
enum OpKind {
    Deposit,
    Withdraw,
    Redeem,
    Transfer,
    Rebalance,
    Yield,
    Loss,
    Donate,
    WithdrawCap,
    BalanceFails,
    PauseStrategy,
    UnpauseStrategy,
    SetTarget,
    PauseVault,
    UnpauseVault,
}

#[derive(Clone, Copy, Debug)]
enum ShareValuePolicy {
    Strict,
    Exempt,
    RebalanceRoundingSlack,
}

struct Harness {
    seed: u64,
    step: u32,
    log: std::vec::Vec<std::string::String>,
    rng: Rng,
    total_minted: i128,
    withdraw_shortfall: i128,
    exact_rounding: bool,
    shareholders: std::vec::Vec<Address>,
    withdraw_caps: [i128; 2],
}

impl Harness {
    fn fail(&self, msg: &str) -> ! {
        panic!(
            "seed={} step={} — {}\noperation log:\n{}",
            self.seed,
            self.step,
            msg,
            self.log.join("\n")
        );
    }

    fn record_mint(&mut self, amount: i128) {
        if amount > 0 {
            self.total_minted = self.total_minted.checked_add(amount).unwrap_or(i128::MAX);
        }
    }

    fn log_op(&mut self, line: &str) {
        self.log.push(line.to_string());
    }

    fn track_holder(&mut self, addr: &Address) {
        if !self.shareholders.iter().any(|a| a == addr) {
            self.shareholders.push(addr.clone());
        }
    }
}

fn pausable_enforced_pause() -> Error {
    Error::from_contract_error(1000)
}

fn fungible_insufficient_balance() -> Error {
    Error::from_contract_error(FungibleTokenError::InsufficientBalance as u32)
}

fn acceptable_fondex(err: &Error) -> bool {
    *err == fondex_error(FondexError::InsufficientLiquidity)
        || *err == fondex_error(FondexError::ZeroShares)
        || *err == fondex_error(FondexError::ZeroAssets)
        || *err == fondex_error(FondexError::StrategyUnavailable)
        || *err == fondex_error(FondexError::AllocationTooHigh)
}

fn contract_err_acceptable<T, HostErr: core::fmt::Debug>(
    result: Result<Result<T, Error>, Result<Error, HostErr>>,
    h: &Harness,
    op: &str,
    acceptable: impl Fn(&Error) -> bool,
) -> Option<T> {
    match result {
        Ok(Ok(v)) => Some(v),
        Ok(Err(e)) => {
            if acceptable(&e) {
                None
            } else {
                h.fail(&std::format!("{op} error: {e:?}"));
            }
        }
        Err(Ok(e)) => {
            if acceptable(&e) {
                None
            } else {
                h.fail(&std::format!("{op} error (outer): {e:?}"));
            }
        }
        Err(Err(host)) => h.fail(&std::format!("{op} host: {host:?}")),
    }
}

fn conversion_err_acceptable<T, HostErr: core::fmt::Debug>(
    result: Result<Result<T, ConversionError>, Result<Error, HostErr>>,
    h: &Harness,
    op: &str,
    acceptable: impl Fn(&Error) -> bool,
) -> Option<T> {
    match result {
        Ok(Ok(v)) => Some(v),
        Ok(Err(c)) => {
            let e: Error = c.into();
            if acceptable(&e) {
                None
            } else {
                h.fail(&std::format!("{op} error: {e:?}"));
            }
        }
        Err(Ok(e)) => {
            if acceptable(&e) {
                None
            } else {
                h.fail(&std::format!("{op} error (outer): {e:?}"));
            }
        }
        Err(Err(host)) => h.fail(&std::format!("{op} host: {host:?}")),
    }
}

fn max_allocation_bps() -> u32 {
    BPS - MIN_IDLE_BPS
}

fn sum_target_bps(vault: &FondexVaultClient) -> u32 {
    let mut sum = 0_u32;
    for info in vault.strategies().iter() {
        sum = sum.saturating_add(info.target_bps);
    }
    sum
}

fn usdc_in_circulation(e: &Env, usdc: &Address, addrs: &[Address]) -> i128 {
    let token = usdc_client(e, usdc);
    addrs.iter().map(|a| token.balance(a)).sum()
}

fn tracked_usdc_accounts(
    users: &[Address],
    owner: &Address,
    fondex: &Address,
    vault: &FondexVaultClient,
    mocks: &[MockStrategyClient],
) -> std::vec::Vec<Address> {
    let mut v = users.to_vec();
    v.push(owner.clone());
    v.push(fondex.clone());
    v.push(vault.address.clone());
    for m in mocks {
        v.push(m.address.clone());
    }
    v
}

fn check_usdc_conservation(e: &Env, usdc: &Address, h: &Harness, accounts: &[Address]) {
    let sum = usdc_in_circulation(e, usdc, accounts);
    if sum != h.total_minted {
        h.fail(&std::format!(
            "USDC conservation: sum balances={sum} != total_minted={}",
            h.total_minted
        ));
    }
}

fn check_allocation(vault: &FondexVaultClient, h: &Harness) {
    let sum = sum_target_bps(vault);
    if sum > max_allocation_bps() {
        h.fail(&std::format!(
            "allocation: sum target_bps={sum} > max {}",
            max_allocation_bps()
        ));
    }
}

fn check_share_supply(vault: &FondexVaultClient, h: &Harness) {
    let mut sum = 0_i128;
    for user in &h.shareholders {
        sum = sum.saturating_add(vault.balance(user));
    }
    if sum != vault.total_supply() {
        h.fail(&std::format!(
            "share supply: sum balances={sum} != total_supply={}",
            vault.total_supply()
        ));
    }
}

fn check_total_assets_consistent(
    vault: &FondexVaultClient,
    mocks: &[MockStrategyClient],
    balance_fails: &[bool],
    h: &Harness,
) {
    if balance_fails.iter().any(|f| *f) {
        return;
    }
    let idle = vault.idle_assets();
    let mut strat_sum = 0_i128;
    for mock in mocks {
        strat_sum = strat_sum.saturating_add(mock.balance(&vault.address));
    }
    let total = vault.total_assets();
    if idle + strat_sum != total {
        h.fail(&std::format!(
            "total assets: idle={idle} + strategies={strat_sum} != total_assets={total}"
        ));
    }
}

fn check_share_value(
    e: &Env,
    ta_before: i128,
    supply_before: i128,
    ta_after: i128,
    supply_after: i128,
    policy: ShareValuePolicy,
    h: &Harness,
) {
    let lhs =
        I256::from_i128(e, ta_after + 1).mul(&I256::from_i128(e, supply_before + VIRTUAL_SHARES));
    let rhs =
        I256::from_i128(e, ta_before + 1).mul(&I256::from_i128(e, supply_after + VIRTUAL_SHARES));
    if lhs >= rhs {
        return;
    }
    match policy {
        ShareValuePolicy::Exempt => return,
        ShareValuePolicy::RebalanceRoundingSlack => {
            let max_ta_drop = h.withdraw_shortfall.saturating_mul(2);
            if !h.exact_rounding && ta_before.saturating_sub(ta_after) <= max_ta_drop {
                return;
            }
        }
        ShareValuePolicy::Strict => {}
    }
    h.fail(&std::format!(
        "share value decreased: ta {ta_before}->{ta_after} supply {supply_before}->{supply_after}"
    ));
}

fn liquidity_failure_explained(
    vault: &FondexVaultClient,
    mocks: &[MockStrategyClient],
    balance_fails: &[bool],
    withdraw_caps: &[i128],
    amount: i128,
) -> bool {
    let idle = vault.idle_assets();
    let mut conservative = idle;
    for (i, mock) in mocks.iter().enumerate() {
        if balance_fails[i] {
            continue;
        }
        let bal = mock.balance(&vault.address);
        let capped = if withdraw_caps[i] == i128::MAX {
            bal
        } else {
            core::cmp::min(bal, withdraw_caps[i])
        };
        conservative = conservative.saturating_add(capped);
    }
    if conservative < amount {
        return true;
    }
    balance_fails.iter().any(|b| *b) || withdraw_caps.iter().any(|c| *c != i128::MAX && *c < amount)
}

fn setup_mocks_rounding(seed: u64, mocks: &[MockStrategyClient], rng: &mut Rng) -> (i128, bool) {
    let exact = seed.is_multiple_of(2);
    let shortfall = if exact {
        0_i128
    } else {
        rng.gen_i128_up_to(50)
    };
    for mock in mocks {
        mock.set_deposit_refund(&shortfall);
        mock.set_withdraw_shortfall(&shortfall);
    }
    (shortfall, exact)
}

fn pick_op(rng: &mut Rng) -> OpKind {
    match rng.next_u64() % 15 {
        0 => OpKind::Deposit,
        1 => OpKind::Withdraw,
        2 => OpKind::Redeem,
        3 => OpKind::Transfer,
        4 => OpKind::Rebalance,
        5 => OpKind::Yield,
        6 => OpKind::Loss,
        7 => OpKind::Donate,
        8 => OpKind::WithdrawCap,
        9 => OpKind::BalanceFails,
        10 => OpKind::PauseStrategy,
        11 => OpKind::UnpauseStrategy,
        12 => OpKind::SetTarget,
        13 => OpKind::PauseVault,
        _ => OpKind::UnpauseVault,
    }
}

fn run_random_sequence(seed: u64, op_count: u32) {
    let e = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    let mut rng = Rng::new(seed);
    let admin = Address::generate(&e);
    let owner = Address::generate(&e);
    let fondex = Address::generate(&e);
    let usdc = register_usdc(&e, &admin);
    let vault = register_vault_with_config(&e, &usdc, &owner, 60, i128::MAX);
    let mock_a = register_mock(&e, &usdc);
    let mock_b = register_mock(&e, &usdc);
    let mocks = [mock_a, mock_b];
    let (shortfall, exact) = setup_mocks_rounding(seed, &mocks, &mut rng);

    add_strategy(&e, &vault, &owner, &mocks[0].address, 5_000);
    add_strategy(&e, &vault, &owner, &mocks[1].address, 4_000);

    let mut users = std::vec::Vec::new();
    for _ in 0..NUM_USERS {
        users.push(Address::generate(&e));
    }

    let mut h = Harness {
        seed,
        step: 0,
        log: std::vec::Vec::new(),
        rng,
        total_minted: 0,
        withdraw_shortfall: shortfall,
        exact_rounding: exact,
        shareholders: std::vec::Vec::new(),
        withdraw_caps: [i128::MAX, i128::MAX],
    };

    for user in &users {
        mint(&e, &usdc, user, USER_USDC);
        h.record_mint(USER_USDC);
    }
    mint(&e, &usdc, &fondex, ONE_USDC);
    h.record_mint(ONE_USDC);
    e.mock_all_auths();
    let initial_shares = vault.deposit(&ONE_USDC, &fondex, &fondex);
    h.track_holder(&fondex);
    h.log_op(&std::format!(
        "init fondex deposit 1 USDC -> {initial_shares} shares"
    ));

    let token = usdc_client(&e, &usdc);
    let mut balance_fails = [false, false];
    let accounts = tracked_usdc_accounts(&users, &owner, &fondex, &vault, &mocks);

    for step in 0..op_count {
        h.step = step;
        check_usdc_conservation(&e, &usdc, &h, &accounts);
        check_allocation(&vault, &h);
        check_share_supply(&vault, &h);
        check_total_assets_consistent(&vault, &mocks, &balance_fails, &h);

        let ta_before = vault.total_assets();
        let supply_before = vault.total_supply();

        let op = pick_op(&mut h.rng);
        let mut share_policy = ShareValuePolicy::Strict;
        let mut check_share_value_after = false;

        match op {
            OpKind::Deposit => {
                let user = users[h.rng.next_u64() as usize % NUM_USERS].clone();
                let bal = token.balance(&user);
                if bal <= 0 {
                    h.log_op(&std::format!("{step} deposit skipped (no USDC)"));
                    continue;
                }
                let amount = if h.rng.next_u64().is_multiple_of(5) {
                    bal
                } else {
                    h.rng.gen_i128_up_to(bal)
                };
                e.mock_all_auths();
                h.log_op(&std::format!("{step} deposit amount={amount}"));
                if let Some(shares) = contract_err_acceptable(
                    vault.try_deposit(&amount, &user, &user),
                    &h,
                    "deposit",
                    |e| acceptable_fondex(e) || *e == pausable_enforced_pause(),
                ) {
                    check_share_value_after = h.exact_rounding;
                    if shares > 0 {
                        h.track_holder(&user);
                    }
                }
            }
            OpKind::Withdraw => {
                let owner_u = users[h.rng.next_u64() as usize % NUM_USERS].clone();
                let receiver = users[h.rng.next_u64() as usize % NUM_USERS].clone();
                let max_w = vault.max_withdraw(&owner_u);
                if max_w <= 0 {
                    h.log_op(&std::format!("{step} withdraw skipped (max=0)"));
                    continue;
                }
                let amount = if h.rng.next_u64().is_multiple_of(4) {
                    max_w
                } else {
                    h.rng.gen_i128_up_to(max_w)
                };
                let preview = vault.preview_withdraw(&amount);
                let recv_before = token.balance(&receiver);
                e.mock_all_auths();
                h.log_op(&std::format!("{step} withdraw assets={amount}"));
                if let Some(burned) = contract_err_acceptable(
                    vault.try_withdraw(&amount, &receiver, &owner_u),
                    &h,
                    "withdraw",
                    |err| {
                        if *err == pausable_enforced_pause() {
                            h.fail("withdraw failed with EnforcedPause");
                        }
                        if *err == fondex_error(FondexError::InsufficientLiquidity)
                            && !liquidity_failure_explained(
                                &vault,
                                &mocks,
                                &balance_fails,
                                &h.withdraw_caps,
                                amount,
                            )
                        {
                            h.fail("InsufficientLiquidity without visible cause");
                        }
                        *err == fungible_insufficient_balance()
                            || acceptable_fondex(err)
                            || *err == fondex_error(FondexError::InsufficientLiquidity)
                    },
                ) {
                    check_share_value_after = h.exact_rounding;
                    if burned != preview {
                        h.fail(&std::format!(
                            "withdraw preview mismatch: burned={burned} preview={preview}"
                        ));
                    }
                    let recv_after = token.balance(&receiver);
                    if recv_after - recv_before != amount {
                        h.fail(&std::format!(
                            "withdraw USDC delta {} expected {amount}",
                            recv_after - recv_before
                        ));
                    }
                }
            }
            OpKind::Redeem => {
                let owner_u = users[h.rng.next_u64() as usize % NUM_USERS].clone();
                let receiver = users[h.rng.next_u64() as usize % NUM_USERS].clone();
                let shares = vault.balance(&owner_u);
                if shares <= 0 {
                    h.log_op(&std::format!("{step} redeem skipped (no shares)"));
                    continue;
                }
                let redeem_shares = if h.rng.next_u64().is_multiple_of(4) {
                    shares
                } else {
                    h.rng.gen_i128_up_to(shares)
                };
                let preview = vault.preview_redeem(&redeem_shares);
                let recv_before = token.balance(&receiver);
                e.mock_all_auths();
                h.log_op(&std::format!("{step} redeem shares={redeem_shares}"));
                if let Some(assets) = contract_err_acceptable(
                    vault.try_redeem(&redeem_shares, &receiver, &owner_u),
                    &h,
                    "redeem",
                    |err| {
                        if *err == pausable_enforced_pause() {
                            h.fail("redeem failed with EnforcedPause");
                        }
                        if *err == fondex_error(FondexError::InsufficientLiquidity)
                            && !liquidity_failure_explained(
                                &vault,
                                &mocks,
                                &balance_fails,
                                &h.withdraw_caps,
                                preview,
                            )
                        {
                            h.fail("InsufficientLiquidity without visible cause");
                        }
                        *err == fungible_insufficient_balance()
                            || acceptable_fondex(err)
                            || *err == fondex_error(FondexError::InsufficientLiquidity)
                    },
                ) {
                    check_share_value_after = h.exact_rounding;
                    if assets != preview {
                        h.fail(&std::format!(
                            "redeem preview mismatch: out={assets} preview={preview}"
                        ));
                    }
                    let recv_after = token.balance(&receiver);
                    if recv_after - recv_before != assets {
                        h.fail(&std::format!(
                            "redeem USDC delta {} expected {assets}",
                            recv_after - recv_before
                        ));
                    }
                }
            }
            OpKind::Transfer => {
                let from = users[h.rng.next_u64() as usize % NUM_USERS].clone();
                let to = users[h.rng.next_u64() as usize % NUM_USERS].clone();
                let bal = vault.balance(&from);
                if bal <= 0 {
                    h.log_op(&std::format!("{step} transfer skipped (no shares)"));
                    continue;
                }
                let amount = if h.rng.next_u64().is_multiple_of(4) {
                    bal
                } else {
                    h.rng.gen_i128_up_to(bal)
                };
                e.mock_all_auths();
                h.log_op(&std::format!("{step} transfer amount={amount}"));
                if conversion_err_acceptable(
                    vault.try_transfer(&from, &to, &amount),
                    &h,
                    "transfer",
                    |e| *e == fungible_insufficient_balance(),
                )
                .is_some()
                {
                    check_share_value_after = h.exact_rounding;
                    h.track_holder(&to);
                }
            }
            OpKind::Rebalance => {
                h.log_op(&std::format!("{step} rebalance"));
                let ta_pre = vault.total_assets();
                e.set_auths(&[]);
                if conversion_err_acceptable(vault.try_rebalance(), &h, "rebalance", |e| {
                    *e == pausable_enforced_pause()
                })
                .is_some()
                {
                    check_share_value_after = true;
                    let ta_post = vault.total_assets();
                    if h.exact_rounding && ta_post < ta_pre {
                        h.fail(&std::format!(
                            "rebalance lost value in exact mode: {ta_pre} -> {ta_post}"
                        ));
                    }
                    let max_loss = h.withdraw_shortfall.saturating_mul(mocks.len() as i128);
                    if !h.exact_rounding && ta_post + max_loss < ta_pre {
                        h.fail(&std::format!(
                            "rebalance lost too much: {ta_pre} -> {ta_post} max_loss={max_loss}"
                        ));
                    }
                    share_policy = ShareValuePolicy::RebalanceRoundingSlack;
                }
            }
            OpKind::Yield => {
                let idx = h.rng.next_u64() as usize % mocks.len();
                if balance_fails[idx] {
                    h.log_op(&std::format!("{step} yield skipped (balance_fails)"));
                    continue;
                }
                let bps = (h.rng.next_u32() % 2_000) + 1;
                h.log_op(&std::format!("{step} yield mock={idx} bps={bps}"));
                let tok_before = token.balance(&mocks[idx].address);
                add_yield(&e, &usdc, &mocks[idx], &vault.address, bps);
                let tok_after = token.balance(&mocks[idx].address);
                h.record_mint(tok_after - tok_before);
                check_share_value_after = h.exact_rounding;
            }
            OpKind::Loss => {
                let idx = h.rng.next_u64() as usize % mocks.len();
                let bps = (h.rng.next_u32() % 2_000) + 1;
                h.log_op(&std::format!("{step} loss mock={idx} bps={bps}"));
                mocks[idx].simulate_loss(&vault.address, &bps);
                share_policy = ShareValuePolicy::Exempt;
            }
            OpKind::Donate => {
                let user = users[h.rng.next_u64() as usize % NUM_USERS].clone();
                let bal = token.balance(&user);
                if bal <= 0 {
                    h.log_op(&std::format!("{step} donate skipped"));
                    continue;
                }
                let amount = h.rng.gen_i128_up_to(bal);
                e.mock_all_auths();
                h.log_op(&std::format!("{step} donate amount={amount}"));
                token.transfer(&user, &vault.address, &amount);
            }
            OpKind::WithdrawCap => {
                let idx = h.rng.next_u64() as usize % mocks.len();
                let cap = if h.rng.next_u64().is_multiple_of(2) {
                    i128::MAX
                } else {
                    h.rng.gen_i128_up_to(100 * ONE_USDC)
                };
                h.withdraw_caps[idx] = cap;
                h.log_op(&std::format!("{step} withdraw_cap mock={idx} cap={cap}"));
                mocks[idx].set_withdraw_cap(&cap);
            }
            OpKind::BalanceFails => {
                let idx = h.rng.next_u64() as usize % mocks.len();
                let enable = h.rng.next_u64().is_multiple_of(2);
                balance_fails[idx] = enable;
                h.log_op(&std::format!(
                    "{step} balance_fails mock={idx} enable={enable}"
                ));
                mocks[idx].set_balance_fails(&enable);
                share_policy = ShareValuePolicy::Exempt;
            }
            OpKind::PauseStrategy => {
                let idx = h.rng.next_u64() as usize % mocks.len();
                h.log_op(&std::format!("{step} pause_strategy mock={idx}"));
                mock_owner_auth(
                    &e,
                    &vault,
                    &owner,
                    "pause_strategy",
                    (mocks[idx].address.clone(),),
                );
                let _ = vault.try_pause_strategy(&mocks[idx].address);
            }
            OpKind::UnpauseStrategy => {
                let idx = h.rng.next_u64() as usize % mocks.len();
                h.log_op(&std::format!("{step} unpause_strategy mock={idx}"));
                mock_owner_auth(
                    &e,
                    &vault,
                    &owner,
                    "unpause_strategy",
                    (mocks[idx].address.clone(),),
                );
                let _ = vault.try_unpause_strategy(&mocks[idx].address);
            }
            OpKind::SetTarget => {
                let idx = h.rng.next_u64() as usize % mocks.len();
                let strategies = vault.strategies();
                let mut current_bps = 0_u32;
                for info in strategies.iter() {
                    if info.address == mocks[idx].address {
                        current_bps = info.target_bps;
                        break;
                    }
                }
                let others_sum = sum_target_bps(&vault).saturating_sub(current_bps);
                let room = max_allocation_bps().saturating_sub(others_sum);
                if room == 0 {
                    h.log_op(&std::format!("{step} set_target skipped (no room)"));
                    continue;
                }
                let new_target = (h.rng.next_u32() % room) + 1;
                h.log_op(&std::format!(
                    "{step} set_target mock={idx} bps={new_target}"
                ));
                mock_owner_auth(
                    &e,
                    &vault,
                    &owner,
                    "set_target",
                    (mocks[idx].address.clone(), new_target),
                );
                let _ = conversion_err_acceptable(
                    vault.try_set_target(&mocks[idx].address, &new_target),
                    &h,
                    "set_target",
                    |e| *e == fondex_error(FondexError::AllocationTooHigh),
                );
            }
            OpKind::PauseVault => {
                h.log_op(&std::format!("{step} pause_vault"));
                mock_owner_auth(&e, &vault, &owner, "pause", ());
                let _ = vault.try_pause();
            }
            OpKind::UnpauseVault => {
                h.log_op(&std::format!("{step} unpause_vault"));
                mock_owner_auth(&e, &vault, &owner, "unpause", ());
                let _ = vault.try_unpause();
            }
        }

        let ta_after = vault.total_assets();
        let supply_after = vault.total_supply();
        if check_share_value_after {
            check_share_value(
                &e,
                ta_before,
                supply_before,
                ta_after,
                supply_after,
                share_policy,
                &h,
            );
        }

        for user in &users {
            if vault.balance(user) > 0 {
                h.track_holder(user);
            }
        }
        if vault.balance(&fondex) > 0 {
            h.track_holder(&fondex);
        }
    }

    check_usdc_conservation(&e, &usdc, &h, &accounts);
    check_allocation(&vault, &h);
    check_share_supply(&vault, &h);
    check_total_assets_consistent(&vault, &mocks, &balance_fails, &h);
}

#[test]
fn invariants_hold_for_random_sequences() {
    for seed in 0..SEED_COUNT_CI {
        run_random_sequence(seed, OP_COUNT_CI);
    }
}

#[test]
#[ignore]
fn invariants_hold_for_many_random_sequences() {
    for seed in 0..SEED_COUNT_LONG {
        run_random_sequence(seed, OP_COUNT_LONG);
    }
}
