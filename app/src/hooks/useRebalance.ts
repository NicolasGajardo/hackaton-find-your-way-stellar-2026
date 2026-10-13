import { useCallback, useState } from 'react'
import { describeTxError } from '../lib/txErrors'
import { rebalanceVault } from '../lib/vault'

export type RebalanceStatus =
  | { kind: 'idle' }
  | { kind: 'signing' }
  | { kind: 'success' }
  | { kind: 'error'; message: string }

export function useRebalance(
  address: string | null,
  paused: boolean,
  wrongNetwork: boolean,
  onRebalanced: () => Promise<void>,
) {
  const [status, setStatus] = useState<RebalanceStatus>({ kind: 'idle' })

  const submit = useCallback(async () => {
    if (!address || paused || wrongNetwork) {
      return
    }
    setStatus({ kind: 'signing' })
    try {
      await rebalanceVault(address)
      setStatus({ kind: 'success' })
      await onRebalanced()
    } catch (e) {
      setStatus({ kind: 'error', message: describeTxError(e) })
    }
  }, [address, paused, wrongNetwork, onRebalanced])

  return {
    status,
    submit,
    canSubmit: address !== null && !paused && !wrongNetwork && status.kind !== 'signing',
  }
}
