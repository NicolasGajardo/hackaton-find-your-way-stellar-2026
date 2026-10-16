import { describe, expect, it } from 'vitest'
import { formatUnits, parsePositiveUnits, parseUnits, UnitsParseError } from './units'

describe('parseUnits / formatUnits (7 decimals — USDC)', () => {
  it('parses 1 USDC', () => {
    expect(parseUnits('1', 7)).toBe(10_000_000n)
  })

  it('parses up to 7 decimal places', () => {
    expect(parseUnits('1.2345678', 7)).toBe(12_345_678n)
    expect(parseUnits('0.0000001', 7)).toBe(1n)
  })

  it('rounds down extra fractional digits', () => {
    expect(parseUnits('1.99999999', 7)).toBe(19_999_999n)
  })

  it('formats and round-trips large amounts', () => {
    const huge = 9_999_999_999_999_999n * 10_000_000n
    expect(formatUnits(huge, 7)).toBe('9999999999999999')
    expect(parseUnits('9999999999999999', 7)).toBe(huge)
  })

  it('rejects an invalid format', () => {
    expect(() => parseUnits('1,5', 7)).toThrow(UnitsParseError)
  })
})

describe('parseUnits / formatUnits (17 decimals — shares)', () => {
  const oneShare = 10n ** 17n

  it('parses 1 share', () => {
    expect(parseUnits('1', 17)).toBe(oneShare)
  })

  it('keeps precision on small amounts', () => {
    expect(parseUnits('0.00000000000000001', 17)).toBe(1n)
  })

  it('truncates more than 17 decimals', () => {
    const input = '1.' + '9'.repeat(20)
    expect(parseUnits(input, 17)).toBe(2n * oneShare - 1n)
  })

  it('formats shares with trailing zeros trimmed', () => {
    expect(formatUnits(oneShare, 17)).toBe('1')
    expect(formatUnits(oneShare / 2n, 17)).toBe('0.5')
  })

  it('large amounts without losing precision', () => {
    const value = 123_456_789_012_345_678_901_234_567_890n
    const asString = formatUnits(value, 17)
    expect(parseUnits(asString, 17)).toBe(value)
  })
})

describe('parsePositiveUnits', () => {
  it('returns the amount when it is positive', () => {
    expect(parsePositiveUnits('2.5', 7)).toBe(25_000_000n)
  })

  it('returns null when empty, invalid or not positive', () => {
    expect(parsePositiveUnits('', 7)).toBeNull()
    expect(parsePositiveUnits('abc', 7)).toBeNull()
    expect(parsePositiveUnits('0', 7)).toBeNull()
    expect(parsePositiveUnits('-1', 7)).toBeNull()
  })
})
