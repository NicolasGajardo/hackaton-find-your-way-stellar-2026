import { useCallback, useEffect, useState } from 'react'
import { USDC_DECIMALS } from '../config'
import { describeTxError } from '../lib/txErrors'
import { formatUnits, parsePositiveUnits } from '../lib/units'
import { depositUsdc, previewDeposit } from '../lib/vault'

export type DepositStatus =
  | { kind: 'idle' }
  | { kind: 'signing' }
  | { kind: 'success'; shares: bigint }
  | { kind: 'error'; message: string }

const PREVIEW_DELAY_MS = 400

export function useDeposit(
  address: string | null,
  usdcBalance: bigint | null,
  maxDeposit: bigint | null,
  paused: boolean,
  onDeposited: () => Promise<void>,
) {
  const [input, setInput] = useState('')
  const [preview, setPreview] = useState<bigint | null>(null)
  const [status, setStatus] = useState<DepositStatus>({ kind: 'idle' })

  const assets = parsePositiveUnits(input, USDC_DECIMALS)
  const exceedsBalance =
    assets !== null && usdcBalance !== null && assets > usdcBalance
  const exceedsCap = assets !== null && maxDeposit !== null && assets > maxDeposit

  useEffect(() => {
    setPreview(null)
    if (assets === null) {
      return
    }
    let cancelled = false
    const timer = setTimeout(() => {
      previewDeposit(assets)
        .then((shares) => {
          if (!cancelled) setPreview(shares)
        })
        .catch(() => {
          if (!cancelled) setPreview(null)
        })
    }, PREVIEW_DELAY_MS)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [assets])

  const submit = useCallback(async () => {
    if (!address || paused || assets === null || exceedsBalance || exceedsCap) {
      return
    }
    setStatus({ kind: 'signing' })
    try {
      const shares = await depositUsdc(address, assets)
      setStatus({ kind: 'success', shares })
      setInput('')
      await onDeposited()
    } catch (e) {
      setStatus({ kind: 'error', message: describeTxError(e) })
    }
  }, [address, assets, exceedsBalance, exceedsCap, paused, onDeposited])

  const setMax = useCallback(() => {
    if (usdcBalance === null || paused) {
      return
    }
    const capped =
      maxDeposit === null || usdcBalance < maxDeposit ? usdcBalance : maxDeposit
    if (capped > 0n) {
      setInput(formatUnits(capped, USDC_DECIMALS))
    }
  }, [usdcBalance, maxDeposit, paused])

  return {
    input,
    setInput: (value: string) => {
      setInput(value)
      if (status.kind !== 'signing') setStatus({ kind: 'idle' })
    },
    setMax,
    assets,
    preview,
    exceedsBalance,
    exceedsCap,
    paused,
    needsBlendUsdc: usdcBalance === 0n,
    status,
    submit,
    canSubmit:
      address !== null &&
      !paused &&
      maxDeposit !== null &&
      maxDeposit > 0n &&
      assets !== null &&
      !exceedsBalance &&
      !exceedsCap &&
      status.kind !== 'signing',
  }
}
