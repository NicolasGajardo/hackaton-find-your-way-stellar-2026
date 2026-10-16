import { USDC_DECIMALS } from '../config'
import { formatUnits } from './units'

/** 10_000 basis points = 100%. */
export const BPS = 10_000n

/** Contract tolerance: don't highlight if the deviation is 2 points or less. */
export const REBALANCE_TOLERANCE_BPS = 200n

export type StrategySlice = {
  address: string
  balance: bigint
  /** Target stored in the contract, even when the strategy is paused. */
  targetBps: bigint
  paused: boolean
}

export type CompositionRow = {
  key: string
  kind: 'strategy' | 'reserve'
  address: string | null
  balance: bigint
  /** Current share of the fund, in basis points. */
  currentBps: bigint
  /**
   * Target percentage to display. For a strategy it is the stored `target_bps`.
   * For the reserve it is `10_000 − Σ targets of the non-paused strategies`.
   */
  targetBps: bigint
  paused: boolean
}

export type Composition = {
  rows: CompositionRow[]
  /** Some row exceeds the tolerance. An empty fund is never considered off target. */
  offTarget: boolean
}

export function currentBps(balance: bigint, total: bigint): bigint {
  if (total <= 0n) {
    return 0n
  }
  return (balance * BPS) / total
}

/** Reserve target: 100% minus the targets of the active strategies. */
export function reserveTargetBps(strategies: readonly StrategySlice[]): bigint {
  let active = 0n
  for (const strategy of strategies) {
    if (!strategy.paused) {
      active += strategy.targetBps
    }
  }
  const target = BPS - active
  return target < 0n ? 0n : target
}

export function deviationBps(current: bigint, target: bigint): bigint {
  return current > target ? current - target : target - current
}

/**
 * Target used by the rebalance. A paused strategy targets 0;
 * the reserve already excludes those targets in `targetBps`.
 */
export function effectiveTargetBps(row: CompositionRow): bigint {
  if (row.kind === 'strategy' && row.paused) {
    return 0n
  }
  return row.targetBps
}

export function buildComposition(
  totalAssets: bigint,
  idleAssets: bigint,
  strategies: readonly StrategySlice[],
): Composition {
  const strategyRows: CompositionRow[] = strategies.map((strategy) => ({
    key: strategy.address,
    kind: 'strategy',
    address: strategy.address,
    balance: strategy.balance,
    currentBps: currentBps(strategy.balance, totalAssets),
    targetBps: strategy.targetBps,
    paused: strategy.paused,
  }))

  const reserve: CompositionRow = {
    key: 'reserve',
    kind: 'reserve',
    address: null,
    balance: idleAssets,
    currentBps: currentBps(idleAssets, totalAssets),
    targetBps: reserveTargetBps(strategies),
    paused: false,
  }

  const rows = [...strategyRows, reserve]
  const offTarget =
    totalAssets > 0n &&
    rows.some(
      (row) =>
        deviationBps(row.currentBps, effectiveTargetBps(row)) > REBALANCE_TOLERANCE_BPS,
    )

  return { rows, offTarget }
}

/** Formats basis points as a percentage (1 bp = 0.01%). */
export function formatBps(bps: bigint): string {
  const negative = bps < 0n
  const abs = negative ? -bps : bps
  const whole = abs / 100n
  const frac = (abs % 100n).toString().padStart(2, '0').replace(/0+$/, '')
  const sign = negative ? '-' : ''
  return frac ? `${sign}${whole.toString()}.${frac}%` : `${sign}${whole.toString()}%`
}

/**
 * Return since inception. The share started at exactly 1 USDC,
 * so it is `(share_price − 1) / 1`, with up to 6 decimals.
 */
export function formatShareReturn(oneShareValueUsdc: bigint): string {
  const oneUsdc = 10n ** BigInt(USDC_DECIMALS)
  const scaled = ((oneShareValueUsdc - oneUsdc) * 100n * 1_000_000n) / oneUsdc
  return `${formatUnits(scaled, 6)}%`
}

/**
 * Reference annual rate for the projection (a fixed value for the demo, it
 * does not come from Blend). 600 bp = 6% per year.
 */
export const REFERENCE_APY_BPS = 600n

/** Invested part of the fund; the liquid reserve earns nothing. */
const INVESTED_BPS = 9_000n

/**
 * Projected 30-day return, in basis points:
 * `annual_rate × 90% × 30 / 365`.
 */
export function projectedReturn30dBps(): bigint {
  return (REFERENCE_APY_BPS * INVESTED_BPS * 30n) / (BPS * 365n)
}
