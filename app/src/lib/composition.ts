import { USDC_DECIMALS } from '../config'
import { formatUnits } from './units'

/** 10_000 puntos base = 100%. */
export const BPS = 10_000n

/** Tolerancia del contrato: no resaltar si la desviación es de 2 puntos o menos. */
export const REBALANCE_TOLERANCE_BPS = 200n

export type StrategySlice = {
  address: string
  balance: bigint
  /** Objetivo guardado en el contrato, también si la estrategia está pausada. */
  targetBps: bigint
  paused: boolean
}

export type CompositionRow = {
  key: string
  kind: 'strategy' | 'reserve'
  address: string | null
  balance: bigint
  /** Porcentaje actual del fondo, en puntos base. */
  currentBps: bigint
  /**
   * Porcentaje objetivo a mostrar. En una estrategia es `target_bps` guardado.
   * En la reserva es `10_000 − Σ objetivos de las estrategias no pausadas`.
   */
  targetBps: bigint
  paused: boolean
}

export type Composition = {
  rows: CompositionRow[]
  /** Alguna fila supera la tolerancia. Un fondo vacío no se considera desviado. */
  offTarget: boolean
}

export function currentBps(balance: bigint, total: bigint): bigint {
  if (total <= 0n) {
    return 0n
  }
  return (balance * BPS) / total
}

/** Objetivo de la reserva: 100% menos los objetivos de las estrategias activas. */
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
 * Objetivo que usa el rebalanceo. Una estrategia pausada apunta a 0;
 * la reserva ya excluye esos objetivos en `targetBps`.
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

/** Formatea puntos base como porcentaje (1 pb = 0,01%). */
export function formatBps(bps: bigint): string {
  const negative = bps < 0n
  const abs = negative ? -bps : bps
  const whole = abs / 100n
  const frac = (abs % 100n).toString().padStart(2, '0').replace(/0+$/, '')
  const sign = negative ? '-' : ''
  return frac ? `${sign}${whole.toString()}.${frac}%` : `${sign}${whole.toString()}%`
}

/**
 * Rentabilidad desde el inicio. La cuota partió valiendo exactamente 1 USDC,
 * así que es `(valor_cuota − 1) / 1`, con hasta 6 decimales.
 */
export function formatShareReturn(oneShareValueUsdc: bigint): string {
  const oneUsdc = 10n ** BigInt(USDC_DECIMALS)
  const scaled = ((oneShareValueUsdc - oneUsdc) * 100n * 1_000_000n) / oneUsdc
  return `${formatUnits(scaled, 6)}%`
}

/**
 * Tasa anual de referencia para la proyección (dato fijo para la demo, no
 * viene de Blend). 600 pb = 6% al año.
 */
export const REFERENCE_APY_BPS = 600n

/** Parte invertida del fondo; la reserva líquida no rinde. */
const INVESTED_BPS = 9_000n

/**
 * Rentabilidad proyectada en 30 días, en puntos base:
 * `tasa_anual × 90% × 30 / 365`.
 */
export function projectedReturn30dBps(): bigint {
  return (REFERENCE_APY_BPS * INVESTED_BPS * 30n) / (BPS * 365n)
}
