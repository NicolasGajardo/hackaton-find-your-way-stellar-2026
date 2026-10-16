import { SHARE_DECIMALS, USDC_DECIMALS } from '../config'
import { formatUnits } from '../lib/units'
import type { UserVaultStats } from '../lib/vault'
import { StatRow } from './StatRow'

type Props = {
  address: string | null
  userVault: UserVaultStats | null
  usdcBalance: bigint | null
  loading: boolean
}

export function InvestmentPanel({
  address,
  userVault,
  usdcBalance,
  loading,
}: Props) {
  if (!address) {
    return (
      <section className="panel panel-muted" aria-labelledby="invest-title">
        <h2 id="invest-title">My investment</h2>
        <p className="muted">Connect your wallet to see your shares and USDC balance.</p>
      </section>
    )
  }

  return (
    <section className="panel" aria-labelledby="invest-title">
      <h2 id="invest-title">My investment</h2>
      {loading && !userVault && <p className="muted">Loading your position…</p>}
      {userVault && usdcBalance !== null && (
        <dl className="stat-list">
          <StatRow
            label="My shares"
            value={formatUnits(userVault.shareBalance, SHARE_DECIMALS)}
          />
          <StatRow
            label="Value in USDC"
            value={`${formatUnits(userVault.sharesValueUsdc, USDC_DECIMALS)} USDC`}
          />
          <StatRow
            label="USDC in wallet"
            value={`${formatUnits(usdcBalance, USDC_DECIMALS)} USDC`}
          />
        </dl>
      )}
    </section>
  )
}
