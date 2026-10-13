import { STRATEGY_NAMES, SHARE_DECIMALS, USDC_DECIMALS } from '../config'
import type { useRebalance } from '../hooks/useRebalance'
import {
  REFERENCE_APY_BPS,
  buildComposition,
  formatBps,
  formatShareReturn,
  projectedReturn30dBps,
} from '../lib/composition'
import { abbreviateAddress, formatUnits } from '../lib/units'
import type { VaultFundStats } from '../lib/vault'
import { StatRow } from './StatRow'

type Props = {
  fund: VaultFundStats | null
  loading: boolean
  error: string | null
  address: string | null
  wrongNetwork: boolean
  rebalance: ReturnType<typeof useRebalance>
  onRetry: () => void
}

const PAUSED_NOTICE = 'Los aportes están pausados. Los rescates siguen disponibles.'

function strategyName(address: string): string {
  return STRATEGY_NAMES[address] ?? abbreviateAddress(address)
}

function barWidth(bps: bigint): string {
  const whole = bps / 100n
  const frac = (bps % 100n).toString().padStart(2, '0')
  return `${whole.toString()}.${frac}%`
}

export function FundPanel({
  fund,
  loading,
  error,
  address,
  wrongNetwork,
  rebalance,
  onRetry,
}: Props) {
  const composition = fund
    ? buildComposition(fund.totalAssets, fund.idleAssets, fund.strategies)
    : null
  const signing = rebalance.status.kind === 'signing'
  const paused = fund?.paused === true

  return (
    <section className="panel panel-wide" aria-labelledby="fund-title">
      <h2 id="fund-title">El fondo</h2>
      {loading && !fund && <p className="muted">Cargando datos del fondo…</p>}
      {error && (
        <div className="alert alert-error">
          <p>{error}</p>
          <button type="button" className="btn btn-ghost" onClick={onRetry}>
            Reintentar
          </button>
        </div>
      )}
      {fund && composition && (
        <>
          {paused && (
            <p className="alert alert-warning" role="status">
              {PAUSED_NOTICE}
            </p>
          )}
          <dl className="stat-list">
            <StatRow
              label="Total del fondo"
              value={`${formatUnits(fund.totalAssets, USDC_DECIMALS)} USDC`}
            />
            <StatRow
              label="Cuotas emitidas"
              value={formatUnits(fund.totalSupply, SHARE_DECIMALS)}
            />
            <StatRow
              label="Valor cuota"
              value={`${formatUnits(fund.oneShareValueUsdc, USDC_DECIMALS)} USDC`}
            />
            <StatRow
              label="Rentabilidad desde el inicio"
              value={formatShareReturn(fund.oneShareValueUsdc)}
              hint="La cuota partió valiendo 1 USDC."
            />
            <StatRow
              label="Rentabilidad proyectada (30 días)"
              value={formatBps(projectedReturn30dBps())}
              hint={`*Estimación con una tasa anual de referencia de ${formatBps(REFERENCE_APY_BPS)} sobre el 90% invertido. No es garantía.`}
            />
          </dl>

          <h3 className="section-title">Composición</h3>
          <div className="composition-bar" aria-hidden="true">
            {composition.rows.map((row, index) =>
              row.currentBps > 0n ? (
                <span
                  key={row.key}
                  className={`composition-segment composition-segment-${row.kind} tone-${index % 4}`}
                  style={{ width: barWidth(row.currentBps) }}
                />
              ) : null,
            )}
          </div>
          <table className="composition">
            <thead>
              <tr>
                <th>Nombre</th>
                <th>Monto</th>
                <th>Actual</th>
                <th>Objetivo</th>
              </tr>
            </thead>
            <tbody>
              {composition.rows.map((row) => (
                <tr key={row.key}>
                  <td>
                    {row.kind === 'reserve'
                      ? 'Reserva (disponible al instante)'
                      : strategyName(row.address ?? '')}
                    {row.paused && <span className="tag">pausada</span>}
                  </td>
                  <td>{formatUnits(row.balance, USDC_DECIMALS)} USDC</td>
                  <td>{formatBps(row.currentBps)}</td>
                  <td>{formatBps(row.targetBps)}</td>
                </tr>
              ))}
            </tbody>
          </table>

          <div className="rebalance">
            <p className="muted">
              Mueve dinero entre la reserva y las estrategias para volver a los porcentajes
              objetivo. Cualquiera puede hacerlo; solo pagas la comisión de red.
            </p>
            {!paused && (
              <p className={composition.offTarget ? 'field-error' : 'muted field-hint'}>
                {composition.offTarget
                  ? 'El fondo se desvió de su objetivo'
                  : 'El fondo está en su objetivo'}
              </p>
            )}
            <button
              type="button"
              className={composition.offTarget && !paused ? 'btn btn-primary' : 'btn btn-ghost'}
              disabled={!rebalance.canSubmit}
              onClick={() => void rebalance.submit()}
            >
              {signing ? 'Firmando…' : 'Rebalancear'}
            </button>
            {!address && (
              <p className="muted field-hint">Conecta tu wallet en testnet para rebalancear.</p>
            )}
            {wrongNetwork && (
              <p className="alert alert-warning">Cambia tu wallet a testnet para rebalancear.</p>
            )}
            {rebalance.status.kind === 'success' && (
              <p className="alert alert-success" role="status">
                Listo. El fondo se rebalanceó.
              </p>
            )}
            {rebalance.status.kind === 'error' && (
              <p className="alert alert-error" role="alert">
                {rebalance.status.message}
              </p>
            )}
          </div>
        </>
      )}
    </section>
  )
}
