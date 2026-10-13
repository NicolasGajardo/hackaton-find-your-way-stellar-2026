import { StellarWalletsKit } from '@creit.tech/stellar-wallets-kit/sdk'
import { defaultModules } from '@creit.tech/stellar-wallets-kit/modules/utils'
import { Networks } from '@creit.tech/stellar-wallets-kit/types'
import { NETWORK_PASSPHRASE } from '../config'

let initialized = false

export function initWalletKit(): void {
  if (initialized) {
    return
  }
  StellarWalletsKit.init({
    modules: defaultModules(),
    network: Networks.TESTNET,
  })
  StellarWalletsKit.setNetwork(Networks.TESTNET)
  initialized = true
}

export { StellarWalletsKit, Networks }

export function isTestnetPassphrase(passphrase: string): boolean {
  return passphrase === NETWORK_PASSPHRASE
}
