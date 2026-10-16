# Fondex — Summary: idea and business model

> More detail in [01-vision.md](01-vision.md) and in the [technical design](02-technical-design.md).

## 1. The idea in one sentence

**Fondex offers decentralized mutual funds on Stellar: anyone can invest with the digital assets they already hold and redeem at any time, under rules enforced by a public smart contract instead of an intermediary you have to trust.**

## 2. The idea in detail

### The problem

Saving and investing in funds today is **expensive and hard to access**: minimum amounts, unclear fees, and investors can't see where their money is or verify the rules that govern it.

### The solution: a mutual fund in a smart contract

1. The user **deposits USDC** (digital dollars), with no minimum amount.
2. They receive **shares** (`fxFUND`) that represent their part of the fund.
3. The fund invests in **liquid, low-risk strategies** (today: USDC lending on Blend, through an audited DeFindex strategy) and keeps a **reserve** to pay redemptions.
4. Interest raises the **share price**.
5. The user **redeems whenever they want**: the contract pays them immediately.

### Why a "mutual fund" and not an "investment fund"?

By how it works, Fondex fits a **mutual fund**: redemption at any time, a continuously computed share price, liquid low-risk investments, and a reserve to pay redemptions. In particular, it resembles a **short-term dollar mutual fund** (a *money market* fund). An investment fund, in contrast, usually accepts less liquid assets, longer redemption periods and more risk.

### Golden rule: only liquid, low-risk strategies

The fund only invests in strategies that meet **all** the criteria of the **investment policy** ([technical design §2.1](02-technical-design.md)): same asset as the fund (USDC), immediate withdrawal, no leverage or derivatives, no exposure to the price of other assets, and audited contracts with real usage. Futures, leverage or volatile assets do **not** go into this fund; if they are ever offered, it would be in **another fund** with a different risk profile.

### Where it grows (future features)

We start with USDC only to **keep things simple and reach a working prototype quickly**, not because Fondex will be USDC-only.

- **Multi-asset:** Stellar holds many kinds of digital assets: stablecoins, local currencies issued by anchors (e.g. CLP) and crypto assets (e.g. BTC). Blend already lends several of them (wBTC, wETH, XLM, etc.). The plan is to support them later, as long as there is a strategy for that asset that meets the investment policy. Since the contract is not upgradeable, the natural approach is **one fund per asset**: the same contract structure deployed with a different asset and strategy (e.g. a "wBTC fund" that lends wBTC on Blend). It requires a new contract version with some configurable minimum amounts, since today they are tuned for USDC. A fund that **mixes** assets (e.g. 50% USDC + 50% wBTC) is more complex: it needs prices to value the share and would be a new contract.
- **Oracles and RWAs:** oracles (price feeds) are **not forbidden**; they were only left out of the prototype to avoid adding risk and complexity. They will be needed for funds that mix assets and for **RWAs** (tokenized real-world assets, such as bonds or treasury bills on the blockchain), which are a good fit for the mutual fund profile.
- **Risk profile:** a wBTC or wETH fund goes up and down with that asset's price, so it would **not** be a low-risk fund like the USDC one. It would be offered as a separate fund, with its risk profile explained to the user.

### Core principle: "Fondex operates, the user owns"

Fondex can **invest** the money, but never **take it**. The contract guarantees this, not a promise:

- **Money only goes to approved strategies** or back to whoever redeems their shares.
- **Adding a strategy has a public timelock**, so anyone has time to redeem if they disagree.
- **Redemptions always available:** no role can block them (pausing only stops deposits).
- **Non-upgradeable contract:** nobody can change the rules after deployment.
- **Explicit fees:** if there are any, they are written in the contract.

For Fondex, *decentralized* means **financial freedom**: the user can see at any time where their money is and which rules govern it.

### Roadmap

| What | Status |
|---|---|
| USDC mutual fund: deposits, shares, redemptions, investing in Blend via DeFindex, reserve, rebalancing, timelocked admin, pause and deposit cap | ✅ Contract and app on testnet |
| **Keeper**: a Fondex bot that runs `rebalance` automatically, so the user only deposits and redeems | Next ([design §16](02-technical-design.md)) |
| Fees and economic model | Next |
| More strategies that meet the policy, **funds for other assets** (wBTC, wETH, XLM…), **RWAs** (tokenized real-world assets) with oracles, governance | Future (see "Where it grows") |
| Operating Fondex from AI assistants (MCP) | Long term |

### Technology

Stellar (fees of fractions of a cent, native USDC). Soroban contracts (Rust), Rust backend, React frontend. Builds on existing ecosystem protocols: **Blend** (lending) and **DeFindex strategies** (called directly, without their API).

## 3. Business model

### Decided

- Fondex **earns through fees and/or usage agreements**.
- Every fee is **a public rule in the contract**, with a maximum cap; never a discretionary charge.

### Open: what to charge

| Fee | How it works | Pros | Cons |
|---|---|---|---|
| **Management** | Annual % of assets under management, charged gradually | Predictable income; standard for mutual funds | Charged even if the fund loses money |
| **Performance** | % of profits | Aligns Fondex with the user | Irregular income; harder to compute on-chain |
| **Deposit** | % on entry | Simple | Discourages entering: contrary to "anyone can invest" |
| **Redemption** | % on exit | Simple | Clashes with "redeem whenever you want" |

**Proposal (to be discussed):** a **low management fee** (e.g. < 1% per year) as the base. It is consistent with the promise of being cheap and easy to explain. Technically it is charged by minting new shares to Fondex, so the money never leaves the vault and the "can't take it" principle holds. Note: in a low-risk fund the yield is limited, so the fee must be low for the user to still earn.

### Other sources of income or funding

- **"Usage agreements":** not yet defined. It could mean charging third parties for using Fondex's infrastructure.
- **Stellar ecosystem funding (SCF):** interested in applying. Not recurring income, but it can fund initial development.

## 4. Risks and open questions

- **Legal:** in Chile, Law 20.712 regulates **both mutual funds and investment funds** (managed by a fund manager, or AGF, supervised by the CMF), in addition to the Fintech Law (21.521). Switching the focus to "mutual fund" is more accurate conceptually, but it **does not take Fondex out of regulation**, and "mutual fund" (*fondo mutuo*) is a legal term. Validate with a lawyer before operating with real money or using the term publicly. **Probably the biggest risk in the business model.**
- **Risk of the protocols used:** bad debt or lack of liquidity on Blend; a bug in the DeFindex strategy. Mitigated with the reserve, the investment policy, the deposit cap and the ability to pause or remove strategies.
- **"Low risk" is not "no risk":** the share price can go down if a protocol suffers losses. This must be communicated clearly to the user.
- **To be decided:** fees, signers of the admin multisig account, and the initial deposit cap for mainnet.
