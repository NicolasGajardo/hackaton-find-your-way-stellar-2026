use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
};

const BPS: i128 = 10_000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MockStrategyError {
    WithdrawCapExceeded = 1,
    InsufficientBalance = 2,
    BalanceQueryFailed = 3,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Asset,
    Balance(Address),
    WithdrawCap,
    DepositRefund,
    WithdrawShortfall,
    DepositOverpull,
    BalanceFails,
}

fn read_balance(e: &Env, account: &Address) -> i128 {
    e.storage()
        .instance()
        .get(&DataKey::Balance(account.clone()))
        .unwrap_or(0)
}

fn write_balance(e: &Env, account: &Address, amount: i128) {
    e.storage()
        .instance()
        .set(&DataKey::Balance(account.clone()), &amount);
}

fn token_client(e: &Env) -> token::Client<'_> {
    let asset: Address = e.storage().instance().get(&DataKey::Asset).unwrap();
    token::Client::new(e, &asset)
}

#[contract]
pub struct MockStrategy;

#[contractimpl]
impl MockStrategy {
    pub fn __constructor(e: &Env, asset: Address) {
        e.storage().instance().set(&DataKey::Asset, &asset);
        e.storage()
            .instance()
            .set(&DataKey::WithdrawCap, &i128::MAX);
        e.storage().instance().set(&DataKey::DepositRefund, &0_i128);
        e.storage()
            .instance()
            .set(&DataKey::WithdrawShortfall, &0_i128);
        e.storage()
            .instance()
            .set(&DataKey::DepositOverpull, &0_i128);
        e.storage().instance().set(&DataKey::BalanceFails, &false);
    }

    pub fn asset(e: &Env) -> Address {
        e.storage().instance().get(&DataKey::Asset).unwrap()
    }

    pub fn balance(e: &Env, from: Address) -> i128 {
        if e.storage()
            .instance()
            .get(&DataKey::BalanceFails)
            .unwrap_or(false)
        {
            panic_with_error!(e, MockStrategyError::BalanceQueryFailed);
        }
        read_balance(e, &from)
    }

    pub fn deposit(e: &Env, amount: i128, from: Address) -> i128 {
        from.require_auth();
        let refund = e
            .storage()
            .instance()
            .get(&DataKey::DepositRefund)
            .unwrap_or(0);
        let overpull = e
            .storage()
            .instance()
            .get(&DataKey::DepositOverpull)
            .unwrap_or(0);
        let token = token_client(e);
        token.transfer(&from, e.current_contract_address(), &(amount + overpull));
        if refund > 0 {
            token.transfer(&e.current_contract_address(), &from, &refund);
        }
        let credited = amount - refund;
        let new_balance = read_balance(e, &from) + credited;
        write_balance(e, &from, new_balance);
        new_balance
    }

    pub fn withdraw(e: &Env, amount: i128, from: Address, to: Address) -> i128 {
        from.require_auth();
        let cap = e
            .storage()
            .instance()
            .get(&DataKey::WithdrawCap)
            .unwrap_or(i128::MAX);
        if amount > cap {
            panic_with_error!(e, MockStrategyError::WithdrawCapExceeded);
        }
        let balance = read_balance(e, &from);
        if amount > balance {
            panic_with_error!(e, MockStrategyError::InsufficientBalance);
        }
        let shortfall = e
            .storage()
            .instance()
            .get(&DataKey::WithdrawShortfall)
            .unwrap_or(0);
        let payout = amount - shortfall;
        write_balance(e, &from, balance - amount);
        if payout > 0 {
            token_client(e).transfer(&e.current_contract_address(), &to, &payout);
        }
        read_balance(e, &from)
    }

    pub fn harvest(_e: &Env, _from: Address, _data: Option<soroban_sdk::Bytes>) {}

    pub fn set_withdraw_cap(e: &Env, max: i128) {
        e.storage().instance().set(&DataKey::WithdrawCap, &max);
    }

    pub fn set_deposit_refund(e: &Env, n: i128) {
        e.storage().instance().set(&DataKey::DepositRefund, &n);
    }

    pub fn set_withdraw_shortfall(e: &Env, n: i128) {
        e.storage().instance().set(&DataKey::WithdrawShortfall, &n);
    }

    pub fn set_deposit_overpull(e: &Env, n: i128) {
        e.storage().instance().set(&DataKey::DepositOverpull, &n);
    }

    pub fn set_balance_fails(e: &Env, fails: bool) {
        e.storage().instance().set(&DataKey::BalanceFails, &fails);
    }

    pub fn simulate_yield(e: &Env, from: Address, bps: u32) {
        let balance = read_balance(e, &from);
        let added = balance * i128::from(bps) / BPS;
        write_balance(e, &from, balance + added);
    }

    pub fn simulate_loss(e: &Env, from: Address, bps: u32) {
        let balance = read_balance(e, &from);
        let removed = balance * i128::from(bps) / BPS;
        write_balance(e, &from, balance - removed);
    }
}
