import { describe, expect, it } from 'vitest'
import { formatUnits, parsePositiveUnits, parseUnits, UnitsParseError } from './units'

describe('parseUnits / formatUnits (7 decimales — USDC)', () => {
  it('parsea 1 USDC', () => {
    expect(parseUnits('1', 7)).toBe(10_000_000n)
  })

  it('parsea decimales hasta 7 lugares', () => {
    expect(parseUnits('1.2345678', 7)).toBe(12_345_678n)
    expect(parseUnits('0.0000001', 7)).toBe(1n)
  })

  it('redondea hacia abajo fracciones extra', () => {
    expect(parseUnits('1.99999999', 7)).toBe(19_999_999n)
  })

  it('formatea y revierte montos grandes', () => {
    const huge = 9_999_999_999_999_999n * 10_000_000n
    expect(formatUnits(huge, 7)).toBe('9999999999999999')
    expect(parseUnits('9999999999999999', 7)).toBe(huge)
  })

  it('rechaza formato inválido', () => {
    expect(() => parseUnits('1,5', 7)).toThrow(UnitsParseError)
  })
})

describe('parseUnits / formatUnits (17 decimales — cuotas)', () => {
  const oneShare = 10n ** 17n

  it('parsea 1 cuota', () => {
    expect(parseUnits('1', 17)).toBe(oneShare)
  })

  it('mantiene precisión en montos pequeños', () => {
    expect(parseUnits('0.00000000000000001', 17)).toBe(1n)
  })

  it('trunca más de 17 decimales', () => {
    const input = '1.' + '9'.repeat(20)
    expect(parseUnits(input, 17)).toBe(2n * oneShare - 1n)
  })

  it('formatea cuotas con ceros finales recortados', () => {
    expect(formatUnits(oneShare, 17)).toBe('1')
    expect(formatUnits(oneShare / 2n, 17)).toBe('0.5')
  })

  it('montos grandes sin perder precisión', () => {
    const value = 123_456_789_012_345_678_901_234_567_890n
    const asString = formatUnits(value, 17)
    expect(parseUnits(asString, 17)).toBe(value)
  })
})

describe('parsePositiveUnits', () => {
  it('devuelve el monto si es positivo', () => {
    expect(parsePositiveUnits('2.5', 7)).toBe(25_000_000n)
  })

  it('devuelve null si está vacío, es inválido o no es positivo', () => {
    expect(parsePositiveUnits('', 7)).toBeNull()
    expect(parsePositiveUnits('abc', 7)).toBeNull()
    expect(parsePositiveUnits('0', 7)).toBeNull()
    expect(parsePositiveUnits('-1', 7)).toBeNull()
  })
})
