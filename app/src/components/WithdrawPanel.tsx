import { SHARE_DECIMALS, USDC_DECIMALS } from '../config'
import type { useWithdraw } from '../hooks/useWithdraw'
import { formatUnits } from '../lib/units'

type Props = {
  address: string | null
  wrongNetwork: boolean
  idleAssets: bigint | null
  withdraw: ReturnType<typeof useWithdraw>
}

export function WithdrawPanel({ address, wrongNetwork, idleAssets, withdraw }: Props) {
  if (!address) {
    return (
      <section className="panel panel-muted" aria-labelledby="withdraw-title">
        <h2 id="withdraw-title">Redeem</h2>
        <p className="muted">Connect your wallet to redeem USDC.</p>
      </section>
    )
  }

  const {
    input,
    setInput,
    setMax,
    assets,
    maxAssets,
    redeemAll,
    sharesToBurn,
    exceedsPosition,
    status,
    submit,
    canSubmit,
  } = withdraw
  const signing = status.kind === 'signing'

  return (
    <section className="panel" aria-labelledby="withdraw-title">
      <h2 id="withdraw-title">Redeem</h2>
      <form
        className="form"
        onSubmit={(e) => {
          e.preventDefault()
          void submit()
        }}
      >
        <label className="field-label" htmlFor="withdraw-amount">
          Amount in USDC
        </label>
        <div className="field">
          <input
            id="withdraw-amount"
            className="input"
            inputMode="decimal"
            autoComplete="off"
            placeholder="0.00"
            value={input}
            disabled={signing}
            onChange={(e) => setInput(e.target.value)}
          />
          <button
            type="button"
            className="btn btn-ghost"
            onClick={setMax}
            disabled={signing || maxAssets === null || maxAssets === 0n}
          >
            Max
          </button>
        </div>

        {maxAssets !== null && (
          <p className="muted field-hint">
            You can redeem up to {formatUnits(maxAssets, USDC_DECIMALS)} USDC
          </p>
        )}
        {input.trim() !== '' && assets === null && (
          <p className="field-error">Enter a valid amount greater than zero.</p>
        )}
        {exceedsPosition && (
          <p className="field-error">Your position is not enough to redeem that amount.</p>
        )}
        {assets !== null && !exceedsPosition && sharesToBurn !== null && (
          <p className="muted field-hint">
            {redeemAll ? 'Will use all your' : 'Will use ≈'}{' '}
            {formatUnits(sharesToBurn, SHARE_DECIMALS, { maxFractionDigits: 7 })} shares
          </p>
        )}

        <button type="submit" className="btn btn-primary" disabled={!canSubmit || wrongNetwork}>
          {signing ? 'Signing in your wallet…' : redeemAll ? 'Redeem all' : 'Redeem'}
        </button>
      </form>

      {idleAssets !== null && (
        <p className="muted field-hint">
          Up to {formatUnits(idleAssets, USDC_DECIMALS)} USDC is paid from the reserve; if you
          ask for more, the fund pulls the difference from Blend in the same transaction.
        </p>
      )}

      {wrongNetwork && (
        <p className="alert alert-warning">Switch your wallet to testnet to redeem.</p>
      )}
      {status.kind === 'success' && (
        <p className="alert alert-success" role="status">
          Redemption complete. You received {formatUnits(status.usdc, USDC_DECIMALS)} USDC in your wallet.
        </p>
      )}
      {status.kind === 'error' && (
        <p className="alert alert-error" role="alert">
          {status.message}
        </p>
      )}
    </section>
  )
}
