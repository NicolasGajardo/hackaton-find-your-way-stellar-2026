# Fondex — Soroban contracts

`contracts/fondex_vault` is the mutual fund vault: it receives USDC, issues `fxFUND` shares (a SEP-41 token inside the same contract), invests in external strategies through `rebalance` and keeps a reserve to pay redemptions.

Built with OpenZeppelin Stellar Contracts 0.7.2 (`stellar-tokens`, `stellar-access`, `stellar-contract-utils`) and `soroban-sdk` 26.1. The full design is in [`../docs/02-technical-design.md`](../docs/02-technical-design.md).

## How it works

- **Constructor:** `__constructor(asset, owner, strategy_timelock, deposit_cap)`. Takes the USDC address, the owner (OpenZeppelin Ownable), the timelock in seconds before a proposed strategy can be accepted (testnet: `60`) and the deposit cap (`i128::MAX` on testnet).
- **Depositing (`deposit`) does not invest.** Anyone can call `rebalance` to move USDC toward each strategy's `target_bps`.
- **Redeeming (`withdraw` / `redeem`) always works**, even with the fund paused. It pays from the reserve first and pulls whatever is missing from the strategies.
- **Admin (`owner`):**
  - `propose_strategy` → wait for the timelock → `accept_strategy`;
  - `cancel_strategy`, `set_target`, `pause_strategy` / `unpause_strategy`, `remove_strategy`;
  - `set_deposit_cap` and `pause` / `unpause` (global);
  - two-step ownership transfer: `transfer_ownership` / `accept_ownership`.

  The sum of allocations, including paused strategies, is capped at 95%: at least 5% always stays in reserve.
- **Deposit cap:** `deposit` rejects amounts that would exceed `deposit_cap`. `max_deposit` is `0` if the fund is paused or if any strategy doesn't respond.
- **Unresponsive strategy:** if a strategy's balance query fails, `deposit` and `preview_deposit` fail with `StrategyUnavailable`. Redemptions use a conservative valuation (that strategy counts as 0) and `rebalance` skips it.
- **Rounding always favors the vault.** Shares have 17 decimals (USDC's 7 + an offset of 10), which mitigates the inflation attack on an empty vault.

## Build and test

Requirements: Rust with the `wasm32v1-none` target, [Stellar CLI](https://developers.stellar.org/docs/tools/cli) 26 and `make`.

```bash
make ci     # format, clippy (-D warnings), tests and WASM build
```

The tests include a fake strategy (`src/test/mock_strategy.rs`) that simulates interest, losses, lack of liquidity and rounding, plus invariant tests with reproducible random sequences. There is a long version of those tests that is run by hand: `cargo test -p fondex-vault -- --ignored`.

WASM: `target/wasm32v1-none/release/fondex_vault.wasm`.

## Deploy to testnet

Uses **Blend test USDC**. The public configuration (no keys) is in `scripts/testnet.env`, and any variable can be overridden with `VAR=…`.

You need the `dev` identity in `stellar keys`, with Blend USDC in that account: you can get it at https://testnet.blend.capital, and the faucet also opens the trustline.

```bash
./scripts/deploy-testnet.sh              # make ci + deploy + strategy + initial deposit
./scripts/deploy-testnet.sh --check-only # preflight checks only
./scripts/deploy-testnet.sh --update-app # asks before writing ../app/.env.local
```

The script adds a row to the table below and does **not** commit.

### End-to-end test

```bash
./scripts/e2e-testnet.sh <VAULT_ID>
```

1. Uses a separate identity, `e2e-user`. The first time it creates it and funds it with friendbot, and `dev` transfers 100 Blend USDC to it.
2. Deposits and calls `rebalance` as that user.
3. Checks the ~90/10 split and waits for Blend interest.
4. Redeems 50 USDC, pulled from the strategy, and then redeems the rest.

## FondexVault deployments (testnet)

| Date | Address | Note |
|---|---|---|
| 2026-10-13 | `CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS` | Blend USDC, target 9000 bps, initial deposit 1 USDC |
