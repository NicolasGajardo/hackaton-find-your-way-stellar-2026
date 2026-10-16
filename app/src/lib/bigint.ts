/** Normalizes i128 values from the SDK (bigint | string | number) to bigint. */
export function toBigInt(value: unknown): bigint {
  if (typeof value === 'bigint') {
    return value
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value) || !Number.isInteger(value)) {
      throw new TypeError(`No se puede convertir number a bigint: ${value}`)
    }
    return BigInt(value)
  }
  if (typeof value === 'string') {
    return BigInt(value)
  }
  throw new TypeError(`Unsupported type for bigint: ${typeof value}`)
}
