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
  it('splits 90/10 between the strategy and the reserve', () => {
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

  it('with an empty fund it does not divide by zero or flag a deviation', () => {
    const composition = buildComposition(0n, 0n, [strategy()])

    expect(composition.rows[0]?.currentBps).toBe(0n)
    expect(composition.rows[1]?.currentBps).toBe(0n)
    expect(composition.rows[1]?.targetBps).toBe(1000n)
    expect(composition.offTarget).toBe(false)
  })

  it('a paused strategy is left out of the reserve target', () => {
    const composition = buildComposition(100n, 100n, [
      strategy({ balance: 0n, paused: true }),
    ])

    const [invested, reserve] = composition.rows
    expect(invested?.paused).toBe(true)
    expect(invested?.targetBps).toBe(9000n)
    expect(reserve?.targetBps).toBe(10_000n)
    expect(composition.offTarget).toBe(false)
  })

  it('does not highlight at 2 points but does at 2.01 points', () => {
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
  it('is 0% when the share is worth 1 USDC', () => {
    expect(formatShareReturn(10_000_000n)).toBe('0%')
  })

  it('shows up to 6 decimals', () => {
    expect(formatShareReturn(10_000_001n)).toBe('0.00001%')
  })
})
