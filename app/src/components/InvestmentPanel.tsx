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
        <h2 id="invest-title">Mi inversión</h2>
        <p className="muted">Conecta tu wallet para ver tus cuotas y saldo de USDC.</p>
      </section>
    )
  }

  return (
    <section className="panel" aria-labelledby="invest-title">
      <h2 id="invest-title">Mi inversión</h2>
      {loading && !userVault && <p className="muted">Cargando tu posición…</p>}
      {userVault && usdcBalance !== null && (
        <dl className="stat-list">
          <StatRow
            label="Mis cuotas"
            value={formatUnits(userVault.shareBalance, SHARE_DECIMALS)}
          />
          <StatRow
            label="Valor en USDC"
            value={`${formatUnits(userVault.sharesValueUsdc, USDC_DECIMALS)} USDC`}
          />
          <StatRow
            label="USDC en la wallet"
            value={`${formatUnits(usdcBalance, USDC_DECIMALS)} USDC`}
          />
        </dl>
      )}
    </section>
  )
}
