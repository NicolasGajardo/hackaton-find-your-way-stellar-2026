import { describe, expect, it } from 'vitest'
import {
  buildComposition,
  formatBps,
  formatShareReturn,
  type StrategySlice,
} from './composition'

const strategy = (overrides: Partial<StrategySlice> = {}): StrategySlice => ({
  address: 'CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY',
  balance: 0n,
  targetBps: 9000n,
  paused: false,
  ...overrides,
})

describe('buildComposition', () => {
  it('reparte 90/10 entre la estrategia y la reserva', () => {
    const composition = buildComposition(100_000_000n, 10_000_000n, [
      strategy({ balance: 90_000_000n }),
    ])

    expect(composition.rows).toHaveLength(2)
    const [invested, reserve] = composition.rows
    expect(invested?.currentBps).toBe(9000n)
    expect(invested?.targetBps).toBe(9000n)
    expect(reserve?.kind).toBe('reserve')
    expect(reserve?.currentBps).toBe(1000n)
    expect(reserve?.targetBps).toBe(1000n)
    expect(composition.offTarget).toBe(false)
    expect(formatBps(invested?.currentBps ?? 0n)).toBe('90%')
    expect(formatBps(reserve?.currentBps ?? 0n)).toBe('10%')
  })

  it('con el fondo vacío no divide por cero y no marca desviación', () => {
    const composition = buildComposition(0n, 0n, [strategy()])

    expect(composition.rows[0]?.currentBps).toBe(0n)
    expect(composition.rows[1]?.currentBps).toBe(0n)
    expect(composition.rows[1]?.targetBps).toBe(1000n)
    expect(composition.offTarget).toBe(false)
  })

  it('una estrategia pausada no entra en el objetivo de la reserva', () => {
    const composition = buildComposition(100n, 100n, [
      strategy({ balance: 0n, paused: true }),
    ])

    const [invested, reserve] = composition.rows
    expect(invested?.paused).toBe(true)
    expect(invested?.targetBps).toBe(9000n)
    expect(reserve?.targetBps).toBe(10_000n)
    expect(composition.offTarget).toBe(false)
  })

  it('no resalta en 2 puntos y sí en 2,01 puntos', () => {
    const onEdge = buildComposition(10_000n, 1_200n, [
      strategy({ balance: 8_800n }),
    ])
    expect(onEdge.rows[0]?.currentBps).toBe(8800n)
    expect(onEdge.offTarget).toBe(false)

    const over = buildComposition(10_000n, 1_201n, [
      strategy({ balance: 8_799n }),
    ])
    expect(over.rows[0]?.currentBps).toBe(8799n)
    expect(over.offTarget).toBe(true)
  })
})

describe('formatShareReturn', () => {
  it('es 0% cuando la cuota vale 1 USDC', () => {
    expect(formatShareReturn(10_000_000n)).toBe('0%')
  })

  it('muestra hasta 6 decimales', () => {
    expect(formatShareReturn(10_000_001n)).toBe('0.00001%')
  })
})
