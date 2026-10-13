import { KitEventType } from '@creit.tech/stellar-wallets-kit/types'
import { useCallback, useEffect, useState } from 'react'
import {
  initWalletKit,
  isTestnetPassphrase,
  StellarWalletsKit,
} from '../wallet/kit'

export type WalletState = {
  address: string | null
  connecting: boolean
  wrongNetwork: boolean
  networkLabel: string | null
}

export function useWallet() {
  const [address, setAddress] = useState<string | null>(null)
  const [connecting, setConnecting] = useState(false)
  const [wrongNetwork, setWrongNetwork] = useState(false)
  const [networkLabel, setNetworkLabel] = useState<string | null>(null)

  const refreshNetwork = useCallback(async () => {
    if (!address) {
      setWrongNetwork(false)
      setNetworkLabel(null)
      return
    }
    try {
      const { network, networkPassphrase } = await StellarWalletsKit.getNetwork()
      setNetworkLabel(network)
      setWrongNetwork(!isTestnetPassphrase(networkPassphrase))
    } catch {
      setWrongNetwork(true)
      setNetworkLabel(null)
    }
  }, [address])

  useEffect(() => {
    initWalletKit()

    const unsubState = StellarWalletsKit.on(
      KitEventType.STATE_UPDATED,
      (event) => {
        const next = event.payload.address ?? null
        setAddress(next)
      },
    )

    const unsubDisconnect = StellarWalletsKit.on(
      KitEventType.DISCONNECT,
      () => {
        setAddress(null)
        setWrongNetwork(false)
        setNetworkLabel(null)
      },
    )

    void StellarWalletsKit.getAddress()
      .then(({ address: addr }) => setAddress(addr))
      .catch(() => setAddress(null))

    return () => {
      unsubState()
      unsubDisconnect()
    }
  }, [])

  useEffect(() => {
    void refreshNetwork()
  }, [address, refreshNetwork])

  const connect = useCallback(async () => {
    setConnecting(true)
    try {
      const { address: addr } = await StellarWalletsKit.authModal()
      setAddress(addr)
      await refreshNetwork()
    } finally {
      setConnecting(false)
    }
  }, [refreshNetwork])

  const disconnect = useCallback(async () => {
    await StellarWalletsKit.disconnect()
    setAddress(null)
    setWrongNetwork(false)
    setNetworkLabel(null)
  }, [])

  return {
    address,
    connecting,
    wrongNetwork,
    networkLabel,
    connect,
    disconnect,
    isConnected: address !== null,
  }
}
