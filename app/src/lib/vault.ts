import type { AssembledTransaction } from '@stellar/stellar-sdk/contract'
import { Client } from 'fondex-vault'
import {
  NETWORK_PASSPHRASE,
  ONE_SHARE,
  RPC_URL,
  VAULT_CONTRACT_ID,
} from '../config'
import { StellarWalletsKit } from '../wallet/kit'
import { toBigInt } from './bigint'
import { WALLET_ACCOUNT_CHANGED } from './txErrors'

let client: Client | null = null

export function getVaultClient(): Client {
  if (!client) {
    client = new Client({
      contractId: VAULT_CONTRACT_ID,
      networkPassphrase: NETWORK_PASSPHRASE,
      rpcUrl: RPC_URL,
    })
  }
  return client
}

async function simulate<T>(run: () => Promise<AssembledTransaction<T>>): Promise<T> {
  const tx = await run()
  await tx.simulate()
  return tx.result
}

async function simulateI128(
  run: () => Promise<AssembledTransaction<bigint | string | number>>,
): Promise<bigint> {
  return toBigInt(await simulate(run))
}

export type StrategyPosition = {
  address: string
  balance: bigint
  targetBps: bigint
  paused: boolean
}

export type VaultFundStats = {
  totalAssets: bigint
  totalSupply: bigint
  oneShareValueUsdc: bigint
  idleAssets: bigint
  paused: boolean
  strategies: StrategyPosition[]
}

export async function fetchVaultFundStats(): Promise<VaultFundStats> {
  const vault = getVaultClient()
  const [totalAssets, totalSupply, oneShareValueUsdc, idleAssets, paused, strategies, balances] =
    await Promise.all([
      simulateI128(() => vault.total_assets()),
      simulateI128(() => vault.total_supply()),
      simulateI128(() => vault.convert_to_assets({ shares: ONE_SHARE })),
      simulateI128(() => vault.idle_assets()),
      simulate(() => vault.paused()),
      simulate(() => vault.strategies()),
      simulate(() => vault.strategy_balances()),
    ])

  const balanceByAddress = new Map(
    balances.map(([address, balance]) => [address, toBigInt(balance)]),
  )

  return {
    totalAssets,
    totalSupply,
    oneShareValueUsdc,
    idleAssets,
    paused,
    strategies: strategies.map((strategy) => ({
      address: strategy.address,
      balance: balanceByAddress.get(strategy.address) ?? 0n,
      targetBps: toBigInt(strategy.target_bps),
      paused: strategy.paused,
    })),
  }
}

/** Shares you would receive today for depositing `assets` (simulation, unsigned). */
export async function previewDeposit(assets: bigint): Promise<bigint> {
  return simulateI128(() => getVaultClient().preview_deposit({ assets }))
}

/** Shares that would be burned today to redeem `assets` USDC (simulation, unsigned). */
export async function previewWithdraw(assets: bigint): Promise<bigint> {
  return simulateI128(() => getVaultClient().preview_withdraw({ assets }))
}

/**
 * Client that signs with the connected wallet. `address` is the source account,
 * so the transaction signature also covers the contract's `require_auth`
 * (`from` on deposits, `owner` on redemptions). `rebalance` requires no contract
 * auth, but the wallet still signs the transaction to pay the fee.
 */
async function getSigningVaultClient(address: string): Promise<Client> {
  // The kit remembers the address across reloads, but Freighter may have lost
  // this site's permission. This requests access again (opening Freighter only
  // if needed) and confirms the active account is still the same one.
  const { address: walletAddress } = await StellarWalletsKit.fetchAddress()
  if (walletAddress !== address) {
    throw new Error(WALLET_ACCOUNT_CHANGED)
  }

  return new Client({
    contractId: VAULT_CONTRACT_ID,
    networkPassphrase: NETWORK_PASSPHRASE,
    rpcUrl: RPC_URL,
    publicKey: address,
    signTransaction: (xdr) =>
      StellarWalletsKit.signTransaction(xdr, {
        networkPassphrase: NETWORK_PASSPHRASE,
        address,
      }),
  })
}

async function signAndSendI128(
  tx: AssembledTransaction<bigint | string | number>,
): Promise<bigint> {
  const sent = await tx.signAndSend()
  return toBigInt(sent.result)
}

/** Deposits `assets` USDC from `address` and mints its shares. Returns the minted shares. */
export async function depositUsdc(
  address: string,
  assets: bigint,
): Promise<bigint> {
  const vault = await getSigningVaultClient(address)
  return signAndSendI128(
    await vault.deposit({ assets, receiver: address, from: address }),
  )
}

/** Redeems exactly `assets` USDC to `address`. Returns the burned shares. */
export async function withdrawUsdc(
  address: string,
  assets: bigint,
): Promise<bigint> {
  const vault = await getSigningVaultClient(address)
  return signAndSendI128(
    await vault.withdraw({ assets, receiver: address, owner: address }),
  )
}

/** Burns exactly `shares` shares from `address`. Returns the USDC received. */
export async function redeemShares(
  address: string,
  shares: bigint,
): Promise<bigint> {
  const vault = await getSigningVaultClient(address)
  return signAndSendI128(
    await vault.redeem({ shares, receiver: address, owner: address }),
  )
}

export type UserVaultStats = {
  shareBalance: bigint
  sharesValueUsdc: bigint
}

export async function fetchMaxDeposit(receiver: string): Promise<bigint> {
  return simulateI128(() => getVaultClient().max_deposit({ receiver }))
}

export async function fetchMaxWithdraw(owner: string): Promise<bigint> {
  return simulateI128(() => getVaultClient().max_withdraw({ owner }))
}

/** Moves USDC between the reserve and the strategies. Does not change user shares or balances. */
export async function rebalanceVault(address: string): Promise<void> {
  const vault = await getSigningVaultClient(address)
  const tx = await vault.rebalance()
  await tx.signAndSend()
}

export async function fetchUserVaultStats(
  account: string,
): Promise<UserVaultStats> {
  const vault = getVaultClient()
  const shareBalance = await simulateI128(() =>
    vault.balance({ account }),
  )
  const sharesValueUsdc = await simulateI128(() =>
    vault.convert_to_assets({ shares: shareBalance }),
  )
  return { shareBalance, sharesValueUsdc }
}
