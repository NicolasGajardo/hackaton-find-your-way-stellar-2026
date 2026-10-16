# Fondex — Vision and initial decisions

> Fondex = decentralized mutual **funds** (*fondos mutuos descentralizados* in Spanish) on the Stellar network.
> Last updated: 2026-10-12

## Vision

That **anyone can save and invest** in a mutual fund: cheap, transparent, with no minimum amounts and with redemption whenever they want, using smart contracts.

For Fondex, *decentralized* means **financial freedom**: the user can see at any time where their money is and which rules govern it, because those rules live in public contracts.

## Product profile: mutual fund

Fondex is a **decentralized mutual fund**, not an investment fund. In practice this means:

- **Liquidity first:** the user can redeem at any time and is paid immediately. The fund keeps a reserve for this.
- **Low risk:** the goal is a stable yield on digital dollars, not maximizing profits.
- **Only liquid, low-risk strategies**, according to the investment policy ([technical design §2.1](02-technical-design.md)). Futures, leverage or volatile assets stay out of this fund; if they are ever offered, it would be as a separate fund, with its own risk profile.

## Decisions made

| Topic | Decision |
|---|---|
| Network | Stellar, Soroban smart contracts (Rust) |
| Product type | **Decentralized mutual fund** (2026-10-12) |
| Backend | Rust |
| Frontend | React (with help from v0) |
| How the fund invests | Its own vault + already deployed **DeFindex strategies** (today: USDC lending on Blend). Fondex does not write its own strategies for now. |
| Allocation | Fixed, set by the admin within the contract's limits. Manager role / voting = future feature. |
| Upgradeable contract | No |
| Legal framework | Chile — Law 20.712 (funds) and Fintech Law (21.521). To be validated with legal counsel. |
| Funding | Interested in applying for Stellar ecosystem funding (SCF). |

## Pending (undecided)

- **Economic model:** which fees are charged, how much and when.
- **Exact legal fit** of the product under Chilean regulation, including whether the term "mutual fund" can be used publicly.
- **Signers** of the admin multisig account and **initial deposit cap** for mainnet.

## Roadmap (draft)

1. **USDC mutual fund** ✅ (testnet): deposit, view your position and redeem; investing in Blend via DeFindex, liquidity reserve, rebalancing and deposit cap ([technical design](02-technical-design.md)). Everything verifiable on-chain.
2. **Keeper:** a Fondex bot that runs `rebalance` automatically (in the Rust backend); the user only deposits and redeems ([technical design §16](02-technical-design.md)).
3. **Fees and economic model.**
4. **Future:** more strategies that meet the policy, multiple funds, governance / voting.
   - **Multi-asset:** funds for other assets that Blend lends (wBTC, wETH, XLM…), one per asset. We start with USDC only to keep the prototype simple.
   - **Oracles and RWAs:** not forbidden, just out of the prototype. They will be needed for funds that mix assets and for RWAs (tokenized real-world assets).
5. **Long term (if the product evolves significantly):** an MCP server in the Rust backend ([rmcp](https://github.com/modelcontextprotocol/rust-sdk)) so users can operate Fondex from AI assistants (check shares, deposit with the user's signature). Does not apply to the Soroban contracts.

## Open notes

- **Synthetic assets:** a synthetic tracks the price of another asset, so it keeps the market risk and adds oracle, collateral and counterparty risk. Given the mutual fund profile, they **do not go** into this fund.
- **Oracles:** not forbidden for Fondex; they were left out of the prototype to avoid adding risk. When they are used (funds that mix assets, RWAs), choose oracles with multiple sources and protect against manipulated or stale prices.
- **Regulation:** managing third-party money in a pooled fund falls under fund regulation (Law 20.712), whether mutual fund or investment fund, as well as the Fintech Law. Requires review with a specialized lawyer before operating with real money.
