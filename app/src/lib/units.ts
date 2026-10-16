const INTEGER_RE = /^-?\d+$/
const DECIMAL_RE = /^-?(\d+)(?:\.(\d+))?$/

export class UnitsParseError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'UnitsParseError'
  }
}

/**
 * Converts a human decimal string to smallest units (bigint).
 * Rounds down if there are more decimals than `decimals`.
 */
export function parseUnits(value: string, decimals: number): bigint {
  const trimmed = value.trim()
  if (!trimmed) {
    throw new UnitsParseError('The amount is empty')
  }
  if (decimals < 0 || decimals > 77) {
    throw new UnitsParseError('Invalid decimals')
  }

  if (INTEGER_RE.test(trimmed)) {
    return BigInt(trimmed) * 10n ** BigInt(decimals)
  }

  const match = DECIMAL_RE.exec(trimmed)
  if (!match) {
    throw new UnitsParseError('Invalid amount format')
  }

  const negative = trimmed.startsWith('-')
  const whole = match[1] ?? '0'
  const fraction = match[2] ?? ''

  if (fraction.length > decimals) {
    const kept = fraction.slice(0, decimals)
    const fracPadded = kept.padEnd(decimals, '0')
    const combined = BigInt(whole + fracPadded)
    return negative ? -combined : combined
  }

  const fracPadded = fraction.padEnd(decimals, '0')
  const combined = BigInt(whole + fracPadded)
  return negative ? -combined : combined
}

/** Like `parseUnits`, but returns `null` if the amount is empty, invalid or not positive. */
export function parsePositiveUnits(value: string, decimals: number): bigint | null {
  try {
    const amount = parseUnits(value, decimals)
    return amount > 0n ? amount : null
  } catch (e) {
    if (e instanceof UnitsParseError) {
      return null
    }
    throw e
  }
}

/**
 * Formats smallest units as a decimal string without scientific notation.
 */
export function formatUnits(
  value: bigint,
  decimals: number,
  options?: { maxFractionDigits?: number; trimTrailingZeros?: boolean },
): string {
  const { maxFractionDigits, trimTrailingZeros = true } = options ?? {}
  const negative = value < 0n
  const abs = negative ? -value : value

  if (decimals === 0) {
    return `${negative ? '-' : ''}${abs.toString()}`
  }

  const base = 10n ** BigInt(decimals)
  const whole = abs / base
  let fraction = (abs % base).toString().padStart(decimals, '0')

  if (maxFractionDigits !== undefined) {
    if (maxFractionDigits <= 0) {
      fraction = ''
    } else if (maxFractionDigits < decimals) {
      fraction = fraction.slice(0, maxFractionDigits)
    }
  }

  if (trimTrailingZeros && fraction) {
    fraction = fraction.replace(/0+$/, '')
  }

  const sign = negative ? '-' : ''
  if (!fraction) {
    return `${sign}${whole.toString()}`
  }
  return `${sign}${whole.toString()}.${fraction}`
}

export function abbreviateAddress(address: string, chars = 4): string {
  if (address.length <= chars * 2 + 1) {
    return address
  }
  return `${address.slice(0, chars)}…${address.slice(-chars)}`
}
