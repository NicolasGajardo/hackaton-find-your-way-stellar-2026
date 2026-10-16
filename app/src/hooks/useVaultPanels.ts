import { useCallback, useEffect, useState } from 'react'
import {
  fetchMaxDeposit,
  fetchMaxWithdraw,
  fetchUserVaultStats,
  fetchVaultFundStats,
  type UserVaultStats,
  type VaultFundStats,
} from '../lib/vault'
import { fetchUsdcBalance } from '../lib/usdcBalance'

type LoadState = 'idle' | 'loading' | 'error'

export function useVaultPanels(address: string | null) {
  const [fund, setFund] = useState<VaultFundStats | null>(null)
  const [userVault, setUserVault] = useState<UserVaultStats | null>(null)
  const [usdcBalance, setUsdcBalance] = useState<bigint | null>(null)
  const [maxDeposit, setMaxDeposit] = useState<bigint | null>(null)
  const [maxWithdraw, setMaxWithdraw] = useState<bigint | null>(null)
  const [fundState, setFundState] = useState<LoadState>('idle')
  const [userState, setUserState] = useState<LoadState>('idle')
  const [error, setError] = useState<string | null>(null)

  const refreshFund = useCallback(async () => {
    setFundState('loading')
    setError(null)
    try {
      const stats = await fetchVaultFundStats()
      setFund(stats)
      setFundState('idle')
    } catch (e) {
      setFundState('error')
      setError(e instanceof Error ? e.message : 'Failed to load the fund')
    }
  }, [])

  const refreshUser = useCallback(async () => {
    if (!address) {
      setUserVault(null)
      setUsdcBalance(null)
      setMaxDeposit(null)
      setMaxWithdraw(null)
      setUserState('idle')
      return
    }
    setUserState('loading')
    setError(null)
    try {
      const [vaultStats, walletUsdc, depositCap, withdrawCap] = await Promise.all([
        fetchUserVaultStats(address),
        fetchUsdcBalance(address),
        fetchMaxDeposit(address),
        fetchMaxWithdraw(address),
      ])
      setUserVault(vaultStats)
      setUsdcBalance(walletUsdc)
      setMaxDeposit(depositCap)
      setMaxWithdraw(withdrawCap)
      setUserState('idle')
    } catch (e) {
      setUserState('error')
      setError(e instanceof Error ? e.message : 'Failed to load your investment')
    }
  }, [address])

  const refreshAll = useCallback(async () => {
    await Promise.all([refreshFund(), refreshUser()])
  }, [refreshFund, refreshUser])

  useEffect(() => {
    void refreshFund()
  }, [refreshFund])

  useEffect(() => {
    void refreshUser()
  }, [refreshUser])

  return {
    fund,
    userVault,
    usdcBalance,
    maxDeposit,
    maxWithdraw,
    fundState,
    userState,
    error,
    refreshAll,
  }
}
