import { Asset, Horizon } from '@stellar/stellar-sdk'
import {
  HORIZON_URL,
  USDC_ASSET_CODE,
  USDC_ISSUER,
} from '../config'

const usdcAsset = new Asset(USDC_ASSET_CODE, USDC_ISSUER)

export async function fetchUsdcBalance(accountId: string): Promise<bigint> {
  const server = new Horizon.Server(HORIZON_URL)
  try {
    const stellarAccount = await server.loadAccount(accountId)
    const line = stellarAccount.balances.find((balance) => {
      if (
        balance.asset_type !== 'credit_alphanum4' &&
        balance.asset_type !== 'credit_alphanum12'
      ) {
        return false
      }
      return (
        balance.asset_code === usdcAsset.getCode() &&
        balance.asset_issuer === usdcAsset.getIssuer()
      )
    })
    if (!line) {
      return 0n
    }
    const amount = line.balance
    const [whole, fraction = ''] = amount.split('.')
    const fracPadded = fraction.padEnd(7, '0').slice(0, 7)
    return BigInt(whole + fracPadded)
  } catch (err) {
    if (
      err instanceof Error &&
      err.message.includes('Resource missing')
    ) {
      return 0n
    }
    throw err
  }
}
