# Fondex

**Decentralized mutual funds on Stellar.**

**[▶ Try the live demo](https://fondex-demo.vercel.app)**

Anyone can invest in a fund with the digital assets they already hold on Stellar (stablecoins like USDC, local currencies issued by anchors like CLP, or crypto assets like BTC) and redeem at any time. A public smart contract enforces the rules, not an intermediary.

## How it works

1. **Deposit** the fund's asset and receive shares.
2. **The fund invests** in liquid, low-risk strategies and keeps a reserve for instant redemptions.
3. **Yield raises the share price.**
4. **Redeem** whenever you want: the contract pays you immediately.

## What's live today

For this hackathon, the first fund is **100% USDC, on testnet**:

- 90% lent on [Blend](https://www.blend.capital/) through an audited [DeFindex](https://www.defindex.io/) strategy, called directly on-chain (no API or off-chain servers).
- 10% kept as a liquidity reserve. Larger redemptions pull the rest from Blend in the same transaction.

Each new asset (CLP, BTC, …) will be its own fund, with its own risk profile.

> ⚠️ **Testnet prototype.** The contracts are not audited. Do not use with real money.

## Deployed contracts (testnet)

| Role | Contract | Address |
|---|---|---|
| The fund | FondexVault: holds USDC and issues `fxFUND` shares | [`CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS`](https://stellar.expert/explorer/testnet/contract/CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS) |
| Fund asset | Blend test USDC (SAC) | [`CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU`](https://stellar.expert/explorer/testnet/contract/CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU) |
| Strategy | DeFindex Blend USDC: where 90% of the fund is lent | [`CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY`](https://stellar.expert/explorer/testnet/contract/CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY) |

## Structure

```
contracts/   Soroban (Rust) vault contract, tests and invariants, testnet scripts
app/         Web app (Vite + React + TypeScript): connect a wallet, deposit, redeem, view the composition and rebalance
docs/        Vision, business model and technical design
```

## Try the app

Requirements: Node.js 22 and a Stellar wallet set to **testnet** (recommended: [Freighter](https://www.freighter.app/); other wallets supported by [Stellar Wallets Kit](https://stellarwalletskit.dev/), such as xBull or Hana, also work).

```bash
cd app
npm install
npm run dev
```

Open `http://localhost:5173` and connect your wallet.

To deposit you need **Blend test USDC** (it is not the same USDC as other faucets):
1. Go to [testnet.blend.capital](https://testnet.blend.capital) with your wallet on testnet.
2. Use the faucet button at the top of the page. It only shows up if your account is missing a trustline to any of Blend's assets.

More details in [`app/README.md`](app/README.md).

## Contracts

Requirements: Rust, [Stellar CLI](https://developers.stellar.org/docs/tools/cli) 26 and `make`.

```bash
cd contracts
make ci          # format, clippy, tests (including invariants with random sequences) and WASM build
```

- One-command testnet deployment: `./scripts/deploy-testnet.sh`.
- End-to-end test against Blend: `./scripts/e2e-testnet.sh <VAULT_ID>`.

More details in [`contracts/README.md`](contracts/README.md).

### Contract guarantees

- **Fondex operates, the user owns:** the fund's assets can only go to approved strategies or to whoever redeems their shares. No role can transfer them to any other address.
- **Redemptions always available:** pausing stops deposits and rebalancing, never redemptions.
- **Public timelock** for adding strategies, at most 4 strategies and a minimum 5% reserve.
- Configurable **deposit cap**, to limit risk early on.
- **Not upgradeable:** nobody can change the rules after deployment.
- **Resilient:** if a strategy stops responding, deposits are blocked and redemptions keep working.

## Documentation

- [Summary: idea and business model](docs/00-summary.md)
- [Vision and decisions](docs/01-vision.md)
- [Contract technical design](docs/02-technical-design.md)

## Status and next steps

- ✅ **Done (testnet):** USDC mutual fund investing in Blend via DeFindex, reserve, rebalancing, timelocked administration, pause, deposit cap and web app.
- **Next:** keeper (automatic rebalancing), fees, audit.
- **Future:** funds for other assets (wBTC, wETH, XLM…) and RWAs with oracles.

This repository is a public copy of the code for demonstration purposes.

## License

[Apache-2.0](LICENSE).
