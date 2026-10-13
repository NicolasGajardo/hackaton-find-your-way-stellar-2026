import { useCallback, useEffect, useState } from 'react'
import { USDC_DECIMALS } from '../config'
import { describeTxError } from '../lib/txErrors'
import { formatUnits, parsePositiveUnits } from '../lib/units'
import {
  previewWithdraw,
  redeemShares,
  withdrawUsdc,
  type UserVaultStats,
} from '../lib/vault'

export type WithdrawStatus =
  | { kind: 'idle' }
  | { kind: 'signing' }
  | { kind: 'success'; usdc: bigint }
  | { kind: 'error'; message: string }

const PREVIEW_DELAY_MS = 400

export function useWithdraw(
  address: string | null,
  userVault: UserVaultStats | null,
  maxWithdraw: bigint | null,
  onWithdrawn: () => Promise<void>,
) {
  const [input, setInputValue] = useState('')
  /**
   * "Rescatar todo": se quema el saldo exacto de cuotas con `redeem` en vez de
   * pedir un monto de USDC con `withdraw`, para no dejar restos por redondeo.
   */
  const [redeemAll, setRedeemAll] = useState(false)
  const [preview, setPreview] = useState<bigint | null>(null)
  const [status, setStatus] = useState<WithdrawStatus>({ kind: 'idle' })

  const assets = parsePositiveUnits(input, USDC_DECIMALS)
  const maxAssets = maxWithdraw
  const exceedsPosition =
    !redeemAll && assets !== null && maxAssets !== null && assets > maxAssets

  useEffect(() => {
    setPreview(null)
    if (redeemAll || assets === null) {
      return
    }
    let cancelled = false
    const timer = setTimeout(() => {
      previewWithdraw(assets)
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
  }, [assets, redeemAll])

  const submit = useCallback(async () => {
    if (!address || !userVault || assets === null || exceedsPosition) {
      return
    }
    setStatus({ kind: 'signing' })
    try {
      let usdc = assets
      if (redeemAll) {
        usdc = await redeemShares(address, userVault.shareBalance)
      } else {
        await withdrawUsdc(address, assets)
      }
      setStatus({ kind: 'success', usdc })
      setInputValue('')
      setRedeemAll(false)
      await onWithdrawn()
    } catch (e) {
      setStatus({ kind: 'error', message: describeTxError(e) })
    }
  }, [address, userVault, assets, exceedsPosition, redeemAll, onWithdrawn])

  const setInput = useCallback(
    (value: string) => {
      setInputValue(value)
      setRedeemAll(false)
      if (status.kind !== 'signing') setStatus({ kind: 'idle' })
    },
    [status.kind],
  )

  const setMax = useCallback(() => {
    if (maxAssets !== null && maxAssets > 0n) {
      setInputValue(formatUnits(maxAssets, USDC_DECIMALS))
      setRedeemAll(true)
      if (status.kind !== 'signing') setStatus({ kind: 'idle' })
    }
  }, [maxAssets, status.kind])

  return {
    input,
    setInput,
    setMax,
    assets,
    maxAssets,
    redeemAll,
    sharesToBurn: redeemAll ? (userVault?.shareBalance ?? null) : preview,
    exceedsPosition,
    status,
    submit,
    canSubmit:
      address !== null &&
      userVault !== null &&
      assets !== null &&
      !exceedsPosition &&
      status.kind !== 'signing',
  }
}
