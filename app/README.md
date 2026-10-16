# Fondex App

Web interface for Fondex's USDC mutual fund on **Stellar testnet**: connect a wallet, view the fund's state and your position, deposit, redeem and rebalance.

## Requirements

- Node.js 20+ and npm
- [Freighter](https://www.freighter.app/) (or another wallet supported by [Stellar Wallets Kit](https://stellarwalletskit.dev/)) on testnet

## Installation

```bash
cd app
npm install
```

The contract bindings live in `packages/fondex-vault`. If you change the contract on testnet, regenerate them:

```bash
stellar contract bindings typescript \
  --network testnet \
  --contract-id CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS \
  --output-dir packages/fondex-vault \
  --overwrite
npm install
```

(`postinstall` applies `scripts/patch-vault-bindings.mjs`; without it, `npm run dev` may fail because of SDK re-exports in the bindings.)

## Development

```bash
npm run dev
```

Open the URL Vite prints (by default `http://localhost:5173`).

## Scripts

| Command         | Description              |
|-----------------|--------------------------|
| `npm run dev`   | Development server       |
| `npm run build` | Production build         |
| `npm run lint`  | Oxlint                   |
| `npm test`      | Unit tests (Vitest)      |

## Configuration

Create `.env.local` (it is in `.gitignore`):

```bash
VITE_VAULT_CONTRACT_ID=CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS
VITE_USDC_CONTRACT_ID=CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU
```

Those values are also the defaults in `src/config.ts`. Optional:

- `VITE_STELLAR_RPC_URL` — Soroban RPC (default: public testnet)
- `VITE_HORIZON_URL` — testnet Horizon

The USDC is Blend's: code `USDC`, issuer `GATALTGTWIOT6BUDBCZM3Q4OQ4BO2COLOAZ7IYSKPLC2PMSOPPGF5V56`.

## The fund on testnet

Vault: `CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS`.

It receives USDC deposits and keeps a reserve. When someone calls **Rebalance**, it invests the rest in the DeFindex Blend strategy (`CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY`): depositing does not invest on its own. Redemptions are paid from the reserve first; if it is not enough, the same transaction pulls the difference from Blend.

### Blend test USDC

This USDC is **not** Circle's testnet USDC. To get it:

1. Set your wallet to **Testnet** and fund the account with [Friendbot](https://lab.stellar.org/account/fund) if it doesn't exist yet.
2. Open https://testnet.blend.capital and use the faucet button at the top of the page. It hands out test USDC and opens the trustline. It only shows up if your account is missing a trustline to any of Blend's assets.

### Try the fund

1. `npm run dev` and open the app.
2. Without connecting a wallet, **The fund** shows the total, the shares, the share price, the return since inception (the share started at 1 USDC) and the composition.
3. **Connect wallet** → choose Freighter and authorize.
4. If your Blend USDC balance is 0, **Deposit** shows a link to the faucet.
5. Enter an amount (or **Max**) and tap **Deposit**. The maximum comes from `max_deposit`.
6. If the composition drifted more than 2 points from the target, **Rebalance** is highlighted. Anyone can call it; you only pay the network fee.
7. In **Redeem**, an amount uses `withdraw`, and **Max** burns all your shares with `redeem`. The text below the form shows how much comes out of the reserve.
