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

const PAUSED_NOTICE = 'Deposits are paused. Redemptions are still available.'

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
      <h2 id="fund-title">The fund</h2>
      {loading && !fund && <p className="muted">Loading fund data…</p>}
      {error && (
        <div className="alert alert-error">
          <p>{error}</p>
          <button type="button" className="btn btn-ghost" onClick={onRetry}>
            Retry
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
              label="Total fund value"
              value={`${formatUnits(fund.totalAssets, USDC_DECIMALS)} USDC`}
            />
            <StatRow
              label="Shares outstanding"
              value={formatUnits(fund.totalSupply, SHARE_DECIMALS)}
            />
            <StatRow
              label="Share price"
              value={`${formatUnits(fund.oneShareValueUsdc, USDC_DECIMALS)} USDC`}
            />
            <StatRow
              label="Return since inception"
              value={formatShareReturn(fund.oneShareValueUsdc)}
              hint="The share started at 1 USDC."
            />
            <StatRow
              label="Projected return (30 days)"
              value={formatBps(projectedReturn30dBps())}
              hint={`*Estimate based on a reference annual rate of ${formatBps(REFERENCE_APY_BPS)} on the 90% invested. Not guaranteed.`}
            />
          </dl>

          <h3 className="section-title">Composition</h3>
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
                <th>Name</th>
                <th>Amount</th>
                <th>Current</th>
                <th>Target</th>
              </tr>
            </thead>
            <tbody>
              {composition.rows.map((row) => (
                <tr key={row.key}>
                  <td>
                    {row.kind === 'reserve'
                      ? 'Reserve (available instantly)'
                      : strategyName(row.address ?? '')}
                    {row.paused && <span className="tag">paused</span>}
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
              Moves money between the reserve and the strategies to return to the target
              percentages. Anyone can do it; you only pay the network fee.
            </p>
            {!paused && (
              <p className={composition.offTarget ? 'field-error' : 'muted field-hint'}>
                {composition.offTarget
                  ? 'The fund has drifted from its target'
                  : 'The fund is on target'}
              </p>
            )}
            <button
              type="button"
              className={composition.offTarget && !paused ? 'btn btn-primary' : 'btn btn-ghost'}
              disabled={!rebalance.canSubmit}
              onClick={() => void rebalance.submit()}
            >
              {signing ? 'Signing…' : 'Rebalance'}
            </button>
            {!address && (
              <p className="muted field-hint">Connect your testnet wallet to rebalance.</p>
            )}
            {wrongNetwork && (
              <p className="alert alert-warning">Switch your wallet to testnet to rebalance.</p>
            )}
            {rebalance.status.kind === 'success' && (
              <p className="alert alert-success" role="status">
                Done. The fund has been rebalanced.
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
