# Fondex — Technical design: a mutual fund that invests through DeFindex strategies

> Contract: [`contracts/contracts/fondex_vault`](../contracts/contracts/fondex_vault) · Deployed on testnet: `CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS`

---

## 1. What we built

The **FondexVault** turns Fondex into a **decentralized mutual fund**: besides receiving USDC and issuing shares (`fxFUND`), it **invests the USDC in liquid, low-risk strategies** so the share price grows, and keeps a reserve to pay redemptions immediately.

- The vault **does not talk to Blend directly**. It deposits into **already deployed DeFindex strategies**, and those strategies deposit into Blend.
- **Fondex does not write its own strategies:** it reuses audited, already deployed strategies.

```
User ──deposit/withdraw/redeem──► FondexVault ──deposit/withdraw──► Blend USDC strategy ──► Blend v2 pool
                                  (fxFUND shares,                   (DeFindex contract,       (lending)
                                   10% USDC in reserve)              already deployed)
```

## 2. Decisions (closed)

| Topic | Decision |
|---|---|
| Strategies | **Only DeFindex strategies** that are already deployed, called directly on the blockchain. No API, no API key, no DeFindex servers. |
| Fondex's own strategies | **No.** Out of the current scope. |
| Initial strategy | DeFindex Blend USDC (testnet `CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY`) |
| USDC (testnet) | Blend test USDC `CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU` |
| Initial allocation | 90% in the strategy · 10% in reserve (not invested) to pay withdrawals |
| When it invests | Through `rebalance()`, which anyone can call. Depositing does **not** invest. |
| Withdrawals | Paid from the reserve; if it is not enough, the same withdrawal pulls the rest from the strategies |
| BLND rewards | Reinvested by the DeFindex strategy itself. Fondex does nothing. |
| Admin | A single owner (`owner`, OpenZeppelin Ownable module). On mainnet it must be a multisig account. |
| Adding strategies | With a timelock between proposing and accepting |
| Pause | Blocks `deposit` and `rebalance`. **Never** blocks `withdraw` or `redeem`. |
| Upgradeable | **No**, neither on testnet nor on mainnet: we test exactly the contract that will go to production. To fix something on testnet we **redeploy** with one command (§12.4). |
| Deposit cap | `deposit_cap`: maximum fund value, configurable by the admin. Testnet: no cap. Mainnet: start low and raise it over time. |

### 2.1 Investment policy (the mutual fund rule)

Fondex is a mutual fund: **it only invests in liquid, low-risk strategies**. A strategy can only be proposed (`propose_strategy`) if it meets **all** of these criteria:

| # | Criterion | What it means | Enforced by the contract? |
|---|---|---|---|
| 1 | **Same asset** | Receives and returns USDC. No exposure to the price of other assets (XLM, BTC, etc.). | Yes: `AssetMismatch` |
| 2 | **Liquid** | Under normal conditions it can be withdrawn in the same transaction, with no queues, lock-up periods or notice periods. | No: admin review |
| 3 | **No leverage or derivatives** | No futures, options, borrowing or recursive lending loops. | No: admin review |
| 4 | **No impermanent loss** | No two-asset liquidity pools. | No: admin review |
| 5 | **Valued without an external oracle** | The USDC value is computed by the protocol itself (e.g. Blend's `b_rate`), not from a market price of another asset. | No: admin review |
| 6 | **Battle-tested** | Audited contracts, deployed on mainnet with real usage, and preferably not upgradeable. | No: admin review |

In addition, the contract always guarantees:
- a **minimum reserve** of `MIN_IDLE_BPS` (5%) not invested, to pay redemptions;
- a public **timelock** before a new strategy receives funds, so anyone can redeem if they disagree;
- **at most `MAX_STRATEGIES`** (4) strategies.

Criteria 2 to 6 are public commitments by Fondex: they are documented for each strategy before proposing it (in the README and in the app). The DeFindex Blend USDC strategy meets all six.

Any strategy that does not meet this policy (futures, leverage, volatile assets) **does not go into this fund**. If it is ever offered, it will be as a separate fund, with its own contract and risk profile.

### Out of scope (do not implement)

Own strategies · BLND handling · manager role · fees · oracles · deposits in other currencies · upgrades · calling the DeFindex API.

Oracles and other assets are out **only for now**, to reach a working prototype. They are future features (see "Where it grows" in [00-summary.md](00-summary.md)). The v2 contract structure (one asset + strategies) works for funds of other assets, but **not as-is**: `MIN_STRATEGY_MOVE`, `WITHDRAW_ROUNDING_BUFFER` and `STRATEGY_DUST` are constants tuned for USDC (e.g. the 1-unit minimum would be 1 wBTC). A fund for another asset needs a version with those values configurable in the constructor.

## 3. Glossary

| Term | Meaning |
|---|---|
| **Share (`fxFUND`)** | Token that represents your part of the fund. Lives inside the vault contract (SEP-41). |
| **Assets (`assets`)** | USDC, in its smallest unit (7 decimals). |
| **`total_assets`** | Total fund value in USDC = reserve + the value of each strategy. |
| **Reserve (`idle`)** | USDC held inside the vault, not invested. |
| **Strategy** | External contract (from DeFindex) that receives USDC from the vault and invests it. |
| **bps** | Basis points. 10,000 bps = 100%; 9,000 bps = 90%. |
| **`rebalance`** | Moving USDC between the reserve and the strategies to get closer to the target allocation. |

## 4. Repository structure

```
contracts/
  Cargo.toml                ← workspace
  contracts/fondex_vault/
    Cargo.toml
    src/
      lib.rs                ← contract: public functions
      storage.rs            ← storage keys and reads/writes
      math.rs               ← shares ↔ assets conversion
      strategy.rs           ← strategy interface (client) + authorization
      errors.rs             ← FondexError
      events.rs             ← events
      test/
        mod.rs
        mock_strategy.rs    ← FAKE strategy for tests only (never deployed)
        deposit_withdraw.rs
        strategies.rs
        rebalance.rs
        admin.rs
        unavailable.rs
        invariants.rs
  scripts/
    deploy-testnet.sh       ← one-command testnet deployment
    e2e-testnet.sh          ← end-to-end test on testnet
```

### Dependencies (exact versions)

```toml
soroban-sdk = "26.1.0"
stellar-tokens = "=0.7.2"          # share token (fungible)
stellar-access = "=0.7.2"          # Ownable
stellar-contract-utils = "=0.7.2"  # pausable + math (mul_div_with_rounding)
```

`fondex_vault` does **not** use `stellar_tokens::vault::Vault`: that vault computes `total_assets` only from the USDC it holds (`vault/storage.rs:142`) and it cannot be changed. From the `stellar-tokens` crate we only use the share token (`fungible::Base`, `FungibleToken`).

## 5. Constants

```rust
pub const SHARE_NAME: &str = "Fondex Fund Share";
pub const SHARE_SYMBOL: &str = "fxFUND";
pub const DECIMALS_OFFSET: u32 = 10;            // shares = 7 + 10 = 17 decimals
pub const BPS: u32 = 10_000;
pub const MIN_IDLE_BPS: u32 = 500;              // the reserve can never drop below 5%
pub const MAX_STRATEGIES: u32 = 4;
pub const MAX_TIMELOCK_SECS: u64 = 604_800;     // 7 days
pub const REBALANCE_TOLERANCE_BPS: u32 = 200;   // don't move if the deviation is < 2% of the fund
pub const MIN_STRATEGY_MOVE: i128 = 10_000_000; // 1 USDC: smaller amounts are not moved
pub const WITHDRAW_ROUNDING_BUFFER: i128 = 100; // extra smallest units when pulling from a strategy
pub const STRATEGY_DUST: i128 = 1_000;          // negligible balance when removing a strategy
```

## 6. Storage

Everything in `instance` storage, with `extend_ttl` in every function.

```rust
#[contracttype]
pub enum DataKey {
    Asset,               // Address — USDC
    Strategies,          // Vec<StrategyInfo>, in registration order
    PendingStrategy,     // PendingStrategy (optional)
    StrategyTimelock,    // u64 — seconds
    DepositCap,          // i128 — maximum total_assets accepted on deposit (i128::MAX = no cap)
}

#[contracttype]
pub struct StrategyInfo {
    pub address: Address,
    pub target_bps: u32,  // target % of the fund in this strategy
    pub paused: bool,     // paused = target 0: receives no deposits and rebalance empties it
}

#[contracttype]
pub struct PendingStrategy {
    pub address: Address,
    pub target_bps: u32,
    pub eta: u64,         // timestamp from which it can be accepted
}
```

- The **target reserve is not stored**: it is `BPS − Σ target_bps` (of the non-paused strategies).
- Rule that always holds: `Σ target_bps ≤ BPS − MIN_IDLE_BPS`, summing **all** strategies, including paused ones (so unpausing never breaks the rule).
- The owner is stored by `stellar_access::ownable`. The pause state is stored by `stellar_contract_utils::pausable`.

## 7. Errors

```rust
#[contracterror]
#[repr(u32)]
pub enum FondexError {
    AmountMustBePositive = 1,
    ZeroShares = 2,               // the deposit is so small it would give 0 shares
    ZeroAssets = 3,               // the redemption is so small it would give 0 USDC
    InsufficientLiquidity = 4,    // not enough USDC between reserve and strategies to pay the withdrawal
    DepositCapExceeded = 5,       // the deposit would push total_assets above deposit_cap
    StrategyUnavailable = 6,      // a strategy does not report its balance: deposits are not accepted (§10.9)
    TooManyStrategies = 10,
    StrategyAlreadyExists = 11,
    StrategyNotFound = 12,
    AssetMismatch = 13,           // strategy.asset() ≠ the vault's USDC
    AllocationTooHigh = 14,       // Σ target_bps > BPS − MIN_IDLE_BPS
    PendingStrategyExists = 15,
    NoPendingStrategy = 16,
    TimelockNotExpired = 17,
    StrategyNotEmpty = 18,        // on removal: more than STRATEGY_DUST remains
    InvalidTimelock = 19,         // constructor: timelock > MAX_TIMELOCK_SECS
    InvalidDepositCap = 20,       // deposit_cap <= 0
}
```

OpenZeppelin errors can also show up: `FungibleTokenError::InsufficientBalance` (100) when burning more shares than someone holds, `PausableError::EnforcedPause` (1000) and the Ownable ones (2100+).

## 8. Share math (`math.rs`)

Use `stellar_contract_utils::math::i128_fixed_point::mul_div_with_rounding(e, x, y, denominator, Rounding::Floor | Rounding::Ceil)`. It is the same formula as OpenZeppelin's, but with our own `total_assets`.

```
V = 10^DECIMALS_OFFSET
convert_to_shares(assets, rounding) = mul_div(assets, total_supply + V, total_assets + 1, rounding)
convert_to_assets(shares, rounding) = mul_div(shares, total_assets + 1, total_supply + V, rounding)

preview_deposit(assets)  = convert_to_shares(assets, Floor)   // shares you receive
preview_withdraw(assets) = convert_to_shares(assets, Ceil)    // shares that get burned
preview_redeem(shares)   = convert_to_assets(shares, Floor)   // USDC you receive
```

Rule: **rounding always favors the vault** (those who stay).

## 9. Strategy interface (`strategy.rs`)

Same as `DeFindexStrategyTrait` (repo `paltalabs/defindex`, `apps/contracts/strategies/core`). Declared with `#[contractclient(name = "StrategyClient")]`:

```rust
pub trait Strategy {
    fn asset(e: Env) -> Address;
    fn deposit(e: Env, amount: i128, from: Address) -> i128;              // returns the balance of `from` afterwards
    fn withdraw(e: Env, amount: i128, from: Address, to: Address) -> i128;
    fn balance(e: Env, from: Address) -> i128;                            // USDC value of what `from` holds
    fn harvest(e: Env, from: Address, data: Option<Bytes>);               // the vault does NOT call it
}
```

### Actual behavior of the DeFindex Blend strategy (verified in its code)

1. **`deposit`** transfers the full `amount` from the vault and may **return a small part** due to rounding. → The vault computes the invested amount by **measuring its USDC balance before and after**.
2. **`withdraw`** sends `to` an amount that may be **a few units lower** than requested (rounding down). → The vault asks for `WITHDRAW_ROUNDING_BUFFER` extra and **measures what it received**.
3. Very small amounts fail (`InvalidSharesMinted` 416, `BTokensAmountBelowMin` 453). → The vault does not move amounts below `MIN_STRATEGY_MOVE`.
4. If Blend has no liquidity, `withdraw` **fails**. → The vault uses `try_withdraw` and moves on to the next strategy.
5. `deposit`/`withdraw` require `from.require_auth()`. Since the vault is the caller, that signature is automatic.

### Authorization when depositing into a strategy (required)

Inside `strategy.deposit`, the strategy executes `usdc.transfer(vault → strategy, amount)`. That transfer is **not** invoked directly by the vault, so the vault must authorize it right before, **only for that amount and to that strategy**:

```rust
e.authorize_as_current_contract(vec![
    &e,
    InvokerContractAuthEntry::Contract(SubContractInvocation {
        context: ContractContext {
            contract: usdc.clone(),
            fn_name: Symbol::new(&e, "transfer"),
            args: (e.current_contract_address(), strategy.clone(), amount).into_val(&e),
        },
        sub_invocations: vec![&e],
    }),
]);
StrategyClient::new(&e, &strategy).deposit(&amount, &e.current_contract_address());
```

`withdraw` does not need this: the USDC comes from Blend to the vault.

## 10. Contract functions

Notation: **Auth** = who must sign · **Pause** = whether `pause()` blocks it.

### 10.1 Constructor

```rust
fn __constructor(e: Env, asset: Address, owner: Address, strategy_timelock: u64, deposit_cap: i128)
```
- `strategy_timelock ≤ MAX_TIMELOCK_SECS`; otherwise → `InvalidTimelock`.
- `deposit_cap > 0`; otherwise → `InvalidDepositCap`. Testnet: `i128::MAX` (no cap).
- Stores `asset`, `timelock` and `deposit_cap`, sets the owner (`ownable::set_owner`) and the token metadata (decimals = USDC decimals + `DECIMALS_OFFSET`, name, symbol).
- Testnet: `strategy_timelock = 60`. **Mainnet: at least 172,800 (48 h).** It is immutable and visible on the blockchain.

### 10.2 Queries (no auth)

| Function | Returns |
|---|---|
| `query_asset() -> Address` | USDC |
| `total_assets() -> i128` | `idle_assets() + Σ StrategyClient.balance(vault)` (all of them, including paused). **Conservative** valuation (§10.9): a strategy that doesn't respond counts as 0 |
| `idle_assets() -> i128` | The vault's USDC balance |
| `strategies() -> Vec<StrategyInfo>` | List of strategies |
| `strategy_balances() -> Vec<(Address, i128)>` | Current value in each strategy (for the frontend) |
| `pending_strategy() -> Option<PendingStrategy>` | Proposed strategy |
| `strategy_timelock() -> u64` | Timelock |
| `convert_to_shares(assets)`, `convert_to_assets(shares)` | Rounding down |
| `preview_deposit`, `preview_withdraw`, `preview_redeem` | As in §8 |
| `max_withdraw(owner) -> i128` | `preview_redeem(balance(owner))`. Does not account for Blend liquidity: if it is short, the withdrawal fails with `InsufficientLiquidity`. |
| `max_redeem(owner) -> i128` | `balance(owner)` |
| `deposit_cap() -> i128` | Current cap |
| `max_deposit(receiver) -> i128` | `0` if paused or if any strategy doesn't respond (§10.9); otherwise `max(deposit_cap − total_assets, 0)` (the same value for everyone) |
| `paused() -> bool`, `get_owner() -> Option<Address>` | From OpenZeppelin |

In addition, the share token exposes the full SEP-41 interface via `#[contractimpl(contracttrait)] impl FungibleToken` with `type ContractType = Base` (`balance`, `transfer`, `approve`, `decimals`, etc.).

### 10.3 `deposit(assets, receiver, from) -> i128`
- **Auth:** `from` · **Pause:** yes
1. `assets > 0` (otherwise `AmountMustBePositive`).
2. `total` = **strict** valuation (§10.9): if any strategy doesn't respond → `StrategyUnavailable`. If `total + assets > deposit_cap` → `DepositCapExceeded`. `preview_deposit` uses the same strict valuation.
3. `shares = preview_deposit(assets)`, **computed before receiving the USDC**. If `shares == 0` → `ZeroShares`.
4. `usdc.transfer(from → vault, assets)`.
5. `Base::mint(receiver, shares)`.
6. `deposit` event. Returns `shares`.
- Does **not** invest in strategies.

### 10.4 `withdraw(assets, receiver, owner) -> i128`
- **Auth:** `owner` · **Pause:** no
1. `assets > 0`.
2. `shares = preview_withdraw(assets)`, computed **before** moving anything.
3. `ensure_liquidity(assets)` (§10.6).
4. Burn `shares` from `owner` (`Base::update(e, Some(&owner), None, shares)`). If they don't have enough → OpenZeppelin error 100.
5. `usdc.transfer(vault → receiver, assets)`.
6. `withdraw` event. Returns `shares`.

### 10.5 `redeem(shares, receiver, owner) -> i128`
- **Auth:** `owner` · **Pause:** no
1. `shares > 0`.
2. `assets = preview_redeem(shares)`, computed **before** moving anything. If `assets == 0` → `ZeroAssets`.
3. `ensure_liquidity(assets)`.
4. Burn `shares` from `owner`.
5. `usdc.transfer(vault → receiver, assets)`.
6. `withdraw` event. Returns `assets`.

### 10.6 `ensure_liquidity(needed)` (internal)
1. `idle = usdc.balance(vault)`. If `idle ≥ needed`, done.
2. For each strategy **in registration order** (including paused ones):
   - `bal = balance(vault)` via `try_balance`; if it fails or `bal == 0`, move on to the next one.
   - `request = min(needed − idle + WITHDRAW_ROUNDING_BUFFER, bal)`.
   - `try_withdraw(request, vault, vault)`. If it fails, move on to the next one.
   - `idle = usdc.balance(vault)` (**measure**, don't trust the returned value). If `idle ≥ needed`, done.
3. If at the end `idle < needed` → `InsufficientLiquidity`.

### 10.7 `rebalance()`
- **Auth:** none (public) · **Pause:** yes
1. `total = total_assets()` (conservative), `tol = total × REBALANCE_TOLERANCE_BPS / BPS`. Strategies that don't report their balance are **skipped**: nothing is deposited into or withdrawn from them (§10.9).
2. **Pull from the overweight ones:** for each strategy, `target = 0` if paused; otherwise `total × target_bps / BPS`. If `bal > target + tol` and `bal − target ≥ MIN_STRATEGY_MOVE`: `withdraw(bal − target)`.
3. **Invest in the underweight ones:** `idle = usdc.balance(vault)`, `idle_target = total × (BPS − Σ active target_bps) / BPS`, `available = idle − idle_target`. For each **non-paused** strategy with `bal < target − tol`: `amount = min(target − bal, available)`. If `amount ≥ MIN_STRATEGY_MOVE`: authorize (§9), `deposit(amount)`, and `available −= (idle_before − idle_after)` (measured).
4. A `strategy_deposit` / `strategy_withdraw` event for each movement and a `rebalance` event at the end with `total_assets` before and after.
- If nothing exceeds the tolerance, it moves no money and does not fail; it still emits `rebalance` (with `before == after`) so it can be monitored.
- If a strategy fails on deposit or withdrawal (`try_deposit` / `try_withdraw`), it is skipped and the rest continue: one broken strategy does not block the rebalance.

### 10.8 Administration (Auth: `owner`)

| Function | What it does | Validations |
|---|---|---|
| `propose_strategy(strategy, target_bps)` | Stores `PendingStrategy { eta: now + timelock }` | no other pending one (`PendingStrategyExists`); not registered (`StrategyAlreadyExists`); fewer than `MAX_STRATEGIES` (`TooManyStrategies`); `StrategyClient.asset() == asset` (`AssetMismatch`); new sum ≤ `BPS − MIN_IDLE_BPS` (`AllocationTooHigh`) |
| `cancel_strategy()` | Deletes the pending one | it exists (`NoPendingStrategy`) |
| `accept_strategy()` | Adds the pending one to `Strategies` | it exists; `now ≥ eta` (`TimelockNotExpired`); revalidates slot count and sum |
| `set_target(strategy, target_bps)` | Changes the target % | it exists (`StrategyNotFound`); new sum ≤ `BPS − MIN_IDLE_BPS` |
| `pause_strategy(strategy)` / `unpause_strategy(strategy)` | Sets `paused` | it exists |
| `remove_strategy(strategy)` | Pulls everything out of the strategy and removes it from the list | it exists; after pulling, `balance ≤ STRATEGY_DUST` (`StrategyNotEmpty`). The remaining dust is abandoned (recorded in the event). |
| `set_deposit_cap(cap)` | Changes the deposit cap | `cap > 0` (`InvalidDepositCap`). It can be set below the current `total_assets`: that only prevents new deposits and **never** affects withdrawals. No timelock. |
| `pause()` / `unpause()` | Global pause (OpenZeppelin `pausable`) | — |
| `transfer_ownership(new_owner, live_until_ledger)` / `accept_ownership()` | Two-step ownership transfer (OpenZeppelin) | — |

Do not expose `renounce_ownership`: without an owner a problematic strategy could not be removed. That is why the owner functions are exposed by hand using the free functions in `stellar_access::ownable`, without `impl Ownable` (which would export it).

### 10.9 A strategy that doesn't report its balance

If `StrategyClient.balance(vault)` fails (e.g. a problem in Blend), the vault cannot know how much the fund is worth. Rule: **nobody gets in at a wrong price, but everyone can get out.**

| Operation | Valuation | With a strategy that doesn't respond |
|---|---|---|
| `deposit`, `preview_deposit` | **Strict** | Fails with `StrategyUnavailable` (6) |
| `max_deposit` | Strict | Returns `0` |
| `withdraw`, `redeem` and their `preview_*`, `max_withdraw` | **Conservative** | The strategy counts as 0: whoever redeems receives their share of what **is** available |
| `total_assets`, `convert_to_*`, `strategy_balances` | Conservative | Counts as 0 (shows up as 0 in `strategy_balances`) |
| `ensure_liquidity` | — | Skips the strategy |
| `rebalance` | Conservative | Neither deposits into nor withdraws from it |

- Implementation: `strategy_balance` uses `try_balance` and returns `Option<i128>`. Two internal functions: `total_assets_strict` (fails with `StrategyUnavailable`) and `total_assets_conservative` (counts 0).
- Accepted consequence: whoever redeems while the strategy is failing **gives up their part of what is stuck**, and that part stays with those who remain. Whoever prefers to wait loses nothing. This way "redemptions always available" holds without anyone being able to exploit the price.
- `remove_strategy` on a strategy that doesn't respond still fails (it can't be verified as empty). See §15.

## 11. Events (`events.rs`)

| Event | Topics | Data |
|---|---|---|
| `deposit` | `from`, `receiver` | `assets`, `shares` |
| `withdraw` | `owner`, `receiver` | `assets`, `shares` |
| `strategy_deposit` | `strategy` | `amount` (measured) |
| `strategy_withdraw` | `strategy` | `amount` (measured) |
| `rebalance` | — | `total_assets_before`, `total_assets_after` |
| `strategy_proposed` | `strategy` | `target_bps`, `eta` |
| `strategy_cancelled` | `strategy` | — |
| `strategy_accepted` | `strategy` | `target_bps` |
| `strategy_removed` | `strategy` | `dust_left` |
| `target_changed` | `strategy` | `old_bps`, `new_bps` |
| `strategy_paused` / `strategy_unpaused` | `strategy` | — |
| `deposit_cap_changed` | — | `old_cap`, `new_cap` |

`Base::mint` emits `mint`; burning via `Base::update` emits no event of its own (it is recorded in `withdraw`). The `transfer`, pause and ownership events are emitted by OpenZeppelin.

## 12. Tests

### 12.1 Fake strategy for tests (`test/mock_strategy.rs`)

A contract **for tests only** (never deployed; it is not a "Fondex strategy"). It implements the §9 interface and can simulate:
- **interest**: `simulate_yield(bps)` raises the value of all balances (it mints test USDC to itself);
- **loss**: `simulate_loss(bps)`;
- **lack of liquidity**: `set_withdraw_cap(max)`; if more is requested, `withdraw` fails;
- **rounding**: `set_deposit_refund(n)` returns `n` units on deposit; `set_withdraw_shortfall(n)` delivers `n` fewer units on withdrawal.

### 12.2 Minimum test list (each one with its own name)

**Shares and rounding**
- first deposit: 1 USDC → 10^17 shares;
- depositing and withdrawing immediately never returns more than was deposited;
- `ZeroShares` and `ZeroAssets` with minimal amounts;
- after `simulate_yield`, the share is worth more and a new depositor receives fewer shares.

**Strategies**
- `rebalance` with 1000 USDC and a 90% target → ~900 in the strategy and ~100 in reserve;
- `rebalance` with a deviation below the tolerance moves nothing;
- `rebalance` with `deposit_refund` and `withdraw_shortfall` leaves `total_assets` correct (difference ≤ a few units in the vault's favor);
- paused strategy: `rebalance` empties it and doesn't deposit into it;
- a `withdraw` larger than the reserve pulls the shortfall from the strategy;
- `withdraw` with `withdraw_cap` and two strategies: uses the second if the first fails;
- `withdraw` without enough liquidity → `InsufficientLiquidity`;
- the `transfer` authorization covers exactly `(vault, strategy, amount)`: a strategy that tries to transfer more fails.

**Administration**
- every admin function fails without the owner's signature (exact auth tree);
- `accept_strategy` before the timelock → `TimelockNotExpired`; afterwards it works;
- `AssetMismatch`, `AllocationTooHigh`, `TooManyStrategies`, `StrategyAlreadyExists`;
- `remove_strategy` pulls everything and removes the strategy; with a balance above dust → `StrategyNotEmpty`.

**Deposit cap**
- depositing up to exactly the cap works; 1 more unit → `DepositCapExceeded`;
- with accrued interest, the cap counts the current fund value (`total_assets`), not what was deposited;
- lowering the cap below `total_assets` blocks deposits but `withdraw` and `redeem` keep working;
- `max_deposit` matches what can actually be deposited;
- `set_deposit_cap` without the owner's signature fails.

**A strategy that doesn't respond (§10.9)** (mock knob: `set_balance_fails(bool)`)
- `deposit` and `preview_deposit` fail with `StrategyUnavailable`; `max_deposit` returns 0;
- `withdraw` and `redeem` work with the reserve and other strategies, valuing the broken one at 0;
- `rebalance` doesn't fail and doesn't touch the broken strategy;
- when the strategy responds again, those who stayed recover the full value.

**Pause**
- paused: `deposit` and `rebalance` fail with 1000; `withdraw` and `redeem` **work**.

**Invariants** (repeated with random sequences of deposits, withdrawals, rebalances, interest and losses)
1. USDC only leaves the vault toward the `receiver` of a withdrawal (burning the `owner`'s shares) or toward a registered strategy.
2. No role can prevent a withdrawal if there is liquidity.
3. `rebalance` doesn't lower `total_assets` by more than rounding (≤ 2 × `WITHDRAW_ROUNDING_BUFFER`).
4. `Σ target_bps ≤ BPS − MIN_IDLE_BPS` always.
5. Nobody gets more USDC than their shares entitle them to.

### 12.3 Testnet test (`scripts/e2e-testnet.sh`)

Using the `stellar` CLI against the real DeFindex strategy:
1. build and deploy (`asset` = Blend USDC, `owner` = admin account, `timelock` = 60, `deposit_cap` = `i128::MAX`);
2. Fondex's initial deposit (1 USDC) — mitigates the inflation attack;
3. `propose_strategy(CALLOM5…, 9000)` → wait 60 s → `accept_strategy`;
4. a user deposits 100 USDC → `rebalance` → check ~90 in the strategy;
5. wait → `total_assets` goes up (Blend interest);
6. withdraw 50 USDC (pulled from the strategy) → withdraw everything with `redeem`.

### 12.4 Redeploying to testnet with one command (`scripts/deploy-testnet.sh`)

Since the contract is not upgradeable, fixing something on testnet = deploying a new one. The script sets everything up with no manual steps:

1. `make ci` (if it fails, it doesn't deploy);
2. `stellar contract build` and `stellar contract deploy` with the constructor arguments (`deposit_cap = i128::MAX`, `timelock = 60`);
3. `propose_strategy(DeFindex strategy, 9000)` → wait for the timelock → `accept_strategy()`;
4. Fondex's initial deposit (1 USDC);
5. prints the new address and adds it to the README registry (date, address);
6. optionally updates `VITE_VAULT_CONTRACT_ID` in `app/.env.local` (with confirmation).

Public configuration (no keys) in `scripts/testnet.env`: admin identity in `stellar keys`, `testnet` network, strategy and USDC addresses. Keys live only in `stellar keys`.

## 13. Implementation status

Everything described in this document is implemented and tested:

- **84 tests** in `make ci`: shares and rounding, strategies, rebalance, administration, pause, deposit cap, unresponsive strategy, and **invariants** with 64 random sequences of 40 operations (plus a long 500 × 60 version that is run by hand).
- **Deployed on testnet** (`CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS`) against the real DeFindex strategy. The end-to-end test passed: deposit, 90/10 rebalance, Blend interest, a redemption larger than the reserve, and a full redemption.
- **Web app** (`app/`) connected: fund composition, deposit, redeem and rebalance.

## 14. Risks

| Risk | Mitigation |
|---|---|
| Blend without liquidity (pool close to 95% utilization) | Reserve ≥ 5% (10% target); clear `InsufficientLiquidity`; the app shows the maximum withdrawable amount |
| Bad debt on Blend | Blend's *backstop* absorbs first losses; if not enough, the share price drops for everyone |
| DeFindex strategy with a bug | Audited (OtterSec, 2025-03), no upgrades; Fondex can pause and remove it |
| Malicious admin adds a trap strategy | Timelock (48 h on mainnet) + multisig + at most 4 strategies |
| First-deposit inflation attack | 10-decimal offset + Fondex's initial deposit |
| Strategy that doesn't report its balance | Deposits are blocked; redemptions keep valuing it at 0 (§10.9) |
| DeFindex keeper stops reinvesting BLND | Only that extra is lost; base interest continues |
| Undetected bug (the contract can't be patched) | Tests + audit; on mainnet, a low `deposit_cap` at first to limit how much money is exposed; if there is a bug: pause, deploy a fixed version and have users migrate |

## 15. Open questions (not blocking implementation)

1. Fees.
2. Legal: Law 20.712 and the Fintech Law (21.521), before mainnet.
3. Who signs the admin multisig account on mainnet.
4. DeFindex strategy for mainnet: look up its address in `paltalabs/defindex/public/mainnet.contracts.json` and verify the Blend pool it points to.
5. What to do with a strategy that stops responding **forever**? Today it can't be removed (§10.9). Future option: a timelocked `force_remove_strategy` that takes it off the list without withdrawing (whatever it holds stays in the vault's name and could be recovered if it is added again).

## 16. Keeper: automatic rebalancing (pending)

Depositing doesn't invest (§10.3), and a contract can't run by itself: someone has to call `rebalance()`. Today anyone does it from the app ("Rebalance" button). In production a **keeper** will do it: a Fondex process that checks the fund and calls `rebalance()` when needed, so the user only deposits and redeems.

- **No special permissions:** `rebalance()` is public. The keeper uses its own account with XLM only for fees; it is **not** the owner account and it cannot move money out of the fund. If the keeper goes down or is compromised, the worst case is that no rebalancing happens (redemptions keep working), and anyone can call `rebalance()` instead.
- **When to run:** every N minutes (e.g. 15), it simulates `strategies()`, `strategy_balances()`, `idle_assets()` and `total_assets()` and computes the deviation like the app does (`src/lib/composition.ts`). It only sends the transaction if some row exceeds `REBALANCE_TOLERANCE_BPS` and the movement would be ≥ `MIN_STRATEGY_MOVE`; that way it doesn't overpay fees. Do not run if `paused()`.
- **Where it lives:** in the Rust backend (a separate crate, e.g. `fondex-keeper`), with `stellar-rpc-client` / `soroban-client`, or a scheduled script at first. Configuration: RPC, contract, keeper identity.
- **Monitoring:** log every `rebalance` event (`total_assets_before/after`) and alert if `total_assets_after < total_assets_before − 2 × WITHDRAW_ROUNDING_BUFFER`, if the keeper account runs out of XLM, or if `rebalance` fails several times in a row.
- **App:** once the keeper is active, the "Rebalance" button is hidden or moved to an advanced section.

## Sources (consulted 2026-10-12)

- DeFindex: `paltalabs/defindex` (commit `a0fcabf`, 2026-07-01) — `strategies/core` (interface and errors), `strategies/blend/src/lib.rs` and `utils.rs` (behavior), `public/testnet.contracts.json` (addresses). GPL-3.0 license: we don't copy code, we only call the deployed contract.
- Blend v2: `blend-capital/blend-contracts-v2`, `blend-capital/blend-utils/testnet.contracts.json`, https://docs.blend.capital
- OpenZeppelin Stellar 0.7.2: `stellar-tokens` (fungible, vault), `stellar-access` (ownable), `stellar-contract-utils` (pausable, math).
- `soroban-sdk` 26.1.0: `Env::authorize_as_current_contract`.
- Live queries to testnet: strategy `CALLOM5…` (asset = Blend USDC, pool `TestnetV2`, no upgrade function) and pool `TestnetV2` (active).
